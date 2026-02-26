//! Result types for TLA+ trace validation.
//!
//! This module defines the types used to represent the results of validating
//! an execution trace against a TLA+ specification using TLC.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during TLC execution.
#[derive(Debug, Error)]
pub enum TlcError {
    /// TLC executable not found.
    #[error("TLC not found at path: {path}")]
    NotFound { path: String },

    /// Java runtime not found or incompatible.
    #[error("Java runtime error: {message}")]
    JavaError { message: String },

    /// TLC process failed to start.
    #[error("Failed to start TLC: {0}")]
    ProcessStart(#[from] std::io::Error),

    /// TLC execution timed out.
    #[error("TLC execution timed out after {0:?}")]
    Timeout(Duration),

    /// TLC exited with an error.
    #[error("TLC exited with code {code}: {message}")]
    ExitError { code: i32, message: String },

    /// Failed to parse TLC output.
    #[error("Failed to parse TLC output: {0}")]
    ParseError(String),

    /// Specification file not found.
    #[error("Specification file not found: {path}")]
    SpecNotFound { path: String },

    /// Trace file not found.
    #[error("Trace file not found: {path}")]
    TraceNotFound { path: String },

    /// Invalid specification.
    #[error("Invalid TLA+ specification: {message}")]
    InvalidSpec { message: String },

    /// Configuration error.
    #[error("TLC configuration error: {message}")]
    ConfigError { message: String },
}

/// The exit status of TLC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TlcExitStatus {
    /// TLC completed successfully with no violations.
    Success,

    /// TLC found a property violation.
    Violation,

    /// TLC found a safety violation (invariant or assertion).
    SafetyViolation,

    /// TLC found a liveness violation (temporal property).
    LivenessViolation,

    /// TLC found a deadlock.
    Deadlock,

    /// TLC encountered a parsing error.
    ParseError,

    /// TLC encountered a semantic error.
    SemanticError,

    /// TLC was interrupted (e.g., by timeout).
    Interrupted,

    /// Unknown exit status.
    Unknown(i32),
}

impl TlcExitStatus {
    /// Create a TlcExitStatus from an exit code.
    pub fn from_exit_code(code: i32) -> Self {
        match code {
            0 => TlcExitStatus::Success,
            10 => TlcExitStatus::SafetyViolation,
            11 => TlcExitStatus::Violation,
            12 => TlcExitStatus::LivenessViolation,
            13 => TlcExitStatus::Deadlock,
            14 => TlcExitStatus::ParseError,
            15 => TlcExitStatus::SemanticError,
            _ => TlcExitStatus::Unknown(code),
        }
    }

    /// Check if this status indicates success.
    pub fn is_success(&self) -> bool {
        matches!(self, TlcExitStatus::Success)
    }

    /// Check if this status indicates a violation.
    pub fn is_violation(&self) -> bool {
        matches!(
            self,
            TlcExitStatus::Violation
                | TlcExitStatus::SafetyViolation
                | TlcExitStatus::LivenessViolation
                | TlcExitStatus::Deadlock
        )
    }

    /// Check if this status indicates an error (not a violation).
    pub fn is_error(&self) -> bool {
        matches!(
            self,
            TlcExitStatus::ParseError
                | TlcExitStatus::SemanticError
                | TlcExitStatus::Interrupted
                | TlcExitStatus::Unknown(_)
        )
    }
}

impl fmt::Display for TlcExitStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TlcExitStatus::Success => write!(f, "Success"),
            TlcExitStatus::Violation => write!(f, "Property Violation"),
            TlcExitStatus::SafetyViolation => write!(f, "Safety Violation"),
            TlcExitStatus::LivenessViolation => write!(f, "Liveness Violation"),
            TlcExitStatus::Deadlock => write!(f, "Deadlock"),
            TlcExitStatus::ParseError => write!(f, "Parse Error"),
            TlcExitStatus::SemanticError => write!(f, "Semantic Error"),
            TlcExitStatus::Interrupted => write!(f, "Interrupted"),
            TlcExitStatus::Unknown(code) => write!(f, "Unknown (exit code {})", code),
        }
    }
}

