//! Sequential Consistency Checker.
//!
//! Sequential consistency is weaker than linearizability. A history is
//! sequentially consistent if there exists a total order of all operations
//! that:
//! 1. Is consistent with the sequential specification (model)
//! 2. Preserves program order (operations from the same process appear in order)
//!
//! Unlike linearizability, sequential consistency does NOT require respecting
//! real-time order (operation A returning before B starts doesn't mean A must
//! come before B in the order).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use mamut_core::{History, Operation, OperationId, ProcessId};
use serde::{de::DeserializeOwned, Serialize};

use crate::linearizability::OperationExtractor;
use crate::result::{CheckResult, CheckStats, Counterexample, CounterexampleOp, ViolationType};
use crate::traits::{Checker, Model};
use crate::ExpectedResult;

/// Configuration for the sequential consistency checker.
#[derive(Debug, Clone)]
pub struct SequentialConsistencyConfig {
    /// Maximum time to spend checking.
    pub timeout: Option<Duration>,
    /// Maximum number of states to explore.
    pub max_states: Option<u64>,
}

impl Default for SequentialConsistencyConfig {
    fn default() -> Self {
        Self {
            timeout: Some(Duration::from_secs(60)),
            max_states: Some(100_000_000),
        }
    }
}

/// A sequential consistency checker.
///
/// This checker verifies that a history satisfies sequential consistency,
/// which is weaker than linearizability.
pub struct SequentialConsistencyChecker<M, V, E>
where
    M: Model,
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
    E: OperationExtractor<V, ModelOp = M::Op>,
{
    /// The sequential specification model.
    model: M,
    /// Configuration.
    config: SequentialConsistencyConfig,
    /// Extractor for operations.
    extractor: E,
    /// Phantom data.
    _phantom: std::marker::PhantomData<V>,
}

