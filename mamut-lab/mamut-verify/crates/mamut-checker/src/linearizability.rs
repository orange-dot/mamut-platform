//! Linearizability checker using the Wing-Gong-Luchangco (WGL) algorithm.
//!
//! This module implements a linearizability checker based on the WGL algorithm,
//! which uses a backtracking search to find a valid linearization of concurrent
//! operations.
//!
//! # Algorithm Overview
//!
//! 1. Build call/return pairs from the history
//! 2. Extract real-time constraints (operation A must come before B if A.return < B.call)
//! 3. Use backtracking search to find a linearization that:
//!    - Respects real-time constraints
//!    - Satisfies the sequential specification (model)
//!
//! # Complexity
//!
//! Linearizability checking is NP-complete in general, so this implementation
//! includes timeout handling and optional parallel search.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use mamut_core::{History, Operation, OperationId, ProcessId};
use serde::{de::DeserializeOwned, Serialize};

use crate::result::{CheckResult, CheckStats, Counterexample, CounterexampleOp, ViolationType};
use crate::traits::{Checker, Model};
use crate::ExpectedResult;

/// Configuration for the linearizability checker.
#[derive(Debug, Clone)]
pub struct LinearizabilityConfig {
    /// Maximum time to spend checking (None for no timeout).
    pub timeout: Option<Duration>,
    /// Whether to use parallel search.
    pub parallel: bool,
    /// Number of worker threads for parallel search.
    pub num_workers: usize,
    /// Maximum number of states to explore before giving up.
    pub max_states: Option<u64>,
    /// Enable state caching for cycle detection.
    pub enable_caching: bool,
    /// Enable early termination on first valid linearization.
    pub early_termination: bool,
}

impl Default for LinearizabilityConfig {
    fn default() -> Self {
        Self {
            timeout: Some(Duration::from_secs(30)),
            parallel: false,
            num_workers: 4,
            max_states: Some(10_000_000),
            enable_caching: true,
            early_termination: true,
        }
    }
}

impl LinearizabilityConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Disable timeout.
    pub fn without_timeout(mut self) -> Self {
        self.timeout = None;
        self
    }

    /// Enable parallel search.
    pub fn with_parallel(mut self, num_workers: usize) -> Self {
        self.parallel = true;
        self.num_workers = num_workers;
        self
    }

    /// Set maximum states to explore.
    pub fn with_max_states(mut self, max_states: u64) -> Self {
        self.max_states = Some(max_states);
        self
    }
}

/// Trait for extracting model operations from history operations.
pub trait OperationExtractor<V>: Send + Sync + Clone {
    /// The model operation type.
    type ModelOp;

    /// Extract a model operation from a history operation.
    fn extract(&self, op: &Operation<V>) -> Self::ModelOp;

    /// Validate that an operation result matches the expected result.
    fn validate(&self, op: &Operation<V>, expected: &ExpectedResult) -> bool;
}

/// A linearizability checker using the WGL algorithm.
///
/// # Type Parameters
///
/// * `M` - The model type that defines the sequential specification.
/// * `V` - The value type in operations.
/// * `E` - The extractor that converts history operations to model operations.
pub struct LinearizabilityChecker<M, V, E>
where
    M: Model,
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
    E: OperationExtractor<V, ModelOp = M::Op>,
{
    /// The sequential specification model.
    model: M,
    /// Configuration for the checker.
    config: LinearizabilityConfig,
    /// Extractor to convert operations.
    extractor: E,
    /// Phantom data for V.
    _phantom: std::marker::PhantomData<V>,
}

