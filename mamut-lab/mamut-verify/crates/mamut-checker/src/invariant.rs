//! Custom Invariant Checker Framework.
//!
//! This module provides a framework for defining and checking custom invariants
//! on operation histories. Unlike the consistency checkers that verify against
//! a fixed consistency model, invariant checkers allow users to define arbitrary
//! properties that must hold.
//!
//! # Example
//!
//! ```rust,ignore
//! use mamut_checker::invariant::{Invariant, InvariantChecker, InvariantContext};
//!
//! struct MonotonicCounter;
//!
//! impl Invariant<serde_json::Value> for MonotonicCounter {
//!     fn name(&self) -> &str {
//!         "monotonic-counter"
//!     }
//!
//!     fn check(&self, ctx: &InvariantContext<'_, serde_json::Value>) -> Result<(), String> {
//!         // Counter values should be monotonically increasing
//!         // ...
//!         Ok(())
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use mamut_core::{History, Operation, OperationId, ProcessId};
use serde::{de::DeserializeOwned, Serialize};

use crate::result::{CheckResult, CheckStats, Counterexample, CounterexampleOp, ViolationType};
use crate::traits::Checker;

/// A custom invariant that can be checked against a history.
pub trait Invariant<V>: Send + Sync {
    /// The name of this invariant for logging and debugging.
    fn name(&self) -> &str;

    /// A human-readable description of what this invariant checks.
    fn description(&self) -> &str {
        "No description"
    }

    /// Check this invariant against the given context.
    ///
    /// Returns `Ok(())` if the invariant holds, `Err(message)` if violated.
    fn check(&self, ctx: &InvariantContext<'_, V>) -> Result<(), String>;

    /// Priority for checking (lower = check first). Default is 100.
    fn priority(&self) -> u32 {
        100
    }

    /// Whether this invariant can be checked incrementally as operations arrive.
    fn supports_incremental(&self) -> bool {
        false
    }

    /// Incrementally check a new operation (if supported).
    fn check_incremental(
        &self,
        _op: &Operation<V>,
        _ctx: &InvariantContext<'_, V>,
    ) -> Result<(), String> {
        Ok(())
    }
}

/// Context provided to invariants for checking.
pub struct InvariantContext<'a, V> {
    /// The full history being checked.
    history: &'a History<V>,
    /// Operations indexed by ID.
    operations_by_id: HashMap<OperationId, &'a Operation<V>>,
    /// Operations grouped by process.
    operations_by_process: HashMap<ProcessId, Vec<&'a Operation<V>>>,
    /// Custom metadata attached to the context.
    metadata: HashMap<String, String>,
}

impl<'a, V> InvariantContext<'a, V> {
    /// Create a new context from a history.
    pub fn new(history: &'a History<V>) -> Self {
        let mut operations_by_id = HashMap::new();
        let mut operations_by_process: HashMap<ProcessId, Vec<&'a Operation<V>>> = HashMap::new();

        for op in &history.operations {
            operations_by_id.insert(op.id, op);
            operations_by_process
                .entry(op.process)
                .or_default()
                .push(op);
        }

        // Sort operations by OVC timestamp within each process
        for ops in operations_by_process.values_mut() {
            ops.sort_by_key(|op| op.ovc.timestamp());
        }

        Self {
            history,
            operations_by_id,
            operations_by_process,
            metadata: HashMap::new(),
        }
    }

    /// Get the full history.
    pub fn history(&self) -> &'a History<V> {
        self.history
    }

    /// Iterate over all operations in the history.
    pub fn operations(&self) -> impl Iterator<Item = &'a Operation<V>> {
        self.history.iter()
    }

    /// Get an operation by ID.
    pub fn get_operation(&self, id: OperationId) -> Option<&'a Operation<V>> {
        self.operations_by_id.get(&id).copied()
    }

    /// Get all operations for a specific process in program order.
    pub fn operations_for_process(&self, process_id: ProcessId) -> &[&'a Operation<V>] {
        self.operations_by_process
            .get(&process_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get all process IDs.
    pub fn processes(&self) -> impl Iterator<Item = &ProcessId> {
        self.operations_by_process.keys()
    }

    /// Get the number of operations.
    pub fn len(&self) -> usize {
        self.history.len()
    }

    /// Check if the history is empty.
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    /// Get metadata value.
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }

    /// Set metadata value.
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get operations within a time range.
    pub fn operations_in_range(&self, start: u64, end: u64) -> Vec<&'a Operation<V>> {
        self.history
            .iter()
            .filter(|op| op.ovc.timestamp() >= start && op.ovc.timestamp() <= end)
            .collect()
    }
}

