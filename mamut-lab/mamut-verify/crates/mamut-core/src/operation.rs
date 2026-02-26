//! Operation types for the verification harness.
//!
//! This module defines the core operation types used to represent actions
//! performed during distributed system testing. Operations form the basis
//! of the history that is later verified for correctness properties.

use crate::node::ProcessId;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for an operation within a test run.
///
/// Operation IDs are assigned sequentially and globally unique within a single
/// test run. They are used to track operations through their lifecycle and
/// to correlate invocations with their completions.
///
/// # Examples
///
/// ```
/// use mamut_core::operation::OperationId;
///
/// let op_id = OperationId(1);
/// assert_eq!(op_id.0, 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct OperationId(pub u64);

impl OperationId {
    /// Creates a new OperationId with the given value.
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the inner value of the OperationId.
    #[inline]
    pub const fn inner(self) -> u64 {
        self.0
    }
}

impl fmt::Display for OperationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OperationId({})", self.0)
    }
}

impl From<u64> for OperationId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<OperationId> for u64 {
    fn from(id: OperationId) -> Self {
        id.0
    }
}

/// Observable Vector Clock (OVC) for tracking causal ordering.
///
/// The OVC captures the logical time at which an operation was observed,
/// enabling the verification algorithm to determine happens-before relationships
/// and potential concurrent operations.
///
/// # Examples
///
/// ```
/// use mamut_core::operation::OVC;
///
/// let ovc = OVC::new(100);
/// assert_eq!(ovc.timestamp(), 100);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct OVC {
    /// The logical timestamp component.
    timestamp: u64,
    /// The node-local counter for disambiguation.
    counter: u32,
}

impl OVC {
    /// Creates a new OVC with the given timestamp and zero counter.
    pub const fn new(timestamp: u64) -> Self {
        Self {
            timestamp,
            counter: 0,
        }
    }

    /// Creates a new OVC with the given timestamp and counter.
    pub const fn with_counter(timestamp: u64, counter: u32) -> Self {
        Self { timestamp, counter }
    }

    /// Returns the timestamp component.
    #[inline]
    pub const fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Returns the counter component.
    #[inline]
    pub const fn counter(&self) -> u32 {
        self.counter
    }

    /// Creates a new OVC that is causally after this one.
    pub const fn increment(&self) -> Self {
        Self {
            timestamp: self.timestamp + 1,
            counter: 0,
        }
    }

    /// Creates a new OVC with an incremented counter (same timestamp).
    pub const fn tick(&self) -> Self {
        Self {
            timestamp: self.timestamp,
            counter: self.counter + 1,
        }
    }

    /// Merges this OVC with another, taking the maximum of each component.
    pub fn merge(&self, other: &Self) -> Self {
        if self.timestamp > other.timestamp {
            *self
        } else if other.timestamp > self.timestamp {
            *other
        } else {
            // Same timestamp, take max counter
            Self {
                timestamp: self.timestamp,
                counter: self.counter.max(other.counter),
            }
        }
    }

    /// Returns true if this OVC is causally before the other.
    pub fn happens_before(&self, other: &Self) -> bool {
        self.timestamp < other.timestamp
            || (self.timestamp == other.timestamp && self.counter < other.counter)
    }

    /// Returns true if these OVCs are concurrent (neither happens-before the other).
    pub fn concurrent_with(&self, other: &Self) -> bool {
        !self.happens_before(other) && !other.happens_before(self) && self != other
    }
}

impl Default for OVC {
    fn default() -> Self {
        Self::new(0)
    }
}

impl fmt::Display for OVC {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OVC({}.{})", self.timestamp, self.counter)
    }
}

/// The phase/state of an operation in its lifecycle.
///
/// Operations progress through phases as they are executed:
/// 1. `Invoked` - The operation has been called but not yet completed
/// 2. One of the terminal states:
///    - `Completed` - The operation finished successfully with a result
///    - `Failed` - The operation encountered an error
///    - `TimedOut` - The operation exceeded its deadline
/// 3. `Info` - An informational event (not a state transition)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OperationPhase {
    /// The operation has been invoked but not yet completed.
    Invoked,

    /// The operation completed successfully with a result.
    Completed {
        /// The result value of the operation.
        result: serde_json::Value,
    },

    /// The operation failed with an error.
    Failed {
        /// A description of the error.
        error: String,
        /// Whether the operation can be retried.
        retryable: bool,
    },

    /// The operation timed out before completing.
    TimedOut {
        /// The deadline (in clock ticks) that was exceeded.
        deadline_ct: u64,
    },

    /// An informational message associated with the operation.
    Info {
        /// The informational message.
        message: String,
    },
}

