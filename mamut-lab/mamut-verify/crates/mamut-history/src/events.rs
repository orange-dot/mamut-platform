//! Event schema definitions for the history persistence layer.
//!
//! This module defines the strongly-typed event schemas for:
//! - Test run lifecycle events
//! - Operation invocation and completion events
//! - Fault injection events
//! - Verification check events

use chrono::{DateTime, Utc};
use mamut_core::{OperationId, ProcessId, RunId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ids::{CheckId, CheckpointId, FaultId};

/// Top-level history event that wraps all event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "category", content = "payload")]
pub enum HistoryEvent {
    /// Test run lifecycle events
    TestRun(TestRunEvent),
    /// Operation invocation events
    Operation(OperationEvent),
    /// Fault injection events
    Fault(FaultEvent),
    /// Verification check events
    Check(CheckEvent),
}

impl HistoryEvent {
    /// Returns the event type name for EventStoreDB
    pub fn event_type(&self) -> &'static str {
        match self {
            HistoryEvent::TestRun(e) => e.event_type(),
            HistoryEvent::Operation(e) => e.event_type(),
            HistoryEvent::Fault(e) => e.event_type(),
            HistoryEvent::Check(e) => e.event_type(),
        }
    }

    /// Returns the timestamp of the event
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            HistoryEvent::TestRun(e) => e.timestamp(),
            HistoryEvent::Operation(e) => e.timestamp(),
            HistoryEvent::Fault(e) => e.timestamp(),
            HistoryEvent::Check(e) => e.timestamp(),
        }
    }
}

/// Configuration for a test run
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TestConfig {
    /// Name of the test
    pub name: String,
    /// Description of what the test verifies
    pub description: Option<String>,
    /// Maximum duration for the test
    pub timeout_ms: Option<u64>,
    /// Seed for random number generation (for reproducibility)
    pub seed: Option<u64>,
    /// Number of concurrent processes/threads
    pub concurrency: Option<u32>,
    /// Custom configuration parameters
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Test execution phase
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestPhase {
    /// Initial setup phase
    Setup,
    /// Main execution phase
    Running,
    /// Fault injection phase
    FaultInjection,
    /// Recovery verification phase
    Recovery,
    /// Cleanup phase
    Teardown,
    /// Verification phase
    Verification,
}

/// Result of a test run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestResult {
    /// Test passed all checks
    Passed,
    /// Test failed with violations
    Failed { violations: Vec<String> },
    /// Test was aborted
    Aborted { reason: String },
    /// Test timed out
    TimedOut,
    /// Test encountered an error
    Error { message: String },
}

/// Events related to test run lifecycle
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TestRunEvent {
    /// Test execution started
    TestStarted {
        run_id: RunId,
        timestamp: DateTime<Utc>,
        config: TestConfig,
    },

    /// Test configuration updated
    TestConfigured {
        run_id: RunId,
        timestamp: DateTime<Utc>,
        config: TestConfig,
    },

    /// Test phase transition
    PhaseTransition {
        run_id: RunId,
        timestamp: DateTime<Utc>,
        from_phase: Option<TestPhase>,
        to_phase: TestPhase,
    },

    /// Test execution completed
    TestCompleted {
        run_id: RunId,
        timestamp: DateTime<Utc>,
        result: TestResult,
        duration_ms: u64,
        total_operations: u64,
        total_faults: u64,
    },

    /// Checkpoint created during test
    CheckpointCreated {
        run_id: RunId,
        timestamp: DateTime<Utc>,
        checkpoint_id: CheckpointId,
        stream_positions: HashMap<String, u64>,
        metadata: HashMap<String, serde_json::Value>,
    },
}

impl TestRunEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            TestRunEvent::TestStarted { .. } => "TestStarted",
            TestRunEvent::TestConfigured { .. } => "TestConfigured",
            TestRunEvent::PhaseTransition { .. } => "PhaseTransition",
            TestRunEvent::TestCompleted { .. } => "TestCompleted",
            TestRunEvent::CheckpointCreated { .. } => "CheckpointCreated",
        }
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            TestRunEvent::TestStarted { timestamp, .. } => *timestamp,
            TestRunEvent::TestConfigured { timestamp, .. } => *timestamp,
            TestRunEvent::PhaseTransition { timestamp, .. } => *timestamp,
            TestRunEvent::TestCompleted { timestamp, .. } => *timestamp,
            TestRunEvent::CheckpointCreated { timestamp, .. } => *timestamp,
        }
    }

    pub fn run_id(&self) -> &RunId {
        match self {
            TestRunEvent::TestStarted { run_id, .. } => run_id,
            TestRunEvent::TestConfigured { run_id, .. } => run_id,
            TestRunEvent::PhaseTransition { run_id, .. } => run_id,
            TestRunEvent::TestCompleted { run_id, .. } => run_id,
            TestRunEvent::CheckpointCreated { run_id, .. } => run_id,
        }
    }
}