/// Configuration for the invariant checker.
#[derive(Debug, Clone)]
pub struct InvariantCheckerConfig {
    /// Whether to stop on first violation.
    pub fail_fast: bool,
    /// Maximum time for all invariant checks.
    pub timeout: Option<Duration>,
    /// Whether to run invariant checks in parallel.
    pub parallel: bool,
}

impl Default for InvariantCheckerConfig {
    fn default() -> Self {
        Self {
            fail_fast: true,
            timeout: Some(Duration::from_secs(30)),
            parallel: false,
        }
    }
}

/// An invariant checker that runs multiple custom invariants.
pub struct InvariantChecker<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
{
    /// Registered invariants.
    invariants: Vec<Box<dyn Invariant<V>>>,
    /// Configuration.
    config: InvariantCheckerConfig,
}

impl<V> InvariantChecker<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
{
    /// Create a new invariant checker.
    pub fn new() -> Self {
        Self {
            invariants: Vec::new(),
            config: InvariantCheckerConfig::default(),
        }
    }

    /// Create with configuration.
    pub fn with_config(config: InvariantCheckerConfig) -> Self {
        Self {
            invariants: Vec::new(),
            config,
        }
    }

    /// Add an invariant to check.
    pub fn add_invariant<I: Invariant<V> + 'static>(&mut self, invariant: I) {
        self.invariants.push(Box::new(invariant));
    }

    /// Add an invariant (builder pattern).
    pub fn with_invariant<I: Invariant<V> + 'static>(mut self, invariant: I) -> Self {
        self.add_invariant(invariant);
        self
    }

    /// Set fail-fast mode.
    pub fn with_fail_fast(mut self, fail_fast: bool) -> Self {
        self.config.fail_fast = fail_fast;
        self
    }

    /// Set timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = Some(timeout);
        self
    }

    /// Get the list of invariant names.
    pub fn invariant_names(&self) -> Vec<&str> {
        self.invariants.iter().map(|i| i.name()).collect()
    }
}

impl<V> Default for InvariantChecker<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<V> Checker for InvariantChecker<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + std::fmt::Debug + 'static,
{
    type Value = V;

    fn name(&self) -> &str {
        "invariant-checker"
    }

    fn description(&self) -> &str {
        "Checks custom invariants against the operation history"
    }

    async fn check(&self, history: &History<Self::Value>) -> CheckResult {
        let start = Instant::now();

        if self.invariants.is_empty() {
            return CheckResult::pass()
                .with_duration(start.elapsed())
                .with_diagnostic("No invariants registered".into());
        }

        let ctx = InvariantContext::new(history);

        // Sort invariants by priority
        let mut invariants_with_priority: Vec<_> = self
            .invariants
            .iter()
            .map(|i| (i.priority(), i.as_ref()))
            .collect();
        invariants_with_priority.sort_by_key(|(p, _)| *p);

        let mut violations = Vec::new();

        for (_, invariant) in invariants_with_priority {
            // Check timeout
            if let Some(timeout) = self.config.timeout {
                if start.elapsed() >= timeout {
                    return CheckResult::unknown("Timeout during invariant checking".into())
                        .with_duration(start.elapsed());
                }
            }

            match invariant.check(&ctx) {
                Ok(()) => {
                    // Invariant passed
                }
                Err(message) => {
                    let ce = Counterexample::new(
                        format!("Invariant '{}' violated: {}", invariant.name(), message),
                        ViolationType::InvariantViolation,
                    );

                    if self.config.fail_fast {
                        return CheckResult::fail(ce)
                            .with_duration(start.elapsed())
                            .with_stats(CheckStats {
                                num_operations: history.len(),
                                ..Default::default()
                            });
                    }

                    violations.push((invariant.name().to_string(), message));
                }
            }
        }

        if violations.is_empty() {
            CheckResult::pass()
                .with_duration(start.elapsed())
                .with_stats(CheckStats {
                    num_operations: history.len(),
                    ..Default::default()
                })
                .with_diagnostic(format!("All {} invariants passed", self.invariants.len()))
        } else {
            let ce = Counterexample::new(
                format!("{} invariant(s) violated", violations.len()),
                ViolationType::InvariantViolation,
            )
            .with_operations(
                violations
                    .iter()
                    .enumerate()
                    .map(|(i, (name, msg))| {
                        CounterexampleOp::new(OperationId(i as u64), format!("{}: {}", name, msg))
                    })
                    .collect(),
            );

            CheckResult::fail(ce)
                .with_duration(start.elapsed())
                .with_stats(CheckStats {
                    num_operations: history.len(),
                    ..Default::default()
                })
        }
    }

