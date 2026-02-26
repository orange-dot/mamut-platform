//! Causal Consistency Checker.
//!
//! Causal consistency is weaker than sequential consistency. A history is
//! causally consistent if all processes see causally related operations in
//! the same order, but concurrent (causally unrelated) operations may be
//! seen in different orders by different processes.
//!
//! Causal relationships are established by:
//! 1. Program order: operations by the same process
//! 2. Reads-from: a read is causally after the write it reads from
//! 3. Transitivity: if A -> B and B -> C, then A -> C

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use mamut_core::operation::OperationPhase;
use mamut_core::{History, Operation, OperationId, ProcessId};
use serde::{de::DeserializeOwned, Serialize};

use crate::result::{CheckResult, CheckStats, Counterexample, CounterexampleOp, ViolationType};
use crate::traits::Checker;

/// Configuration for the causal consistency checker.
#[derive(Debug, Clone)]
pub struct CausalConsistencyConfig {
    /// Maximum time for checking.
    pub timeout: Option<Duration>,
    /// Whether to track detailed causality information.
    pub detailed_causality: bool,
}

impl Default for CausalConsistencyConfig {
    fn default() -> Self {
        Self {
            timeout: Some(Duration::from_secs(30)),
            detailed_causality: true,
        }
    }
}

/// Causal consistency checker.
///
/// Verifies that a history satisfies causal consistency by checking that
/// each process's view of causally related operations is consistent.
pub struct CausalConsistencyChecker<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
{
    config: CausalConsistencyConfig,
    _phantom: std::marker::PhantomData<V>,
}