/// Type of operation being performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationType {
    /// Operation name (e.g., "read", "write", "cas")
    pub name: String,
    /// Target resource (e.g., "register", "set", "queue")
    pub target: String,
}

/// Result of an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum OperationResult {
    /// Operation succeeded with a value
    Ok { value: serde_json::Value },
    /// Operation returned an error
    Err { error: String },
    /// Operation result is unknown (e.g., due to timeout)
    Unknown,
}

/// Events related to operation invocations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OperationEvent {
    /// Operation was invoked
    Invoked {
        operation_id: OperationId,
        process_id: ProcessId,
        timestamp: DateTime<Utc>,
        operation_type: OperationType,
        arguments: Vec<serde_json::Value>,
    },

    /// Operation returned successfully
    Returned {
        operation_id: OperationId,
        process_id: ProcessId,
        timestamp: DateTime<Utc>,
        result: OperationResult,
        duration_ns: u64,
    },

    /// Operation failed with an error
    Failed {
        operation_id: OperationId,
        process_id: ProcessId,
        timestamp: DateTime<Utc>,
        error: String,
        retryable: bool,
    },

    /// Operation timed out
    TimedOut {
        operation_id: OperationId,
        process_id: ProcessId,
        timestamp: DateTime<Utc>,
        timeout_ms: u64,
    },
}

impl OperationEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            OperationEvent::Invoked { .. } => "OperationInvoked",
            OperationEvent::Returned { .. } => "OperationReturned",
            OperationEvent::Failed { .. } => "OperationFailed",
            OperationEvent::TimedOut { .. } => "OperationTimedOut",
        }
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            OperationEvent::Invoked { timestamp, .. } => *timestamp,
            OperationEvent::Returned { timestamp, .. } => *timestamp,
            OperationEvent::Failed { timestamp, .. } => *timestamp,
            OperationEvent::TimedOut { timestamp, .. } => *timestamp,
        }
    }

    pub fn operation_id(&self) -> OperationId {
        match self {
            OperationEvent::Invoked { operation_id, .. } => *operation_id,
            OperationEvent::Returned { operation_id, .. } => *operation_id,
            OperationEvent::Failed { operation_id, .. } => *operation_id,
            OperationEvent::TimedOut { operation_id, .. } => *operation_id,
        }
    }

    pub fn process_id(&self) -> ProcessId {
        match self {
            OperationEvent::Invoked { process_id, .. } => *process_id,
            OperationEvent::Returned { process_id, .. } => *process_id,
            OperationEvent::Failed { process_id, .. } => *process_id,
            OperationEvent::TimedOut { process_id, .. } => *process_id,
        }
    }
}

/// Type of fault to inject
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum FaultType {
    /// Network partition between processes
    NetworkPartition { affected_processes: Vec<ProcessId> },
    /// Network delay/latency
    NetworkDelay {
        delay_ms: u64,
        jitter_ms: Option<u64>,
    },
    /// Process crash/kill
    ProcessCrash { process_id: ProcessId },
    /// Process pause (SIGSTOP)
    ProcessPause { process_id: ProcessId },
    /// Clock skew
    ClockSkew { skew_ms: i64 },
    /// Disk failure
    DiskFailure { failure_type: String },
    /// Custom fault type
    Custom {
        name: String,
        parameters: HashMap<String, serde_json::Value>,
    },
}

/// Context for fault injection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultContext {
    /// Unique identifier for this fault context
    pub context_id: String,
    /// Description of the fault scenario
    pub description: Option<String>,
    /// Active faults in this context
    pub active_faults: Vec<FaultId>,
    /// Metadata for the context
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Events related to fault injection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FaultEvent {
    /// Fault was injected
    Injected {
        fault_id: FaultId,
        timestamp: DateTime<Utc>,
        fault_type: FaultType,
        duration_ms: Option<u64>,
        metadata: HashMap<String, serde_json::Value>,
    },

    /// Fault was healed/removed
    Healed {
        fault_id: FaultId,
        timestamp: DateTime<Utc>,
        actual_duration_ms: u64,
    },

    /// Fault context was created
    ContextCreated {
        timestamp: DateTime<Utc>,
        context: FaultContext,
    },
}

impl FaultEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            FaultEvent::Injected { .. } => "FaultInjected",
            FaultEvent::Healed { .. } => "FaultHealed",
            FaultEvent::ContextCreated { .. } => "FaultContextCreated",
        }
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            FaultEvent::Injected { timestamp, .. } => *timestamp,
            FaultEvent::Healed { timestamp, .. } => *timestamp,
            FaultEvent::ContextCreated { timestamp, .. } => *timestamp,
        }
    }
}