/// The status of a trace validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationStatus {
    /// The trace is valid according to the specification.
    Valid,

    /// The trace violates one or more properties.
    Invalid,

    /// Validation encountered an error.
    Error(String),

    /// Validation timed out.
    Timeout,

    /// Validation was inconclusive (e.g., state space too large).
    Inconclusive(String),
}

impl ValidationStatus {
    /// Check if the validation was successful.
    pub fn is_valid(&self) -> bool {
        matches!(self, ValidationStatus::Valid)
    }

    /// Check if the validation found violations.
    pub fn is_invalid(&self) -> bool {
        matches!(self, ValidationStatus::Invalid)
    }

    /// Check if the validation encountered an error.
    pub fn is_error(&self) -> bool {
        matches!(self, ValidationStatus::Error(_) | ValidationStatus::Timeout)
    }
}

impl fmt::Display for ValidationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationStatus::Valid => write!(f, "Valid"),
            ValidationStatus::Invalid => write!(f, "Invalid"),
            ValidationStatus::Error(msg) => write!(f, "Error: {}", msg),
            ValidationStatus::Timeout => write!(f, "Timeout"),
            ValidationStatus::Inconclusive(msg) => write!(f, "Inconclusive: {}", msg),
        }
    }
}

/// The result of validating a trace against a TLA+ specification.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// The overall validation status.
    pub status: ValidationStatus,

    /// Property violations found during validation.
    pub violations: Vec<PropertyViolation>,

    /// The TLC exit status.
    pub tlc_exit_status: Option<TlcExitStatus>,

    /// Time taken for validation.
    pub duration: Option<Duration>,

    /// Number of states explored.
    pub states_explored: Option<u64>,

    /// Number of distinct states found.
    pub distinct_states: Option<u64>,

    /// Counterexample trace if a violation was found.
    pub counterexample: Option<CounterexampleTrace>,

    /// Raw TLC output for debugging.
    pub raw_output: Option<String>,

    /// Additional diagnostic information.
    pub diagnostics: Vec<String>,
}

impl ValidationResult {
    /// Create a new valid result.
    pub fn valid() -> Self {
        ValidationResult {
            status: ValidationStatus::Valid,
            violations: Vec::new(),
            tlc_exit_status: Some(TlcExitStatus::Success),
            duration: None,
            states_explored: None,
            distinct_states: None,
            counterexample: None,
            raw_output: None,
            diagnostics: Vec::new(),
        }
    }

    /// Create a new invalid result with violations.
    pub fn invalid(violations: Vec<PropertyViolation>) -> Self {
        ValidationResult {
            status: ValidationStatus::Invalid,
            violations,
            tlc_exit_status: None,
            duration: None,
            states_explored: None,
            distinct_states: None,
            counterexample: None,
            raw_output: None,
            diagnostics: Vec::new(),
        }
    }

    /// Create an error result.
    pub fn error(message: impl Into<String>) -> Self {
        ValidationResult {
            status: ValidationStatus::Error(message.into()),
            violations: Vec::new(),
            tlc_exit_status: None,
            duration: None,
            states_explored: None,
            distinct_states: None,
            counterexample: None,
            raw_output: None,
            diagnostics: Vec::new(),
        }
    }

    /// Create a timeout result.
    pub fn timeout() -> Self {
        ValidationResult {
            status: ValidationStatus::Timeout,
            violations: Vec::new(),
            tlc_exit_status: Some(TlcExitStatus::Interrupted),
            duration: None,
            states_explored: None,
            distinct_states: None,
            counterexample: None,
            raw_output: None,
            diagnostics: Vec::new(),
        }
    }

    /// Check if the trace is valid.
    pub fn is_valid(&self) -> bool {
        self.status.is_valid()
    }