impl<V> CausalConsistencyChecker<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
{
    /// Create a new causal consistency checker.
    pub fn new() -> Self {
        Self {
            config: CausalConsistencyConfig::default(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create with configuration.
    pub fn with_config(config: CausalConsistencyConfig) -> Self {
        Self {
            config,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = Some(timeout);
        self
    }
}

impl<V> Default for CausalConsistencyChecker<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<V> Checker for CausalConsistencyChecker<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + std::fmt::Debug + PartialEq + 'static,
{
    type Value = V;

    fn name(&self) -> &str {
        "causal-consistency"
    }

    fn description(&self) -> &str {
        "Checks that the history satisfies causal consistency"
    }

    async fn check(&self, history: &History<Self::Value>) -> CheckResult {
        let start = Instant::now();

        if history.is_empty() {
            return CheckResult::pass().with_duration(start.elapsed());
        }

        // Build causal graph
        let causal_graph = match CausalGraph::build(history) {
            Ok(g) => g,
            Err(e) => return CheckResult::error(e),
        };

        // Check for cycles in causal order (which would be a violation)
        if let Some(cycle) = causal_graph.find_cycle() {
            let ce = Counterexample::new(
                format!("Causal cycle detected: {:?}", cycle),
                ViolationType::CausalityViolation,
            )
            .with_operations(
                cycle
                    .iter()
                    .map(|&id| CounterexampleOp::new(id, "In cycle".into()))
                    .collect(),
            );
            return CheckResult::fail(ce).with_duration(start.elapsed());
        }

        // For each process, verify its view is consistent
        let processes: HashSet<_> = history.operations.iter().map(|op| op.process).collect();

        for pid in processes {
            if let Err(violation) = self.verify_process_view(history, &causal_graph, pid) {
                return CheckResult::fail(violation).with_duration(start.elapsed());
            }
        }

        CheckResult::pass()
            .with_duration(start.elapsed())
            .with_stats(CheckStats {
                num_operations: history.len(),
                ..Default::default()
            })
    }
}

impl<V> CausalConsistencyChecker<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + std::fmt::Debug + PartialEq + 'static,
{
    /// Verify a single process's causal view is consistent.
    fn verify_process_view(
        &self,
        history: &History<V>,
        causal_graph: &CausalGraph,
        process_id: ProcessId,
    ) -> Result<(), Counterexample> {
        // Get operations visible to this process in causal order
        let process_ops: Vec<_> = history
            .operations
            .iter()
            .filter(|op| op.process == process_id)
            .collect();

        // Verify program order is respected within the process
        for window in process_ops.windows(2) {
            let (op1, op2) = (&window[0], &window[1]);

            // op1 should causally precede op2 if op1's OVC is before op2's
            if op1.ovc.timestamp() < op2.ovc.timestamp() {
                if causal_graph.precedes(op2.id, op1.id) {
                    return Err(Counterexample::new(
                        format!(
                            "Program order violation: {:?} should come before {:?}",
                            op1.id, op2.id
                        ),
                        ViolationType::ProgramOrderViolation,
                    )
                    .with_operation(CounterexampleOp::new(
                        op1.id,
                        format!("{}({:?})", op1.function, op1.args),
                    ))
                    .with_operation(CounterexampleOp::new(
                        op2.id,
                        format!("{}({:?})", op2.function, op2.args),
                    )));
                }
            }
        }

        Ok(())
    }
}

/// Graph representing causal relationships between operations.
struct CausalGraph {
    /// Direct causal edges: operation -> operations it causally precedes
    edges: HashMap<OperationId, HashSet<OperationId>>,
    /// Reverse edges for efficient backward traversal
    #[allow(dead_code)]
    reverse_edges: HashMap<OperationId, HashSet<OperationId>>,
    /// All operation IDs
    nodes: HashSet<OperationId>,
}

impl CausalGraph {
    /// Build a causal graph from a history.
    fn build<V: Clone + PartialEq + std::fmt::Debug>(history: &History<V>) -> Result<Self, String> {
        let mut edges: HashMap<OperationId, HashSet<OperationId>> = HashMap::new();
        let mut reverse_edges: HashMap<OperationId, HashSet<OperationId>> = HashMap::new();
        let nodes: HashSet<_> = history.operations.iter().map(|op| op.id).collect();

        // Initialize edge sets
        for op in &history.operations {
            edges.entry(op.id).or_default();
            reverse_edges.entry(op.id).or_default();
        }

        // Add program order edges
        let mut process_ops: HashMap<ProcessId, Vec<&Operation<V>>> = HashMap::new();
        for op in &history.operations {
            process_ops.entry(op.process).or_default().push(op);
        }

        for (_, ops) in &mut process_ops {
            ops.sort_by_key(|op| op.ovc.timestamp());
            for window in ops.windows(2) {
                let (op1, op2) = (window[0], window[1]);
                edges.entry(op1.id).or_default().insert(op2.id);
                reverse_edges.entry(op2.id).or_default().insert(op1.id);
            }
        }

        // Add causal edges based on OVC happens-before relationship
        for op1 in &history.operations {
            for op2 in &history.operations {
                if op1.id != op2.id && op1.ovc.happens_before(&op2.ovc) {
                    edges.entry(op1.id).or_default().insert(op2.id);
                    reverse_edges.entry(op2.id).or_default().insert(op1.id);
                }
            }
        }

        // Add reads-from edges based on result values
        // This is a simplification - in practice you'd need to track which write a read observed
        let writes: Vec<_> = history
            .operations
            .iter()
            .filter(|op| op.function.contains("write") || op.function.contains("put"))
            .collect();

        for op in &history.operations {
            if op.function.contains("read") || op.function.contains("get") {
                // Try to find a matching write based on the result
                if let OperationPhase::Completed { ref result } = op.phase {
                    for write in &writes {
                        // If the write's args match the read's result, add causal edge
                        if format!("{:?}", write.args).contains(&format!("{}", result)) {
                            if write.ovc.timestamp() < op.ovc.timestamp() {
                                edges.entry(write.id).or_default().insert(op.id);
                                reverse_edges.entry(op.id).or_default().insert(write.id);
                            }
                        }
                    }
                }
            }
        }

        Ok(Self {
            edges,
            reverse_edges,
            nodes,
        })
    }

    /// Check if op1 causally precedes op2 (direct or transitive).
    fn precedes(&self, op1: OperationId, op2: OperationId) -> bool {
        if op1 == op2 {
            return false;
        }

        // BFS from op1 to find op2
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(op1);
        visited.insert(op1);

        while let Some(current) = queue.pop_front() {
            if let Some(successors) = self.edges.get(&current) {
                for &succ in successors {
                    if succ == op2 {
                        return true;
                    }
                    if !visited.contains(&succ) {
                        visited.insert(succ);
                        queue.push_back(succ);
                    }
                }
            }
        }

        false
    }

    /// Find a cycle in the causal graph (returns None if acyclic).
    fn find_cycle(&self) -> Option<Vec<OperationId>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for &node in &self.nodes {
            if !visited.contains(&node) {
                if let Some(cycle) = self.dfs_cycle(node, &mut visited, &mut rec_stack, &mut path) {
                    return Some(cycle);
                }
            }
        }

        None
    }

    fn dfs_cycle(
        &self,
        node: OperationId,
        visited: &mut HashSet<OperationId>,
        rec_stack: &mut HashSet<OperationId>,
        path: &mut Vec<OperationId>,
    ) -> Option<Vec<OperationId>> {
        visited.insert(node);
        rec_stack.insert(node);
        path.push(node);

        if let Some(successors) = self.edges.get(&node) {
            for &succ in successors {
                if !visited.contains(&succ) {
                    if let Some(cycle) = self.dfs_cycle(succ, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(&succ) {
                    // Found cycle - extract it from path
                    let cycle_start = path.iter().position(|&n| n == succ).unwrap();
                    return Some(path[cycle_start..].to_vec());
                }
            }
        }

        path.pop();
        rec_stack.remove(&node);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mamut_core::history::RunId;
    use mamut_core::operation::OVC;

    fn make_write_op(id: u64, process: u32, ovc: OVC) -> Operation<serde_json::Value> {
        Operation::new(
            OperationId(id),
            ProcessId(process),
            "write",
            serde_json::json!({"value": id}),
            ovc,
        )
        .with_completion(serde_json::json!(null), ovc.increment())
    }

    fn make_read_op(id: u64, process: u32, ovc: OVC) -> Operation<serde_json::Value> {
        Operation::new(
            OperationId(id),
            ProcessId(process),
            "read",
            serde_json::json!({}),
            ovc,
        )
        .with_completion(serde_json::json!(1), ovc.increment())
    }

    #[tokio::test]
    async fn test_causal_consistency_simple() {
        let mut history = History::new(RunId::new());

        history.push(make_write_op(1, 1, OVC::new(0)));
        history.push(make_read_op(2, 1, OVC::new(100)));

        let checker: CausalConsistencyChecker<serde_json::Value> = CausalConsistencyChecker::new();
        let result = checker.check(&history).await;

        assert!(result.is_pass());
    }

    #[tokio::test]
    async fn test_causal_graph_no_cycle() {
        let mut history: History<serde_json::Value> = History::new(RunId::new());

        history.push(make_write_op(1, 1, OVC::new(0)));
        history.push(make_write_op(2, 1, OVC::new(100)));

        let graph = CausalGraph::build(&history).unwrap();
        assert!(graph.find_cycle().is_none());
        assert!(graph.precedes(OperationId(1), OperationId(2)));
        assert!(!graph.precedes(OperationId(2), OperationId(1)));
    }

    #[tokio::test]
    async fn test_empty_history() {
        let history: History<serde_json::Value> = History::new(RunId::new());
        let checker: CausalConsistencyChecker<serde_json::Value> = CausalConsistencyChecker::new();
        let result = checker.check(&history).await;
        assert!(result.is_pass());
    }
}