/// Type of consistency check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckType {
    /// Linearizability check
    Linearizability,
    /// Sequential consistency check
    SequentialConsistency,
    /// Serializability check
    Serializability,
    /// Causal consistency check
    CausalConsistency,
    /// Custom consistency model
    Custom(String),
}

/// A violation detected during checking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    /// Type of violation
    pub violation_type: String,
    /// Human-readable description
    pub description: String,
    /// Operations involved in the violation
    pub involved_operations: Vec<OperationId>,
    /// Expected behavior
    pub expected: Option<String>,
    /// Actual behavior observed
    pub actual: Option<String>,
    /// Additional context
    pub context: HashMap<String, serde_json::Value>,
}

/// Minimal history that demonstrates a violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimalHistory {
    /// Operations in the minimal history
    pub operations: Vec<OperationId>,
    /// Total number of operations before minimization
    pub original_size: usize,
    /// Number of operations after minimization
    pub minimized_size: usize,
    /// The violation this history demonstrates
    pub violation: Violation,
}

/// Result of a consistency check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckResult {
    /// Check passed
    Passed { operations_checked: u64 },
    /// Check failed with violations
    Failed {
        operations_checked: u64,
        violation_count: u64,
    },
    /// Check was inconclusive
    Inconclusive { reason: String },
}

/// Events related to verification checks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CheckEvent {
    /// Check started
    CheckStarted {
        check_id: CheckId,
        timestamp: DateTime<Utc>,
        check_type: CheckType,
        operation_count: u64,
    },

    /// Check completed
    CheckCompleted {
        check_id: CheckId,
        timestamp: DateTime<Utc>,
        result: CheckResult,
        duration_ms: u64,
    },

    /// Violation detected during check
    ViolationDetected {
        check_id: CheckId,
        timestamp: DateTime<Utc>,
        violation: Violation,
    },

    /// Minimal history extracted for a violation
    MinimalHistoryExtracted {
        check_id: CheckId,
        timestamp: DateTime<Utc>,
        minimal_history: MinimalHistory,
    },
}

impl CheckEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            CheckEvent::CheckStarted { .. } => "CheckStarted",
            CheckEvent::CheckCompleted { .. } => "CheckCompleted",
            CheckEvent::ViolationDetected { .. } => "ViolationDetected",
            CheckEvent::MinimalHistoryExtracted { .. } => "MinimalHistoryExtracted",
        }
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            CheckEvent::CheckStarted { timestamp, .. } => *timestamp,
            CheckEvent::CheckCompleted { timestamp, .. } => *timestamp,
            CheckEvent::ViolationDetected { timestamp, .. } => *timestamp,
            CheckEvent::MinimalHistoryExtracted { timestamp, .. } => *timestamp,
        }
    }

    pub fn check_id(&self) -> &CheckId {
        match self {
            CheckEvent::CheckStarted { check_id, .. } => check_id,
            CheckEvent::CheckCompleted { check_id, .. } => check_id,
            CheckEvent::ViolationDetected { check_id, .. } => check_id,
            CheckEvent::MinimalHistoryExtracted { check_id, .. } => check_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_event_serialization() {
        let event = HistoryEvent::TestRun(TestRunEvent::TestStarted {
            run_id: RunId::new(),
            timestamp: Utc::now(),
            config: TestConfig {
                name: "linearizability-test".to_string(),
                ..Default::default()
            },
        });

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: HistoryEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event.event_type(), deserialized.event_type());
    }

    #[test]
    fn test_operation_event_serialization() {
        let event = OperationEvent::Invoked {
            operation_id: OperationId::new(1),
            process_id: ProcessId(1),
            timestamp: Utc::now(),
            operation_type: OperationType {
                name: "write".to_string(),
                target: "register".to_string(),
            },
            arguments: vec![serde_json::json!(42)],
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: OperationEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event.event_type(), deserialized.event_type());
    }

    #[test]
    fn test_fault_event_serialization() {
        let event = FaultEvent::Injected {
            fault_id: FaultId::new(),
            timestamp: Utc::now(),
            fault_type: FaultType::NetworkPartition {
                affected_processes: vec![ProcessId(1), ProcessId(2)],
            },
            duration_ms: Some(5000),
            metadata: HashMap::new(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: FaultEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event.event_type(), deserialized.event_type());
    }

    #[test]
    fn test_check_event_serialization() {
        let event = CheckEvent::CheckStarted {
            check_id: CheckId::new(),
            timestamp: Utc::now(),
            check_type: CheckType::Linearizability,
            operation_count: 1000,
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: CheckEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event.event_type(), deserialized.event_type());
    }
}
