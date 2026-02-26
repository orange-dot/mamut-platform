//! Result types for consistency checkers.

use std::fmt;
use std::time::Duration;

use mamut_core::OperationId;

/// The result of a consistency check.
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// The overall status of the check.
    pub status: CheckStatus,
    /// Counterexample if the check failed.
    pub counterexample: Option<Counterexample>,
    /// List of violations found (may be empty for passing checks).
    pub violations: Vec<Violation>,
    /// Time taken to perform the check.
    pub duration: Option<Duration>,
    /// Additional diagnostic information.
    pub diagnostics: Vec<String>,
    /// Statistics about the check process.
    pub stats: CheckStats,
}

impl CheckResult {
    /// Create a passing result.
    pub fn pass() -> Self {
        Self {
            status: CheckStatus::Pass,
            counterexample: None,
            violations: Vec::new(),
            duration: None,
            diagnostics: Vec::new(),
            stats: CheckStats::default(),
        }
    }

    /// Create a failing result with a counterexample.
    pub fn fail(counterexample: Counterexample) -> Self {
        Self {
            status: CheckStatus::Fail,
            counterexample: Some(counterexample),
            violations: Vec::new(),
            duration: None,
            diagnostics: Vec::new(),
            stats: CheckStats::default(),
        }
    }

    /// Create an unknown result (e.g., due to timeout).
    pub fn unknown(reason: String) -> Self {
        Self {
            status: CheckStatus::Unknown(reason),
            counterexample: None,
            violations: Vec::new(),
            duration: None,
            diagnostics: Vec::new(),
            stats: CheckStats::default(),
        }
    }

    /// Create an error result.
    pub fn error(message: String) -> Self {
        Self {
            status: CheckStatus::Error(message),
            counterexample: None,
            violations: Vec::new(),
            duration: None,
            diagnostics: Vec::new(),
            stats: CheckStats::default(),
        }
    }

    /// Add the duration to this result.
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Add violations to this result.
    pub fn with_violations(mut self, violations: Vec<Violation>) -> Self {
        self.violations = violations;
        self
    }

    /// Add a diagnostic message.
    pub fn with_diagnostic(mut self, message: String) -> Self {
        self.diagnostics.push(message);
        self
    }

    /// Add statistics to this result.
    pub fn with_stats(mut self, stats: CheckStats) -> Self {
        self.stats = stats;
        self
    }

    /// Check if this result indicates the history passed.
    pub fn is_pass(&self) -> bool {
        matches!(self.status, CheckStatus::Pass)
    }

    /// Check if this result indicates a failure.
    pub fn is_fail(&self) -> bool {
        matches!(self.status, CheckStatus::Fail)
    }

    /// Check if the result is inconclusive.
    pub fn is_unknown(&self) -> bool {
        matches!(self.status, CheckStatus::Unknown(_))
    }
}

impl fmt::Display for CheckResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CheckResult {{ status: {}", self.status)?;
        if let Some(duration) = self.duration {
            write!(f, ", duration: {:?}", duration)?;
        }
        if let Some(ref ce) = self.counterexample {
            write!(f, ", counterexample: {} operations", ce.operations.len())?;
        }
        write!(f, " }}")
    }
}

/// The status of a consistency check.
#[derive(Debug, Clone, PartialEq)]
pub enum CheckStatus {
    /// The history satisfies the consistency model.
    Pass,
    /// The history violates the consistency model.
    Fail,
    /// The check could not determine consistency (e.g., timeout).
    Unknown(String),
    /// An error occurred during checking.
    Error(String),
}

impl fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckStatus::Pass => write!(f, "PASS"),
            CheckStatus::Fail => write!(f, "FAIL"),
            CheckStatus::Unknown(reason) => write!(f, "UNKNOWN: {}", reason),
            CheckStatus::Error(msg) => write!(f, "ERROR: {}", msg),
        }
    }
}

/// A counterexample demonstrating a consistency violation.
#[derive(Debug, Clone)]
pub struct Counterexample {
    /// A human-readable description of the violation.
    pub description: String,
    /// The operations involved in the counterexample.
    pub operations: Vec<CounterexampleOp>,
    /// The type of violation.
    pub violation_type: ViolationType,
    /// A suggested valid linearization (if one exists).
    pub suggested_linearization: Option<Vec<OperationId>>,
}

