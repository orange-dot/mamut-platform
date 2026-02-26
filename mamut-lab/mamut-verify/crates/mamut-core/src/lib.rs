//! Mamut Core - Core types for the verification harness.
//!
//! This crate provides the fundamental types used throughout the Mamut
//! verification framework, including:
//!
//! - [`node`]: Node and process identification types (`NodeId`, `ProcessId`)
//! - [`operation`]: Operation types for tracking distributed system actions
//! - [`history`]: History containers and storage traits for verification
//! - [`error`]: Error types for the verification framework
//! - [`ovc`]: Omniscient Verification Clock for multi-dimensional time tracking
//!
//! # Overview
//!
//! The Mamut verification harness records operations from distributed systems
//! and verifies that they satisfy correctness properties like linearizability.
//! This crate defines the core data types that flow through the system.
//!
//! # Omniscient Verification Clock (OVC)
//!
//! The OVC module provides a multi-dimensional timestamp that captures:
//!
//! - **Controller Time**: Ground truth from the test controller
//! - **Observed Time**: What system nodes believe the time to be
//! - **Logical Time**: Vector clocks for causality tracking
//! - **Uncertainty**: Bounds on actual event times
//! - **Fault Context**: Information about time-related faults
//!
//! # Example
//!
//! ```
//! use mamut_core::node::ProcessId;
//! use mamut_core::operation::{Operation, OperationId, OVC};
//! use mamut_core::history::{History, RunId};
//! use serde_json::json;
//!
//! // Create a new history for a test run
//! let mut history = History::<serde_json::Value>::new(RunId::new());
//!
//! // Record an operation
//! let op = Operation::new(
//!     OperationId(1),
//!     ProcessId(0),
//!     "put",
//!     json!({"key": "x", "value": 42}),
//!     OVC::new(100),
//! );
//! history.push(op);
//!
//! assert_eq!(history.len(), 1);
//! ```

pub mod error;
pub mod history;
pub mod node;
pub mod operation;
pub mod ovc;

// Re-export commonly used types at the crate root for convenience
pub use error::{HistoryError, OperationError, VerificationError};
pub use history::{History, HistoryMetadata, HistoryStore, InMemoryHistoryStore, RunId};
pub use node::{NodeId, ProcessId};
pub use operation::{OVC, Operation, OperationId, OperationPhase};

// Re-export OVC types at crate root for convenience
pub use ovc::{
    CompactVectorClock, ControllerTime, FaultSeverity, ObservedTime, OVCBatch,
    OVCCompact, TimeFaultContext, TimeFaultType, UncertaintyInterval, VectorClock,
};
// Note: ovc::NodeId is intentionally not re-exported to avoid conflict with node::NodeId
// Use ovc::NodeId explicitly when working with the OVC subsystem