    /// Check if violations were found.
    pub fn is_invalid(&self) -> bool {
        self.status.is_invalid()
    }

    /// Set the TLC exit status.
    pub fn with_exit_status(mut self, status: TlcExitStatus) -> Self {
        self.tlc_exit_status = Some(status);
        self
    }

    /// Set the duration.
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Set the state counts.
    pub fn with_state_counts(mut self, explored: u64, distinct: u64) -> Self {
        self.states_explored = Some(explored);
        self.distinct_states = Some(distinct);
        self
    }

    /// Set the counterexample.
    pub fn with_counterexample(mut self, ce: CounterexampleTrace) -> Self {
        self.counterexample = Some(ce);
        self
    }

    /// Set the raw output.
    pub fn with_raw_output(mut self, output: impl Into<String>) -> Self {
        self.raw_output = Some(output.into());
        self
    }

    /// Add a diagnostic message.
    pub fn with_diagnostic(mut self, message: impl Into<String>) -> Self {
        self.diagnostics.push(message.into());
        self
    }

    /// Get the primary violation, if any.
    pub fn primary_violation(&self) -> Option<&PropertyViolation> {
        self.violations.first()
    }
}

impl fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ValidationResult {{ status: {}", self.status)?;

        if let Some(duration) = self.duration {
            write!(f, ", duration: {:?}", duration)?;
        }

        if let Some(states) = self.states_explored {
            write!(f, ", states: {}", states)?;
        }

        if !self.violations.is_empty() {
            write!(f, ", violations: {}", self.violations.len())?;
        }

        write!(f, " }}")
    }
}

/// A property violation detected during trace validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyViolation {
    /// The type of property that was violated.
    pub property_type: PropertyType,

    /// The name of the violated property (e.g., invariant name).
    pub property_name: Option<String>,

    /// A human-readable description of the violation.
    pub description: String,

    /// The step number where the violation was detected.
    pub step: Option<usize>,

    /// The state at which the violation occurred.
    pub state: Option<CounterexampleState>,

    /// The action that led to the violation.
    pub action: Option<String>,
}

impl PropertyViolation {
    /// Create a new property violation.
    pub fn new(property_type: PropertyType, description: impl Into<String>) -> Self {
        PropertyViolation {
            property_type,
            property_name: None,
            description: description.into(),
            step: None,
            state: None,
            action: None,
        }
    }

    /// Set the property name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.property_name = Some(name.into());
        self
    }

    /// Set the step number.
    pub fn with_step(mut self, step: usize) -> Self {
        self.step = Some(step);
        self
    }

    /// Set the state.
    pub fn with_state(mut self, state: CounterexampleState) -> Self {
        self.state = Some(state);
        self
    }

    /// Set the action.
    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }
}

impl fmt::Display for PropertyViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.property_type)?;

        if let Some(ref name) = self.property_name {
            write!(f, " '{}'", name)?;
        }

        write!(f, ": {}", self.description)?;

        if let Some(step) = self.step {
            write!(f, " (at step {})", step)?;
        }

        Ok(())
    }
}

/// The type of TLA+ property that was violated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertyType {
    /// An invariant (state predicate that must always be true).
    Invariant,

    /// An action property (constraint on state transitions).
    Action,

    /// A temporal property (liveness, fairness).
    Temporal,

    /// A type invariant (type correctness).
    TypeInvariant,

    /// A state constraint (limiting state space).
    StateConstraint,

    /// Deadlock (no enabled actions).
    Deadlock,

    /// An assertion failure.
    Assertion,

    /// Unknown property type.
    Unknown,
}