impl<M, V, E> LinearizabilityChecker<M, V, E>
where
    M: Model,
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
    E: OperationExtractor<V, ModelOp = M::Op>,
{
    /// Create a new linearizability checker.
    pub fn new(model: M, extractor: E) -> Self {
        Self {
            model,
            config: LinearizabilityConfig::default(),
            extractor,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(model: M, extractor: E, config: LinearizabilityConfig) -> Self {
        Self {
            model,
            config,
            extractor,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set the timeout for checking.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = Some(timeout);
        self
    }

    /// Enable parallel search.
    pub fn with_parallel(mut self, num_workers: usize) -> Self {
        self.config.parallel = true;
        self.config.num_workers = num_workers;
        self
    }
}

#[async_trait]
impl<M, V, E> Checker for LinearizabilityChecker<M, V, E>
where
    M: Model + 'static,
    M::State: std::fmt::Debug,
    V: Serialize + DeserializeOwned + Send + Sync + Clone + std::fmt::Debug + 'static,
    E: OperationExtractor<V, ModelOp = M::Op> + 'static,
{
    type Value = V;

    fn name(&self) -> &str {
        "linearizability"
    }

    fn description(&self) -> &str {
        "Checks that the history can be linearized to satisfy the sequential specification"
    }

    async fn check(&self, history: &History<Self::Value>) -> CheckResult {
        let start = Instant::now();

        if history.is_empty() {
            return CheckResult::pass().with_duration(start.elapsed());
        }

        // Build the search context
        let context = match SearchContext::build(history, &self.model, &self.extractor) {
            Ok(ctx) => ctx,
            Err(e) => return CheckResult::error(e),
        };

        // Run the search
        let cancelled = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(SearchStats::new());

        let result = if self.config.parallel && self.config.num_workers > 1 {
            self.parallel_search(&context, cancelled.clone(), stats.clone())
                .await
        } else {
            self.sequential_search(&context, cancelled.clone(), stats.clone())
        };

        let duration = start.elapsed();
        let check_stats = CheckStats {
            num_operations: history.len(),
            states_explored: stats.states_explored.load(Ordering::Relaxed),
            linearizations_tried: stats.linearizations_tried.load(Ordering::Relaxed),
            backtracks: stats.backtracks.load(Ordering::Relaxed),
            pruned: stats.pruned.load(Ordering::Relaxed),
            max_depth: stats.max_depth.load(Ordering::Relaxed) as usize,
            peak_memory_bytes: None,
        };

        match result {
            SearchResult::Found(linearization) => CheckResult::pass()
                .with_duration(duration)
                .with_stats(check_stats)
                .with_diagnostic(format!("Found linearization: {:?}", linearization)),

            SearchResult::NotFound(partial) => {
                let ce = self.build_counterexample(history, &context, partial);
                CheckResult::fail(ce)
                    .with_duration(duration)
                    .with_stats(check_stats)
            }

            SearchResult::Timeout => CheckResult::unknown("Search timed out".into())
                .with_duration(duration)
                .with_stats(check_stats),

            SearchResult::MaxStatesReached => {
                CheckResult::unknown("Maximum states reached".into())
                    .with_duration(duration)
                    .with_stats(check_stats)
            }

            SearchResult::Cancelled => CheckResult::unknown("Search cancelled".into())
                .with_duration(duration)
                .with_stats(check_stats),

            SearchResult::Error(e) => CheckResult::error(e)
                .with_duration(duration)
                .with_stats(check_stats),
        }
    }
}

impl<M, V, E> LinearizabilityChecker<M, V, E>
where
    M: Model,
    M::State: std::fmt::Debug,
    V: Serialize + DeserializeOwned + Send + Sync + Clone + std::fmt::Debug + 'static,
    E: OperationExtractor<V, ModelOp = M::Op>,
{
    /// Perform sequential (single-threaded) search.
    fn sequential_search(
        &self,
        context: &SearchContext<M, V, E>,
        cancelled: Arc<AtomicBool>,
        stats: Arc<SearchStats>,
    ) -> SearchResult {
        let deadline = self.config.timeout.map(|t| Instant::now() + t);
        let max_states = self.config.max_states;

        let mut searcher = BacktrackingSearcher::new(
            context,
            deadline,
            max_states,
            cancelled,
            stats,
            self.config.early_termination,
        );

        searcher.search()
    }

    /// Perform parallel search using work stealing.
    async fn parallel_search(
        &self,
        context: &SearchContext<M, V, E>,
        cancelled: Arc<AtomicBool>,
        stats: Arc<SearchStats>,
    ) -> SearchResult {
        // For simplicity, we use a parallel-prefix approach where each worker
        // explores different initial operation orderings.
        let deadline = self.config.timeout.map(|t| Instant::now() + t);
        let max_states = self.config.max_states;

        // Get initial candidates
        let initial_ops = context.get_initially_linearizable();
        if initial_ops.is_empty() {
            return SearchResult::NotFound(Vec::new());
        }

        // If only one initial candidate, fall back to sequential
        if initial_ops.len() == 1 {
            return self.sequential_search(context, cancelled, stats);
        }

        // Split work among workers
        let num_workers = self.config.num_workers.min(initial_ops.len());
        let found = Arc::new(AtomicBool::new(false));
        let result_linearization = Arc::new(std::sync::Mutex::new(None));

        let mut handles = Vec::new();

        for chunk in initial_ops.chunks((initial_ops.len() + num_workers - 1) / num_workers) {
            let context = context.clone();
            let cancelled = cancelled.clone();
            let found = found.clone();
            let stats = stats.clone();
            let result_linearization = result_linearization.clone();
            let chunk: Vec<_> = chunk.to_vec();

            let handle = tokio::task::spawn_blocking(move || {
                for &start_op in &chunk {
                    if cancelled.load(Ordering::Relaxed) || found.load(Ordering::Relaxed) {
                        break;
                    }

                    let mut searcher = BacktrackingSearcher::new(
                        &context,
                        deadline,
                        max_states.map(|m| m / num_workers as u64),
                        cancelled.clone(),
                        stats.clone(),
                        true,
                    );

                    if let SearchResult::Found(lin) = searcher.search_from(start_op) {
                        found.store(true, Ordering::Relaxed);
                        *result_linearization.lock().unwrap() = Some(lin);
                        break;
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all workers
        for handle in handles {
            let _ = handle.await;
        }

        // Check result
        if found.load(Ordering::Relaxed) {
            if let Some(lin) = result_linearization.lock().unwrap().take() {
                return SearchResult::Found(lin);
            }
        }

        // Check termination reason
        if cancelled.load(Ordering::Relaxed) {
            SearchResult::Cancelled
        } else if deadline.map(|d| Instant::now() >= d).unwrap_or(false) {
            SearchResult::Timeout
        } else {
            SearchResult::NotFound(Vec::new())
        }
    }

    /// Build a counterexample from a failed search.
    fn build_counterexample(
        &self,
        history: &History<V>,
        _context: &SearchContext<M, V, E>,
        partial: Vec<OperationId>,
    ) -> Counterexample {
        let mut ce = Counterexample::new(
            "No valid linearization found".into(),
            ViolationType::NoLinearization,
        );

        // Find conflicting operations
        let linearized: HashSet<_> = partial.iter().copied().collect();
        let remaining: Vec<_> = history
            .operations
            .iter()
            .filter(|op| !linearized.contains(&op.id))
            .collect();

        // Add partial linearization info
        if !partial.is_empty() {
            ce.description = format!(
                "Linearized {} of {} operations before getting stuck",
                partial.len(),
                history.len()
            );
        }

        // Add operations that couldn't be linearized
        for op in remaining {
            let ce_op = CounterexampleOp::new(op.id, format!("{}({:?})", op.function, op.args))
                .with_interval(op.ovc.timestamp(), op.ovc.timestamp());
            ce = ce.with_operation(ce_op);
        }

        ce
    }
}

/// Context for the search algorithm.
#[derive(Clone)]
struct SearchContext<M, V, E>
where
    M: Model,
    V: Clone,
    E: OperationExtractor<V, ModelOp = M::Op>,
{
    /// The model.
    model: M,
    /// The extractor.
    extractor: E,
    /// Operations indexed by ID.
    operations: HashMap<OperationId, Operation<V>>,
    /// Model operations indexed by operation ID.
    model_ops: HashMap<OperationId, M::Op>,
    /// Real-time constraints: (a, b) means a must come before b.
    #[allow(dead_code)]
    must_precede: HashMap<OperationId, HashSet<OperationId>>,
    /// Reverse constraints: operations that must come after.
    must_follow: HashMap<OperationId, HashSet<OperationId>>,
    /// All operation IDs in the history.
    all_ops: Vec<OperationId>,
}

impl<M, V, E> SearchContext<M, V, E>
where
    M: Model,
    V: Clone,
    E: OperationExtractor<V, ModelOp = M::Op>,
{
    /// Build the search context from a history.
    fn build(history: &History<V>, model: &M, extractor: &E) -> Result<Self, String> {
        let mut operations = HashMap::new();
        let mut model_ops = HashMap::new();
        let mut must_precede: HashMap<OperationId, HashSet<OperationId>> = HashMap::new();
        let mut must_follow: HashMap<OperationId, HashSet<OperationId>> = HashMap::new();
        let mut all_ops = Vec::new();

        // Index operations and extract model ops
        for op in &history.operations {
            operations.insert(op.id, op.clone());
            model_ops.insert(op.id, extractor.extract(op));
            all_ops.push(op.id);
        }

        // Build real-time constraints from OVC timestamps
        // Operation A must precede B if A completes before B starts
        // We use OVC timestamps as a proxy for real-time ordering
        for op1 in &history.operations {
            for op2 in &history.operations {
                if op1.id == op2.id {
                    continue;
                }

                // If op1's timestamp is strictly less than op2's, op1 happened before op2
                if op1.ovc.happens_before(&op2.ovc) {
                    must_precede.entry(op1.id).or_default().insert(op2.id);
                    must_follow.entry(op2.id).or_default().insert(op1.id);
                }
            }
        }

        // Also respect program order (operations from same process)
        let mut process_ops: HashMap<ProcessId, Vec<&Operation<V>>> = HashMap::new();
        for op in &history.operations {
            process_ops.entry(op.process).or_default().push(op);
        }

        for (_, ops) in &mut process_ops {
            ops.sort_by_key(|op| op.ovc.timestamp());
            for window in ops.windows(2) {
                let (op1, op2) = (window[0], window[1]);
                must_precede.entry(op1.id).or_default().insert(op2.id);
                must_follow.entry(op2.id).or_default().insert(op1.id);
            }
        }

        Ok(Self {
            model: model.clone(),
            extractor: extractor.clone(),
            operations,
            model_ops,
            must_precede,
            must_follow,
            all_ops,
        })
    }

    /// Get operations that can be linearized first (no predecessors).
    fn get_initially_linearizable(&self) -> Vec<OperationId> {
        self.all_ops
            .iter()
            .filter(|id| {
                self.must_follow
                    .get(id)
                    .map(|s| s.is_empty())
                    .unwrap_or(true)
            })
            .copied()
            .collect()
    }

    /// Get operations that can be linearized next given current linearization.
    fn get_next_candidates(&self, linearized: &HashSet<OperationId>) -> Vec<OperationId> {
        self.all_ops
            .iter()
            .filter(|id| {
                if linearized.contains(id) {
                    return false;
                }
                // All predecessors must be linearized
                self.must_follow
                    .get(id)
                    .map(|preds| preds.iter().all(|p| linearized.contains(p)))
                    .unwrap_or(true)
            })
            .copied()
            .collect()
    }
}

/// Atomic statistics for search progress.
struct SearchStats {
    states_explored: AtomicU64,
    linearizations_tried: AtomicU64,
    backtracks: AtomicU64,
    pruned: AtomicU64,
    max_depth: AtomicU64,
}

impl SearchStats {
    fn new() -> Self {
        Self {
            states_explored: AtomicU64::new(0),
            linearizations_tried: AtomicU64::new(0),
            backtracks: AtomicU64::new(0),
            pruned: AtomicU64::new(0),
            max_depth: AtomicU64::new(0),
        }
    }
}

/// Result of the search.
enum SearchResult {
    /// Found a valid linearization.
    Found(Vec<OperationId>),
    /// No linearization exists.
    NotFound(Vec<OperationId>),
    /// Search timed out.
    Timeout,
    /// Maximum states reached.
    MaxStatesReached,
    /// Search was cancelled.
    Cancelled,
    /// Error during search.
    Error(String),
}

/// Backtracking searcher for linearizations.
struct BacktrackingSearcher<'a, M, V, E>
where
    M: Model,
    V: Clone,
    E: OperationExtractor<V, ModelOp = M::Op>,
{
    context: &'a SearchContext<M, V, E>,
    deadline: Option<Instant>,
    max_states: Option<u64>,
    cancelled: Arc<AtomicBool>,
    stats: Arc<SearchStats>,
    early_termination: bool,
}

impl<'a, M, V, E> BacktrackingSearcher<'a, M, V, E>
where
    M: Model,
    V: Clone + std::fmt::Debug,
    E: OperationExtractor<V, ModelOp = M::Op>,
{
    fn new(
        context: &'a SearchContext<M, V, E>,
        deadline: Option<Instant>,
        max_states: Option<u64>,
        cancelled: Arc<AtomicBool>,
        stats: Arc<SearchStats>,
        early_termination: bool,
    ) -> Self {
        Self {
            context,
            deadline,
            max_states,
            cancelled,
            stats,
            early_termination,
        }
    }

    /// Main search entry point.
    fn search(&mut self) -> SearchResult {
        let initial_state = self.context.model.init();
        let linearized = HashSet::new();
        let linearization = Vec::new();

        self.backtrack(initial_state, linearized, linearization)
    }

    /// Search starting from a specific operation.
    fn search_from(&mut self, start: OperationId) -> SearchResult {
        let initial_state = self.context.model.init();
        let op = self.context.operations.get(&start).unwrap();
        let model_op = self.context.model_ops.get(&start).unwrap();

        // Try to execute the starting operation
        let (new_state, expected) = self.context.model.step(&initial_state, model_op);
        if !self.context.extractor.validate(op, &expected) {
            return SearchResult::NotFound(vec![]);
        }

        let mut linearized = HashSet::new();
        linearized.insert(start);
        let linearization = vec![start];

        self.backtrack(new_state, linearized, linearization)
    }

    /// Recursive backtracking search.
    fn backtrack(
        &mut self,
        state: M::State,
        linearized: HashSet<OperationId>,
        linearization: Vec<OperationId>,
    ) -> SearchResult {
        // Check termination conditions
        if self.cancelled.load(Ordering::Relaxed) {
            return SearchResult::Cancelled;
        }

        if let Some(deadline) = self.deadline {
            if Instant::now() >= deadline {
                return SearchResult::Timeout;
            }
        }

        let states = self.stats.states_explored.load(Ordering::Relaxed);
        if let Some(max) = self.max_states {
            if states >= max {
                return SearchResult::MaxStatesReached;
            }
        }

        self.stats.states_explored.fetch_add(1, Ordering::Relaxed);
        self.stats
            .max_depth
            .fetch_max(linearization.len() as u64, Ordering::Relaxed);

        // Check if we've linearized all operations
        if linearization.len() == self.context.all_ops.len() {
            self.stats.linearizations_tried.fetch_add(1, Ordering::Relaxed);
            return SearchResult::Found(linearization);
        }

        // Get candidate operations to linearize next
        let candidates = self.context.get_next_candidates(&linearized);
        if candidates.is_empty() {
            // Stuck - no valid candidates but not all operations linearized
            self.stats.backtracks.fetch_add(1, Ordering::Relaxed);
            return SearchResult::NotFound(linearization);
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

            // Check if operation can be executed
            if !self.context.model.can_execute(&state, model_op) {
                self.stats.pruned.fetch_add(1, Ordering::Relaxed);
                continue;
            }

            // Execute operation on model
            let (new_state, expected) = self.context.model.step(&state, model_op);

            // Check if result matches
            if !self.context.extractor.validate(op, &expected) {
                self.stats.pruned.fetch_add(1, Ordering::Relaxed);
                continue;
            }

            // Recurse
            let mut new_linearized = linearized.clone();
            new_linearized.insert(candidate);

            let mut new_linearization = linearization.clone();
            new_linearization.push(candidate);

            let result = self.backtrack(new_state, new_linearized, new_linearization);

            match result {
                SearchResult::Found(_) if self.early_termination => return result,
                SearchResult::Found(_) => return result,
                SearchResult::Timeout => return result,
                SearchResult::MaxStatesReached => return result,
                SearchResult::Cancelled => return result,
                SearchResult::Error(_) => return result,
                SearchResult::NotFound(_) => {
                    // Continue trying other candidates
                    self.stats.backtracks.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        // No candidate worked
        SearchResult::NotFound(linearization)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mamut_core::history::RunId;
    use mamut_core::operation::{OperationPhase, OVC};

    // Simple register model for testing
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
                        (
                            state.clone(),
                            ExpectedResult::Err(format!("expected {}, got {}", state.0, v)),
                        )
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
                // Read - get value from completion result
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
    async fn test_simple_linearizable_history() {
        let mut history = History::new(RunId::new());

        // W(1) returns before R(1) starts - clearly linearizable
        history.push(make_write_op(1, 1, 1, OVC::new(0)));
        history.push(make_read_op(2, 2, 1, OVC::new(100)));

        let checker = LinearizabilityChecker::new(RegisterModel, RegisterExtractor)
            .with_timeout(Duration::from_secs(5));

        let result = checker.check(&history).await;
        assert!(
            result.is_pass(),
            "Simple sequential history should be linearizable"
        );
    }

    #[tokio::test]
    async fn test_concurrent_linearizable_history() {
        let mut history = History::new(RunId::new());

        // Concurrent operations: W(1) and R that returns 1
        // These overlap in time but R(1) is consistent with W(1) happening first
        history.push(make_write_op(1, 1, 1, OVC::new(0)));
        history.push(make_read_op(2, 2, 1, OVC::new(5)));

        let checker = LinearizabilityChecker::new(RegisterModel, RegisterExtractor)
            .with_timeout(Duration::from_secs(5));

        let result = checker.check(&history).await;
        assert!(
            result.is_pass(),
            "Concurrent history with valid linearization should pass"
        );
    }

    #[tokio::test]
    async fn test_empty_history() {
        let history: History<serde_json::Value> = History::new(RunId::new());

        let checker = LinearizabilityChecker::new(RegisterModel, RegisterExtractor);
        let result = checker.check(&history).await;
        assert!(result.is_pass(), "Empty history should be linearizable");
    }
}