impl OperationPhase {
    /// Returns true if this phase represents a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed { .. } | Self::Failed { .. } | Self::TimedOut { .. }
        )
    }

    /// Returns true if the operation completed successfully.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Completed { .. })
    }

    /// Returns true if the operation failed (either error or timeout).
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failed { .. } | Self::TimedOut { .. })
    }

    /// Returns true if the operation can be retried.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Failed { retryable, .. } => *retryable,
            Self::TimedOut { .. } => true,
            _ => false,
        }
    }

    /// Creates a completed phase with the given result.
    pub fn completed(result: serde_json::Value) -> Self {
        Self::Completed { result }
    }

    /// Creates a failed phase with the given error.
    pub fn failed(error: impl Into<String>, retryable: bool) -> Self {
        Self::Failed {
            error: error.into(),
            retryable,
        }
    }

    /// Creates a timed out phase with the given deadline.
    pub fn timed_out(deadline_ct: u64) -> Self {
        Self::TimedOut { deadline_ct }
    }

    /// Creates an info phase with the given message.
    pub fn info(message: impl Into<String>) -> Self {
        Self::Info {
            message: message.into(),
        }
    }
}

impl fmt::Display for OperationPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invoked => write!(f, "Invoked"),
            Self::Completed { result } => write!(f, "Completed({})", result),
            Self::Failed { error, retryable } => {
                write!(f, "Failed({}, retryable={})", error, retryable)
            }
            Self::TimedOut { deadline_ct } => write!(f, "TimedOut(deadline={})", deadline_ct),
            Self::Info { message } => write!(f, "Info({})", message),
        }
    }
}

/// An operation in the distributed system being verified.
///
/// Operations represent individual actions taken by processes during testing.
/// Each operation has a unique ID, belongs to a process, calls a specific
/// function with arguments, and progresses through phases as it executes.
///
/// # Type Parameters
///
/// * `V` - The type of the operation arguments. This is typically a
///         serializable type that captures the input to the operation.
///
/// # Examples
///
/// ```
/// use mamut_core::operation::{Operation, OperationId, OperationPhase, OVC};
/// use mamut_core::node::ProcessId;
/// use serde_json::json;
///
/// let op = Operation {
///     id: OperationId(1),
///     process: ProcessId(0),
///     function: "put".to_string(),
///     args: json!({"key": "foo", "value": "bar"}),
///     phase: OperationPhase::Invoked,
///     ovc: OVC::new(100),
/// };
///
/// assert_eq!(op.function, "put");
/// assert!(!op.phase.is_terminal());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation<V> {
    /// The unique identifier for this operation.
    pub id: OperationId,

    /// The process that issued this operation.
    pub process: ProcessId,

    /// The name of the function being called.
    pub function: String,

    /// The arguments passed to the function.
    pub args: V,

    /// The current phase of the operation.
    pub phase: OperationPhase,

    /// The observable vector clock at the time of this event.
    pub ovc: OVC,
}

impl<V> Operation<V> {
    /// Creates a new operation in the Invoked phase.
    pub fn new(
        id: OperationId,
        process: ProcessId,
        function: impl Into<String>,
        args: V,
        ovc: OVC,
    ) -> Self {
        Self {
            id,
            process,
            function: function.into(),
            args,
            phase: OperationPhase::Invoked,
            ovc,
        }
    }

    /// Returns true if this operation has completed (terminal state).
    pub fn is_complete(&self) -> bool {
        self.phase.is_terminal()
    }

    /// Returns true if this operation completed successfully.
    pub fn is_success(&self) -> bool {
        self.phase.is_success()
    }

    /// Creates a new operation with the phase updated to Completed.
    pub fn with_completion(self, result: serde_json::Value, ovc: OVC) -> Self {
        Self {
            phase: OperationPhase::completed(result),
            ovc,
            ..self
        }
    }

    /// Creates a new operation with the phase updated to Failed.
    pub fn with_failure(self, error: impl Into<String>, retryable: bool, ovc: OVC) -> Self {
        Self {
            phase: OperationPhase::failed(error, retryable),
            ovc,
            ..self
        }
    }

    /// Creates a new operation with the phase updated to TimedOut.
    pub fn with_timeout(self, deadline_ct: u64, ovc: OVC) -> Self {
        Self {
            phase: OperationPhase::timed_out(deadline_ct),
            ovc,
            ..self
        }
    }