impl Counterexample {
    /// Create a new counterexample.
    pub fn new(description: String, violation_type: ViolationType) -> Self {
        Self {
            description,
            operations: Vec::new(),
            violation_type,
            suggested_linearization: None,
        }
    }

    /// Add an operation to this counterexample.
    pub fn with_operation(mut self, op: CounterexampleOp) -> Self {
        self.operations.push(op);
        self
    }

    /// Add multiple operations to this counterexample.
    pub fn with_operations(mut self, ops: Vec<CounterexampleOp>) -> Self {
        self.operations = ops;
        self
    }

    /// Add a suggested linearization.
    pub fn with_suggestion(mut self, linearization: Vec<OperationId>) -> Self {
        self.suggested_linearization = Some(linearization);
        self
    }
}

impl fmt::Display for Counterexample {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Counterexample: {}", self.description)?;
        writeln!(f, "  Type: {:?}", self.violation_type)?;
        writeln!(f, "  Operations:")?;
        for op in &self.operations {
            writeln!(f, "    - {}", op)?;
        }
        Ok(())
    }
}

/// An operation in a counterexample.
#[derive(Debug, Clone)]
pub struct CounterexampleOp {
    /// The operation ID.
    pub id: OperationId,
    /// A description of the operation.
    pub description: String,
    /// The expected value/result according to the model.
    pub expected: String,
    /// The actual observed value/result.
    pub actual: String,
    /// The time interval of this operation (start_timestamp, end_timestamp).
    pub interval: Option<(u64, u64)>,
}

impl CounterexampleOp {
    /// Create a new counterexample operation.
    pub fn new(id: OperationId, description: String) -> Self {
        Self {
            id,
            description,
            expected: String::new(),
            actual: String::new(),
            interval: None,
        }
    }

    /// Set the expected value.
    pub fn with_expected(mut self, expected: impl Into<String>) -> Self {
        self.expected = expected.into();
        self
    }

    /// Set the actual value.
    pub fn with_actual(mut self, actual: impl Into<String>) -> Self {
        self.actual = actual.into();
        self
    }

    /// Set the time interval.
    pub fn with_interval(mut self, start: u64, end: u64) -> Self {
        self.interval = Some((start, end));
        self
    }
}

impl fmt::Display for CounterexampleOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.id, self.description)?;
        if !self.expected.is_empty() || !self.actual.is_empty() {
            write!(f, " (expected: {}, actual: {})", self.expected, self.actual)?;
        }
        if let Some((start, end)) = self.interval {
            write!(f, " [{}-{}]", start, end)?;
        }
        Ok(())
    }
}

/// A specific violation found during checking.
#[derive(Debug, Clone)]
pub struct Violation {
    /// The type of violation.
    pub violation_type: ViolationType,
    /// The operations involved.
    pub operations: Vec<OperationId>,
    /// A description of why this is a violation.
    pub description: String,
    /// The state at which the violation was detected.
    pub state_description: Option<String>,
}

impl Violation {
    /// Create a new violation.
    pub fn new(violation_type: ViolationType, description: String) -> Self {
        Self {
            violation_type,
            operations: Vec::new(),
            description,
            state_description: None,
        }
    }

    /// Add involved operations.
    pub fn with_operations(mut self, ops: Vec<OperationId>) -> Self {
        self.operations = ops;
        self
    }

    /// Add state description.
    pub fn with_state(mut self, state: String) -> Self {
        self.state_description = Some(state);
        self
    }
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.violation_type, self.description)?;
        if !self.operations.is_empty() {
            write!(f, " (operations: ")?;
            for (i, op) in self.operations.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", op)?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

/// Types of consistency violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViolationType {
    /// No valid linearization exists.
    NoLinearization,
    /// Real-time order violated (linearization doesn't respect happens-before).
    RealTimeOrderViolation,
    /// Program order violated (operations from same process out of order).
    ProgramOrderViolation,
    /// Read returned a value that was never written.
    StaleRead,
    /// Read returned a value from the future.
    FutureRead,
    /// Causal dependency violated.
    CausalityViolation,
    /// Write-write conflict not properly ordered.
    WriteConflict,
    /// Custom invariant violated.
    InvariantViolation,
    /// Model specification violation.
    SpecificationViolation,
    /// Unknown violation type.
    Unknown,
}