impl fmt::Display for PropertyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PropertyType::Invariant => write!(f, "Invariant"),
            PropertyType::Action => write!(f, "Action Property"),
            PropertyType::Temporal => write!(f, "Temporal Property"),
            PropertyType::TypeInvariant => write!(f, "Type Invariant"),
            PropertyType::StateConstraint => write!(f, "State Constraint"),
            PropertyType::Deadlock => write!(f, "Deadlock"),
            PropertyType::Assertion => write!(f, "Assertion"),
            PropertyType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// A counterexample trace showing a path to a violation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterexampleTrace {
    /// The states in the counterexample trace.
    pub states: Vec<CounterexampleState>,

    /// The property that was violated.
    pub violated_property: Option<String>,

    /// Description of the counterexample.
    pub description: Option<String>,

    /// Whether this is a lasso (loops back to an earlier state).
    pub is_lasso: bool,

    /// For lassos, the index where the loop starts.
    pub loop_start: Option<usize>,
}

impl CounterexampleTrace {
    /// Create a new counterexample trace.
    pub fn new(states: Vec<CounterexampleState>) -> Self {
        CounterexampleTrace {
            states,
            violated_property: None,
            description: None,
            is_lasso: false,
            loop_start: None,
        }
    }

    /// Set the violated property.
    pub fn with_violated_property(mut self, property: impl Into<String>) -> Self {
        self.violated_property = Some(property.into());
        self
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Mark this as a lasso with loop starting at the given index.
    pub fn with_lasso(mut self, loop_start: usize) -> Self {
        self.is_lasso = true;
        self.loop_start = Some(loop_start);
        self
    }

    /// Get the length of the counterexample.
    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// Check if the counterexample is empty.
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    /// Get the initial state.
    pub fn initial_state(&self) -> Option<&CounterexampleState> {
        self.states.first()
    }

    /// Get the final state (where violation occurs).
    pub fn final_state(&self) -> Option<&CounterexampleState> {
        self.states.last()
    }
}

impl fmt::Display for CounterexampleTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref prop) = self.violated_property {
            writeln!(f, "Counterexample for property: {}", prop)?;
        }

        if let Some(ref desc) = self.description {
            writeln!(f, "Description: {}", desc)?;
        }

        writeln!(f, "Trace ({} states):", self.states.len())?;

        for (i, state) in self.states.iter().enumerate() {
            writeln!(f, "  State {}: {}", i + 1, state)?;
        }

        if self.is_lasso {
            if let Some(loop_start) = self.loop_start {
                writeln!(f, "  (loops back to state {})", loop_start + 1)?;
            }
        }

        Ok(())
    }
}

/// A single state in a counterexample trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterexampleState {
    /// The step number (1-indexed, matching TLC output).
    pub step: usize,

    /// The action that led to this state (None for initial state).
    pub action: Option<String>,

    /// The variable values in this state.
    pub variables: HashMap<String, serde_json::Value>,

    /// Additional annotations from TLC.
    pub annotations: Option<String>,
}

impl CounterexampleState {
    /// Create a new counterexample state.
    pub fn new(step: usize) -> Self {
        CounterexampleState {
            step,
            action: None,
            variables: HashMap::new(),
            annotations: None,
        }
    }

    /// Set the action.
    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    /// Set a variable value.
    pub fn with_variable(mut self, name: impl Into<String>, value: serde_json::Value) -> Self {
        self.variables.insert(name.into(), value);
        self
    }

    /// Set multiple variable values.
    pub fn with_variables(mut self, variables: HashMap<String, serde_json::Value>) -> Self {
        self.variables = variables;
        self
    }

    /// Set annotations.
    pub fn with_annotations(mut self, annotations: impl Into<String>) -> Self {
        self.annotations = Some(annotations.into());
        self
    }

    /// Get a variable value.
    pub fn get_variable(&self, name: &str) -> Option<&serde_json::Value> {
        self.variables.get(name)
    }
}

impl fmt::Display for CounterexampleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref action) = self.action {
            write!(f, "{} -> ", action)?;
        }

        write!(f, "{{")?;

        let mut first = true;
        for (name, value) in &self.variables {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}: {}", name, value)?;
            first = false;
        }

        write!(f, "}}")
    }
}

/// Parsed output from TLC execution.
#[derive(Debug, Clone)]
pub struct ParsedTlcOutput {
    /// The TLC version string.
    pub version: Option<String>,