    /// Maps the arguments to a different type.
    pub fn map_args<U, F>(self, f: F) -> Operation<U>
    where
        F: FnOnce(V) -> U,
    {
        Operation {
            id: self.id,
            process: self.process,
            function: self.function,
            args: f(self.args),
            phase: self.phase,
            ovc: self.ovc,
        }
    }
}

impl<V> Operation<V>
where
    V: Clone,
{
    /// Creates a copy of this operation with a new phase.
    pub fn with_phase(&self, phase: OperationPhase, ovc: OVC) -> Self {
        Self {
            id: self.id,
            process: self.process,
            function: self.function.clone(),
            args: self.args.clone(),
            phase,
            ovc,
        }
    }
}

impl<V> fmt::Display for Operation<V>
where
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Op[{} @ {}] {}({:?}) -> {}",
            self.id, self.process, self.function, self.args, self.phase
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_operation_id_creation() {
        let id = OperationId::new(42);
        assert_eq!(id.inner(), 42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn test_operation_id_display() {
        let id = OperationId(123);
        assert_eq!(format!("{}", id), "OperationId(123)");
    }

    #[test]
    fn test_operation_id_conversion() {
        let id: OperationId = 100u64.into();
        assert_eq!(id.0, 100);

        let value: u64 = id.into();
        assert_eq!(value, 100);
    }

    #[test]
    fn test_ovc_creation() {
        let ovc = OVC::new(100);
        assert_eq!(ovc.timestamp(), 100);
        assert_eq!(ovc.counter(), 0);

        let ovc = OVC::with_counter(100, 5);
        assert_eq!(ovc.timestamp(), 100);
        assert_eq!(ovc.counter(), 5);
    }

    #[test]
    fn test_ovc_increment() {
        let ovc = OVC::with_counter(100, 5);
        let next = ovc.increment();
        assert_eq!(next.timestamp(), 101);
        assert_eq!(next.counter(), 0);
    }

    #[test]
    fn test_ovc_tick() {
        let ovc = OVC::with_counter(100, 5);
        let next = ovc.tick();
        assert_eq!(next.timestamp(), 100);
        assert_eq!(next.counter(), 6);
    }

    #[test]
    fn test_ovc_merge() {
        let a = OVC::with_counter(100, 5);
        let b = OVC::with_counter(100, 3);
        let merged = a.merge(&b);
        assert_eq!(merged.timestamp(), 100);
        assert_eq!(merged.counter(), 5);

        let c = OVC::with_counter(99, 10);
        let merged = a.merge(&c);
        assert_eq!(merged.timestamp(), 100);
        assert_eq!(merged.counter(), 5);

        let d = OVC::with_counter(101, 1);
        let merged = a.merge(&d);
        assert_eq!(merged.timestamp(), 101);
        assert_eq!(merged.counter(), 1);
    }

    #[test]
    fn test_ovc_happens_before() {
        let a = OVC::with_counter(100, 5);
        let b = OVC::with_counter(101, 0);
        assert!(a.happens_before(&b));
        assert!(!b.happens_before(&a));

        let c = OVC::with_counter(100, 6);
        assert!(a.happens_before(&c));
        assert!(!c.happens_before(&a));
    }

    #[test]
    fn test_ovc_concurrent() {
        let a = OVC::with_counter(100, 5);
        let b = OVC::with_counter(100, 5);
        assert!(!a.concurrent_with(&b)); // Equal, not concurrent

        // Note: In a real distributed system, concurrent events would have
        // incomparable vector clocks. With our simplified OVC, we can't
        // truly represent concurrent events, but we test the logic.
    }

    #[test]
    fn test_ovc_display() {
        let ovc = OVC::with_counter(100, 5);
        assert_eq!(format!("{}", ovc), "OVC(100.5)");
    }

    #[test]
    fn test_operation_phase_terminal() {
        assert!(!OperationPhase::Invoked.is_terminal());
        assert!(OperationPhase::completed(json!(null)).is_terminal());
        assert!(OperationPhase::failed("error", true).is_terminal());
        assert!(OperationPhase::timed_out(100).is_terminal());
        assert!(!OperationPhase::info("test").is_terminal());
    }

    #[test]
    fn test_operation_phase_success() {
        assert!(!OperationPhase::Invoked.is_success());
        assert!(OperationPhase::completed(json!(null)).is_success());
        assert!(!OperationPhase::failed("error", true).is_success());
        assert!(!OperationPhase::timed_out(100).is_success());
    }

    #[test]
    fn test_operation_phase_retryable() {
        assert!(!OperationPhase::Invoked.is_retryable());
        assert!(!OperationPhase::completed(json!(null)).is_retryable());
        assert!(OperationPhase::failed("error", true).is_retryable());
        assert!(!OperationPhase::failed("error", false).is_retryable());
        assert!(OperationPhase::timed_out(100).is_retryable());
    }

    #[test]
    fn test_operation_phase_display() {
        assert_eq!(format!("{}", OperationPhase::Invoked), "Invoked");
        assert_eq!(
            format!("{}", OperationPhase::completed(json!(42))),
            "Completed(42)"
        );
        assert_eq!(
            format!("{}", OperationPhase::failed("oops", true)),
            "Failed(oops, retryable=true)"
        );
        assert_eq!(
            format!("{}", OperationPhase::timed_out(100)),
            "TimedOut(deadline=100)"
        );
        assert_eq!(format!("{}", OperationPhase::info("hello")), "Info(hello)");
    }

    #[test]
    fn test_operation_creation() {
        let op = Operation::new(
            OperationId(1),
            ProcessId(0),
            "get",
            json!({"key": "foo"}),
            OVC::new(100),
        );

        assert_eq!(op.id, OperationId(1));
        assert_eq!(op.process, ProcessId(0));
        assert_eq!(op.function, "get");
        assert!(!op.is_complete());
        assert!(!op.is_success());
    }

    #[test]
    fn test_operation_with_completion() {
        let op = Operation::new(
            OperationId(1),
            ProcessId(0),
            "get",
            json!({"key": "foo"}),
            OVC::new(100),
        );

        let completed = op.with_completion(json!("bar"), OVC::new(101));
        assert!(completed.is_complete());
        assert!(completed.is_success());
        assert_eq!(completed.ovc.timestamp(), 101);
    }

    #[test]
    fn test_operation_with_failure() {
        let op = Operation::new(
            OperationId(1),
            ProcessId(0),
            "put",
            json!({"key": "foo", "value": "bar"}),
            OVC::new(100),
        );

        let failed = op.with_failure("network error", true, OVC::new(101));
        assert!(failed.is_complete());
        assert!(!failed.is_success());
        assert!(failed.phase.is_retryable());
    }

    #[test]
    fn test_operation_with_timeout() {
        let op = Operation::new(
            OperationId(1),
            ProcessId(0),
            "cas",
            json!({}),
            OVC::new(100),
        );

        let timed_out = op.with_timeout(150, OVC::new(151));
        assert!(timed_out.is_complete());
        assert!(!timed_out.is_success());
        assert!(timed_out.phase.is_retryable());
    }

    #[test]
    fn test_operation_map_args() {
        let op = Operation::new(
            OperationId(1),
            ProcessId(0),
            "get",
            "original",
            OVC::new(100),
        );

        let mapped = op.map_args(|s| s.to_uppercase());
        assert_eq!(mapped.args, "ORIGINAL");
        assert_eq!(mapped.id, OperationId(1));
    }

    #[test]
    fn test_operation_with_phase() {
        let op = Operation::new(
            OperationId(1),
            ProcessId(0),
            "get",
            json!({"key": "foo"}),
            OVC::new(100),
        );

        let completed = op.with_phase(OperationPhase::completed(json!("bar")), OVC::new(101));
        assert!(completed.is_complete());
        assert_eq!(completed.args, json!({"key": "foo"}));
    }

    #[test]
    fn test_operation_display() {
        let op = Operation::new(
            OperationId(1),
            ProcessId(0),
            "get",
            "key1",
            OVC::new(100),
        );

        let display = format!("{}", op);
        assert!(display.contains("Op["));
        assert!(display.contains("get"));
        assert!(display.contains("key1"));
    }

    #[test]
    fn test_operation_serialization() {
        let op = Operation::new(
            OperationId(1),
            ProcessId(0),
            "get",
            json!({"key": "foo"}),
            OVC::new(100),
        );

        let json = serde_json::to_string(&op).unwrap();
        let deserialized: Operation<serde_json::Value> = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, op.id);
        assert_eq!(deserialized.process, op.process);
        assert_eq!(deserialized.function, op.function);
        assert_eq!(deserialized.args, op.args);
    }

    #[test]
    fn test_ovc_serialization() {
        let ovc = OVC::with_counter(100, 5);
        let json = serde_json::to_string(&ovc).unwrap();
        let deserialized: OVC = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ovc);
    }
}