impl fmt::Display for ViolationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViolationType::NoLinearization => write!(f, "No Valid Linearization"),
            ViolationType::RealTimeOrderViolation => write!(f, "Real-Time Order Violation"),
            ViolationType::ProgramOrderViolation => write!(f, "Program Order Violation"),
            ViolationType::StaleRead => write!(f, "Stale Read"),
            ViolationType::FutureRead => write!(f, "Future Read"),
            ViolationType::CausalityViolation => write!(f, "Causality Violation"),
            ViolationType::WriteConflict => write!(f, "Write Conflict"),
            ViolationType::InvariantViolation => write!(f, "Invariant Violation"),
            ViolationType::SpecificationViolation => write!(f, "Specification Violation"),
            ViolationType::Unknown => write!(f, "Unknown Violation"),
        }
    }
}

/// Statistics about the checking process.
#[derive(Debug, Clone, Default)]
pub struct CheckStats {
    /// Number of operations in the history.
    pub num_operations: usize,
    /// Number of states explored.
    pub states_explored: u64,
    /// Number of linearizations tried.
    pub linearizations_tried: u64,
    /// Maximum search depth reached.
    pub max_depth: usize,
    /// Number of backtracks performed.
    pub backtracks: u64,
    /// Number of pruned branches.
    pub pruned: u64,
    /// Peak memory usage in bytes (if tracked).
    pub peak_memory_bytes: Option<usize>,
}

impl CheckStats {
    /// Create empty stats.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record exploring a new state.
    pub fn record_state(&mut self) {
        self.states_explored += 1;
    }

    /// Record trying a linearization.
    pub fn record_linearization(&mut self) {
        self.linearizations_tried += 1;
    }

    /// Record a backtrack.
    pub fn record_backtrack(&mut self) {
        self.backtracks += 1;
    }

    /// Record pruning a branch.
    pub fn record_prune(&mut self) {
        self.pruned += 1;
    }

    /// Update max depth.
    pub fn update_depth(&mut self, depth: usize) {
        if depth > self.max_depth {
            self.max_depth = depth;
        }
    }
}

impl fmt::Display for CheckStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Stats {{ ops: {}, states: {}, linearizations: {}, backtracks: {}, pruned: {} }}",
            self.num_operations,
            self.states_explored,
            self.linearizations_tried,
            self.backtracks,
            self.pruned
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_result_pass() {
        let result = CheckResult::pass();
        assert!(result.is_pass());
        assert!(!result.is_fail());
        assert!(!result.is_unknown());
    }

    #[test]
    fn test_check_result_fail() {
        let ce = Counterexample::new("test violation".into(), ViolationType::NoLinearization);
        let result = CheckResult::fail(ce);
        assert!(!result.is_pass());
        assert!(result.is_fail());
        assert!(result.counterexample.is_some());
    }

    #[test]
    fn test_check_result_unknown() {
        let result = CheckResult::unknown("timeout".into());
        assert!(!result.is_pass());
        assert!(!result.is_fail());
        assert!(result.is_unknown());
    }

    #[test]
    fn test_counterexample_display() {
        let ce = Counterexample::new("Test violation".into(), ViolationType::StaleRead)
            .with_operation(
                CounterexampleOp::new(OperationId(1), "Read(x)".into())
                    .with_expected("42")
                    .with_actual("0")
                    .with_interval(10, 20),
            );

        let display = format!("{}", ce);
        assert!(display.contains("Test violation"));
        assert!(display.contains("StaleRead"));
        assert!(display.contains("Read(x)"));
    }

    #[test]
    fn test_check_stats() {
        let mut stats = CheckStats::new();
        stats.record_state();
        stats.record_state();
        stats.record_linearization();
        stats.record_backtrack();
        stats.update_depth(5);

        assert_eq!(stats.states_explored, 2);
        assert_eq!(stats.linearizations_tried, 1);
        assert_eq!(stats.backtracks, 1);
        assert_eq!(stats.max_depth, 5);
    }
}