impl<M, V, E> SequentialConsistencyChecker<M, V, E>
where
    M: Model,
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
    E: OperationExtractor<V, ModelOp = M::Op>,
{
    /// Create a new sequential consistency checker.
    pub fn new(model: M, extractor: E) -> Self {
        Self {
            model,
            config: SequentialConsistencyConfig::default(),
            extractor,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(model: M, extractor: E, config: SequentialConsistencyConfig) -> Self {
        Self {
            model,
            config,
            extractor,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = Some(timeout);
        self
    }
}

#[async_trait]
impl<M, V, E> Checker for SequentialConsistencyChecker<M, V, E>
where
    M: Model + 'static,
    M::State: std::fmt::Debug,
    V: Serialize + DeserializeOwned + Send + Sync + Clone + std::fmt::Debug + 'static,
    E: OperationExtractor<V, ModelOp = M::Op> + 'static,
{
    type Value = V;

    fn name(&self) -> &str {
        "sequential-consistency"
    }

    fn description(&self) -> &str {
        "Checks that the history satisfies sequential consistency (preserves program order)"
    }

    async fn check(&self, history: &History<Self::Value>) -> CheckResult {
        let start = Instant::now();

        if history.is_empty() {
            return CheckResult::pass().with_duration(start.elapsed());
        }

        // Build program order constraints
        let context = match SeqSearchContext::build(history, &self.model, &self.extractor) {
            Ok(ctx) => ctx,
            Err(e) => return CheckResult::error(e),
        };

        // Search for valid sequential ordering
        let stats = Arc::new(SearchStats::new());
        let cancelled = Arc::new(AtomicBool::new(false));
        let deadline = self.config.timeout.map(|t| Instant::now() + t);

        let mut searcher =
            SeqSearcher::new(&context, deadline, self.config.max_states, cancelled, stats.clone());

        let result = searcher.search();
        let duration = start.elapsed();

        let check_stats = CheckStats {
            num_operations: history.len(),
            states_explored: stats.states_explored.load(Ordering::Relaxed),
            linearizations_tried: stats.orderings_tried.load(Ordering::Relaxed),
            backtracks: stats.backtracks.load(Ordering::Relaxed),
            pruned: stats.pruned.load(Ordering::Relaxed),
            max_depth: stats.max_depth.load(Ordering::Relaxed) as usize,
            peak_memory_bytes: None,
        };

        match result {
            SeqSearchResult::Found(ordering) => CheckResult::pass()
                .with_duration(duration)
                .with_stats(check_stats)
                .with_diagnostic(format!("Found sequential ordering: {:?}", ordering)),

            SeqSearchResult::NotFound(partial) => {
                let ce = self.build_counterexample(history, partial);
                CheckResult::fail(ce)
                    .with_duration(duration)
                    .with_stats(check_stats)
            }

            SeqSearchResult::Timeout => CheckResult::unknown("Search timed out".into())
                .with_duration(duration)
                .with_stats(check_stats),

            SeqSearchResult::MaxStatesReached => {
                CheckResult::unknown("Maximum states reached".into())
                    .with_duration(duration)
                    .with_stats(check_stats)
            }
        }
    }
}

impl<M, V, E> SequentialConsistencyChecker<M, V, E>
where
    M: Model,
    M::State: std::fmt::Debug,
    V: Serialize + DeserializeOwned + Send + Sync + Clone + std::fmt::Debug + 'static,
    E: OperationExtractor<V, ModelOp = M::Op>,
{
    fn build_counterexample(
        &self,
        history: &History<V>,
        partial: Vec<OperationId>,
    ) -> Counterexample {
        let mut ce = Counterexample::new(
            "No valid sequential ordering found".into(),
            ViolationType::ProgramOrderViolation,
        );

        let linearized: std::collections::HashSet<_> = partial.iter().copied().collect();
        let remaining: Vec<_> = history
            .operations
            .iter()
            .filter(|op| !linearized.contains(&op.id))
            .collect();

        if !partial.is_empty() {
            ce.description = format!(
                "Ordered {} of {} operations before getting stuck",
                partial.len(),
                history.len()
            );
        }

        for op in remaining {
            let ce_op = CounterexampleOp::new(op.id, format!("{}({:?})", op.function, op.args))
                .with_interval(op.ovc.timestamp(), op.ovc.timestamp());
            ce = ce.with_operation(ce_op);
        }

        ce
    }
}

/// Search context for sequential consistency.
struct SeqSearchContext<M, V, E>
where
    M: Model,
    V: Clone,
    E: OperationExtractor<V, ModelOp = M::Op>,
{
    model: M,
    extractor: E,
    operations: HashMap<OperationId, Operation<V>>,
    model_ops: HashMap<OperationId, M::Op>,
    /// Program order: for each process, the ordered list of operations
    process_ops: HashMap<ProcessId, Vec<OperationId>>,
    /// All operation IDs
    all_ops: Vec<OperationId>,
}

impl<M, V, E> SeqSearchContext<M, V, E>
where
    M: Model,
    V: Clone,
    E: OperationExtractor<V, ModelOp = M::Op>,
{
    fn build(history: &History<V>, model: &M, extractor: &E) -> Result<Self, String> {
        let mut operations = HashMap::new();
        let mut model_ops = HashMap::new();
        let mut process_ops: HashMap<ProcessId, Vec<(u64, OperationId)>> = HashMap::new();
        let mut all_ops = Vec::new();

        for op in &history.operations {
            operations.insert(op.id, op.clone());
            model_ops.insert(op.id, extractor.extract(op));
            all_ops.push(op.id);

            process_ops
                .entry(op.process)
                .or_default()
                .push((op.ovc.timestamp(), op.id));
        }

        // Sort each process's operations by timestamp (program order)
        let process_ops: HashMap<_, _> = process_ops
            .into_iter()
            .map(|(pid, mut ops)| {
                ops.sort_by_key(|(time, _)| *time);
                (pid, ops.into_iter().map(|(_, id)| id).collect())
            })
            .collect();

        Ok(Self {
            model: model.clone(),
            extractor: extractor.clone(),
            operations,
            model_ops,
            process_ops,
            all_ops,
        })
    }

    /// Get next candidates respecting program order only.
    fn get_next_candidates(&self, process_indices: &HashMap<ProcessId, usize>) -> Vec<OperationId> {
        let mut candidates = Vec::new();

        for (pid, ops) in &self.process_ops {
            let idx = process_indices.get(pid).copied().unwrap_or(0);
            if idx < ops.len() {
                candidates.push(ops[idx]);
            }
        }

        candidates
    }
}

/// Search statistics.
struct SearchStats {
    states_explored: AtomicU64,
    orderings_tried: AtomicU64,
    backtracks: AtomicU64,
    pruned: AtomicU64,
    max_depth: AtomicU64,
}

impl SearchStats {
    fn new() -> Self {
        Self {
            states_explored: AtomicU64::new(0),
            orderings_tried: AtomicU64::new(0),
            backtracks: AtomicU64::new(0),
            pruned: AtomicU64::new(0),
            max_depth: AtomicU64::new(0),
        }
    }
}

enum SeqSearchResult {
    Found(Vec<OperationId>),
    NotFound(Vec<OperationId>),
    Timeout,
    MaxStatesReached,
}

struct SeqSearcher<'a, M, V, E>
where
    M: Model,
    V: Clone,
    E: OperationExtractor<V, ModelOp = M::Op>,
{
    context: &'a SeqSearchContext<M, V, E>,
    deadline: Option<Instant>,
    max_states: Option<u64>,
    cancelled: Arc<AtomicBool>,
    stats: Arc<SearchStats>,
}

impl<'a, M, V, E> SeqSearcher<'a, M, V, E>
where
    M: Model,
    V: Clone + std::fmt::Debug,
    E: OperationExtractor<V, ModelOp = M::Op>,
{
    fn new(
        context: &'a SeqSearchContext<M, V, E>,
        deadline: Option<Instant>,
        max_states: Option<u64>,
        cancelled: Arc<AtomicBool>,
        stats: Arc<SearchStats>,
    ) -> Self {
        Self {
            context,
            deadline,
            max_states,
            cancelled,
            stats,
        }
    }

    fn search(&mut self) -> SeqSearchResult {
        let initial_state = self.context.model.init();
        let process_indices: HashMap<ProcessId, usize> = self
            .context
            .process_ops
            .keys()
            .map(|&pid| (pid, 0))
            .collect();

        self.backtrack(initial_state, process_indices, Vec::new())
    }

    fn backtrack(
        &mut self,
        state: M::State,
        process_indices: HashMap<ProcessId, usize>,
        ordering: Vec<OperationId>,
    ) -> SeqSearchResult {
        // Check termination
        if self.cancelled.load(Ordering::Relaxed) {
            return SeqSearchResult::Timeout;
        }

        if let Some(deadline) = self.deadline {
            if Instant::now() >= deadline {
                return SeqSearchResult::Timeout;
            }
        }

        let states = self.stats.states_explored.load(Ordering::Relaxed);
        if let Some(max) = self.max_states {
            if states >= max {
                return SeqSearchResult::MaxStatesReached;
            }
        }

        self.stats.states_explored.fetch_add(1, Ordering::Relaxed);
        self.stats
            .max_depth
            .fetch_max(ordering.len() as u64, Ordering::Relaxed);

        // Check if complete
        if ordering.len() == self.context.all_ops.len() {
            self.stats.orderings_tried.fetch_add(1, Ordering::Relaxed);
            return SeqSearchResult::Found(ordering);
        }

        // Get candidates
        let candidates = self.context.get_next_candidates(&process_indices);
        if candidates.is_empty() {
            self.stats.backtracks.fetch_add(1, Ordering::Relaxed);
            return SeqSearchResult::NotFound(ordering);
        }

        // Try each candidate
        for candidate in candidates {
            let op = match self.context.operations.get(&candidate) {
                Some(op) => op,
                None => continue,
            };

            let model_op = match self.context.model_ops.get(&candidate) {
                Some(op) => op,
                None => continue,
            };

            // Check if executable
            if !self.context.model.can_execute(&state, model_op) {
                self.stats.pruned.fetch_add(1, Ordering::Relaxed);
                continue;
            }

            // Execute
            let (new_state, expected) = self.context.model.step(&state, model_op);

            if !self.context.extractor.validate(op, &expected) {
                self.stats.pruned.fetch_add(1, Ordering::Relaxed);
                continue;
            }

            // Update indices
            let mut new_indices = process_indices.clone();
            *new_indices.entry(op.process).or_insert(0) += 1;

            let mut new_ordering = ordering.clone();
            new_ordering.push(candidate);

            let result = self.backtrack(new_state, new_indices, new_ordering);

            match result {
                SeqSearchResult::Found(_) => return result,
                SeqSearchResult::Timeout => return result,
                SeqSearchResult::MaxStatesReached => return result,
                SeqSearchResult::NotFound(_) => {
                    self.stats.backtracks.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        SeqSearchResult::NotFound(ordering)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mamut_core::history::RunId;
    use mamut_core::operation::{OperationPhase, OVC};

    // Reuse register model from linearizability tests
    #[derive(Clone)]
    struct RegisterModel;

    #[derive(Clone, Debug, PartialEq)]
    struct RegisterState(i32);

    #[derive(Clone)]
    enum RegisterOp {
        Read(i32),
        Write(i32),
    }

    impl Model for RegisterModel {
        type Op = RegisterOp;
        type State = RegisterState;

        fn init(&self) -> Self::State {
            RegisterState(0)
        }

        fn step(&self, state: &Self::State, op: &Self::Op) -> (Self::State, ExpectedResult) {
            match op {
                RegisterOp::Read(v) => {
                    if *v == state.0 {
                        (state.clone(), ExpectedResult::Ok)
                    } else {
                        (state.clone(), ExpectedResult::Err("mismatch".into()))
                    }
                }
                RegisterOp::Write(v) => (RegisterState(*v), ExpectedResult::Ok),
            }
        }

        fn equivalent(&self, s1: &Self::State, s2: &Self::State) -> bool {
            s1 == s2
        }
    }

    #[derive(Clone)]
    struct RegisterExtractor;

    impl OperationExtractor<serde_json::Value> for RegisterExtractor {
        type ModelOp = RegisterOp;

        fn extract(&self, op: &Operation<serde_json::Value>) -> Self::ModelOp {
            if op.function == "write" {
                let v = op.args["value"].as_i64().unwrap_or(0) as i32;
                RegisterOp::Write(v)
            } else {
                if let OperationPhase::Completed { ref result } = op.phase {
                    let v = result.as_i64().unwrap_or(0) as i32;
                    RegisterOp::Read(v)
                } else {
                    RegisterOp::Read(0)
                }
            }
        }

        fn validate(&self, _op: &Operation<serde_json::Value>, expected: &ExpectedResult) -> bool {
            matches!(expected, ExpectedResult::Ok | ExpectedResult::Any)
        }
    }

    fn make_write_op(id: u64, process: u32, value: i32, ovc: OVC) -> Operation<serde_json::Value> {
        Operation::new(
            OperationId(id),
            ProcessId(process),
            "write",
            serde_json::json!({"value": value}),
            ovc,
        )
        .with_completion(serde_json::json!(null), ovc.increment())
    }

    fn make_read_op(
        id: u64,
        process: u32,
        result: i32,
        ovc: OVC,
    ) -> Operation<serde_json::Value> {
        Operation::new(
            OperationId(id),
            ProcessId(process),
            "read",
            serde_json::json!({}),
            ovc,
        )
        .with_completion(serde_json::json!(result), ovc.increment())
    }

    #[tokio::test]
    async fn test_sequential_consistency_simple() {
        let mut history = History::new(RunId::new());

        // Simple sequential history
        history.push(make_write_op(1, 1, 42, OVC::new(0)));
        history.push(make_read_op(2, 1, 42, OVC::new(100)));

        let checker = SequentialConsistencyChecker::new(RegisterModel, RegisterExtractor);
        let result = checker.check(&history).await;

        assert!(result.is_pass());
    }

    #[tokio::test]
    async fn test_sequential_consistency_multiprocess() {
        let mut history = History::new(RunId::new());

        // Two processes with interleaved operations
        history.push(make_write_op(1, 1, 1, OVC::new(0)));
        history.push(make_write_op(2, 2, 2, OVC::new(5)));
        history.push(make_read_op(3, 1, 2, OVC::new(100)));

        let checker = SequentialConsistencyChecker::new(RegisterModel, RegisterExtractor);
        let result = checker.check(&history).await;

        // Should be sequentially consistent: W(1), W(2), R(2)
        assert!(result.is_pass());
    }
}