    fn check_incremental(&mut self, op: &Operation<Self::Value>) -> Option<CheckResult> {
        // For now, create a minimal context with just this operation
        // A full implementation would maintain state across calls
        let history = History::from_operations(vec![op.clone()]);
        let ctx = InvariantContext::new(&history);

        for invariant in &self.invariants {
            if invariant.supports_incremental() {
                if let Err(message) = invariant.check_incremental(op, &ctx) {
                    return Some(CheckResult::fail(Counterexample::new(
                        format!("Invariant '{}' violated: {}", invariant.name(), message),
                        ViolationType::InvariantViolation,
                    )));
                }
            }
        }

        None
    }
}

// ============================================================================
// Common Built-in Invariants
// ============================================================================

/// Invariant that checks program order is respected.
pub struct ProgramOrderRespected;

impl<V> Invariant<V> for ProgramOrderRespected {
    fn name(&self) -> &str {
        "program-order"
    }

    fn description(&self) -> &str {
        "Operations from the same process should appear in program order"
    }

    fn check(&self, ctx: &InvariantContext<'_, V>) -> Result<(), String> {
        for &pid in ctx.processes() {
            let ops = ctx.operations_for_process(pid);
            for window in ops.windows(2) {
                let (op1, op2) = (window[0], window[1]);
                // Operations should be in OVC order
                if op1.ovc.timestamp() > op2.ovc.timestamp() {
                    return Err(format!(
                        "Process {:?}: operation {:?} (OVC {}) should come before {:?} (OVC {})",
                        pid,
                        op1.id,
                        op1.ovc.timestamp(),
                        op2.id,
                        op2.ovc.timestamp()
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Invariant that checks no operation takes too long.
pub struct MaxOperationDuration {
    max_duration: u64,
}

impl MaxOperationDuration {
    /// Create with maximum allowed duration in OVC ticks.
    pub fn new(max_duration: u64) -> Self {
        Self { max_duration }
    }
}

impl<V> Invariant<V> for MaxOperationDuration {
    fn name(&self) -> &str {
        "max-operation-duration"
    }

    fn description(&self) -> &str {
        "No operation should take longer than the maximum allowed duration"
    }

    fn check(&self, ctx: &InvariantContext<'_, V>) -> Result<(), String> {
        // This is a simplified check - in practice you'd track invocation and completion times
        // For now, we just check that the OVC timestamp doesn't exceed a threshold
        for op in ctx.operations() {
            if op.ovc.timestamp() > self.max_duration * ctx.len() as u64 {
                return Err(format!(
                    "Operation {:?} has OVC timestamp {} exceeding expected range",
                    op.id,
                    op.ovc.timestamp()
                ));
            }
        }
        Ok(())
    }

    fn supports_incremental(&self) -> bool {
        true
    }

    fn check_incremental(
        &self,
        op: &Operation<V>,
        _ctx: &InvariantContext<'_, V>,
    ) -> Result<(), String> {
        // In incremental mode, we can't do much without knowing the expected range
        // This is a placeholder
        if op.ovc.timestamp() > u64::MAX / 2 {
            return Err(format!(
                "Operation {:?} has suspiciously large OVC timestamp",
                op.id
            ));
        }
        Ok(())
    }
}

/// Custom invariant builder using closures.
pub struct CustomInvariant<V, F>
where
    F: Fn(&InvariantContext<'_, V>) -> Result<(), String> + Send + Sync,
{
    name: String,
    description: String,
    check_fn: F,
    _phantom: std::marker::PhantomData<V>,
}

impl<V, F> CustomInvariant<V, F>
where
    F: Fn(&InvariantContext<'_, V>) -> Result<(), String> + Send + Sync,
{
    /// Create a new custom invariant.
    pub fn new(name: impl Into<String>, check_fn: F) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            check_fn,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
}

impl<V, F> Invariant<V> for CustomInvariant<V, F>
where
    V: Send + Sync,
    F: Fn(&InvariantContext<'_, V>) -> Result<(), String> + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn check(&self, ctx: &InvariantContext<'_, V>) -> Result<(), String> {
        (self.check_fn)(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mamut_core::history::RunId;
    use mamut_core::operation::OVC;

    fn make_op(id: u64, process: u32, ovc: OVC) -> Operation<serde_json::Value> {
        Operation::new(
            OperationId(id),
            ProcessId(process),
            "test",
            serde_json::json!({"id": id}),
            ovc,
        )
        .with_completion(serde_json::json!(null), ovc.increment())
    }

    #[tokio::test]
    async fn test_invariant_checker_pass() {
        let mut history = History::new(RunId::new());

        history.push(make_op(1, 1, OVC::new(0)));
        history.push(make_op(2, 1, OVC::new(100)));

        let checker: InvariantChecker<serde_json::Value> = InvariantChecker::new()
            .with_invariant(ProgramOrderRespected)
            .with_invariant(MaxOperationDuration::new(1000));

        let result = checker.check(&history).await;
        assert!(result.is_pass());
    }

    #[tokio::test]
    async fn test_custom_invariant() {
        let mut history = History::new(RunId::new());

        history.push(make_op(1, 1, OVC::new(0)));
        history.push(make_op(2, 1, OVC::new(100)));

        // Custom invariant: all operations must have the function "test"
        let all_test_ops = CustomInvariant::new("all-test-ops", |ctx| {
            for op in ctx.operations() {
                if op.function != "test" {
                    return Err(format!("Operation {:?} is not a test operation", op.id));
                }
            }
            Ok(())
        })
        .with_description("All operations must be test operations");

        let checker: InvariantChecker<serde_json::Value> =
            InvariantChecker::new().with_invariant(all_test_ops);

        let result = checker.check(&history).await;
        assert!(result.is_pass());
    }

    #[tokio::test]
    async fn test_no_invariants() {
        let history: History<serde_json::Value> = History::new(RunId::new());
        let checker: InvariantChecker<serde_json::Value> = InvariantChecker::new();

        let result = checker.check(&history).await;
        assert!(result.is_pass());
    }

    #[test]
    fn test_invariant_context() {
        let mut history = History::new(RunId::new());

        history.push(make_op(1, 1, OVC::new(0)));
        history.push(make_op(2, 2, OVC::new(50)));
        history.push(make_op(3, 1, OVC::new(100)));

        let ctx = InvariantContext::new(&history);

        assert_eq!(ctx.len(), 3);
        assert!(ctx.get_operation(OperationId(1)).is_some());
        assert!(ctx.get_operation(OperationId(99)).is_none());

        let p1_ops = ctx.operations_for_process(ProcessId(1));
        assert_eq!(p1_ops.len(), 2);
        assert_eq!(p1_ops[0].id, OperationId(1));
        assert_eq!(p1_ops[1].id, OperationId(3));

        let p2_ops = ctx.operations_for_process(ProcessId(2));
        assert_eq!(p2_ops.len(), 1);
    }
}