    /// Start time of the run.
    pub start_time: Option<String>,

    /// End time of the run.
    pub end_time: Option<String>,

    /// Number of states generated.
    pub states_generated: Option<u64>,

    /// Number of distinct states.
    pub distinct_states: Option<u64>,

    /// Number of states left on the queue.
    pub states_left: Option<u64>,

    /// Whether checking completed.
    pub completed: bool,

    /// Errors found during checking.
    pub errors: Vec<String>,

    /// Warnings found during checking.
    pub warnings: Vec<String>,

    /// Coverage information.
    pub coverage: Vec<CoverageInfo>,

    /// Counterexample if violation found.
    pub counterexample: Option<CounterexampleTrace>,
}

impl ParsedTlcOutput {
    /// Create a new empty parsed output.
    pub fn new() -> Self {
        ParsedTlcOutput {
            version: None,
            start_time: None,
            end_time: None,
            states_generated: None,
            distinct_states: None,
            states_left: None,
            completed: false,
            errors: Vec::new(),
            warnings: Vec::new(),
            coverage: Vec::new(),
            counterexample: None,
        }
    }
}

impl Default for ParsedTlcOutput {
    fn default() -> Self {
        Self::new()
    }
}

/// Coverage information from TLC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageInfo {
    /// The module name.
    pub module: String,

    /// The action name.
    pub action: String,

    /// Number of times this action was taken.
    pub count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tlc_exit_status() {
        assert!(TlcExitStatus::from_exit_code(0).is_success());
        assert!(TlcExitStatus::from_exit_code(10).is_violation());
        assert!(TlcExitStatus::from_exit_code(11).is_violation());
        assert!(TlcExitStatus::from_exit_code(12).is_violation());
        assert!(TlcExitStatus::from_exit_code(13).is_violation());
        assert!(TlcExitStatus::from_exit_code(14).is_error());
        assert!(TlcExitStatus::from_exit_code(99).is_error());
    }

    #[test]
    fn test_validation_result() {
        let result = ValidationResult::valid();
        assert!(result.is_valid());
        assert!(!result.is_invalid());

        let violation = PropertyViolation::new(PropertyType::Invariant, "x must be positive")
            .with_name("PositiveInvariant")
            .with_step(5);
        let result = ValidationResult::invalid(vec![violation]);
        assert!(!result.is_valid());
        assert!(result.is_invalid());
        assert_eq!(result.violations.len(), 1);
    }

    #[test]
    fn test_property_violation() {
        let violation = PropertyViolation::new(PropertyType::Invariant, "x must be positive")
            .with_name("PositiveInvariant")
            .with_step(5)
            .with_action("Decrement");

        assert_eq!(violation.property_type, PropertyType::Invariant);
        assert_eq!(
            violation.property_name.as_deref(),
            Some("PositiveInvariant")
        );
        assert_eq!(violation.step, Some(5));
        assert_eq!(violation.action.as_deref(), Some("Decrement"));
    }

    #[test]
    fn test_counterexample_trace() {
        let state1 = CounterexampleState::new(1).with_variable("x", serde_json::json!(0));
        let state2 = CounterexampleState::new(2)
            .with_action("Increment")
            .with_variable("x", serde_json::json!(1));
        let state3 = CounterexampleState::new(3)
            .with_action("Decrement")
            .with_variable("x", serde_json::json!(0));

        let trace = CounterexampleTrace::new(vec![state1, state2, state3])
            .with_violated_property("PositiveInvariant")
            .with_description("x became zero");

        assert_eq!(trace.len(), 3);
        assert!(!trace.is_lasso);
        assert_eq!(
            trace.violated_property.as_deref(),
            Some("PositiveInvariant")
        );

        let initial = trace.initial_state().unwrap();
        assert_eq!(initial.step, 1);
        assert_eq!(initial.get_variable("x"), Some(&serde_json::json!(0)));
    }
}
