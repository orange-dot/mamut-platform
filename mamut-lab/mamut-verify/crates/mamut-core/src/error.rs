//! Error types for the verification harness.
//!
//! This module provides error types using `thiserror` for ergonomic error handling
//! across the verification framework.

use crate::history::RunId;
use crate::operation::OperationId;
use thiserror::Error;

/// Errors that can occur during history operations.
#[derive(Debug, Error)]
pub enum HistoryError {
    /// Failed to serialize an operation or history.
    #[error("serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Failed to read or write history data.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The requested run was not found.
    #[error("run not found: {0}")]
    RunNotFound(RunId),

    /// The history is corrupted or in an invalid state.
    #[error("corrupted history: {reason}")]
    Corrupted { reason: String },

    /// Operation was not found in the history.
    #[error("operation not found: {0}")]
    OperationNotFound(OperationId),

    /// Storage backend error.
    #[error("storage error: {0}")]
    Storage(String),

    /// The history store is closed.
    #[error("history store is closed")]
    StoreClosed,

    /// Concurrent modification detected.
    #[error("concurrent modification detected for run {run_id}")]
    ConcurrentModification { run_id: RunId },
}

/// Errors that can occur during operation execution.
#[derive(Debug, Error)]
pub enum OperationError {
    /// The operation timed out.
    #[error("operation {op_id} timed out after {elapsed_ms}ms")]
    Timeout { op_id: OperationId, elapsed_ms: u64 },

    /// The operation was cancelled.
    #[error("operation {0} was cancelled")]
    Cancelled(OperationId),

    /// The operation failed with a retryable error.
    #[error("retryable error in operation {op_id}: {message}")]
    Retryable { op_id: OperationId, message: String },

    /// The operation failed with a non-retryable error.
    #[error("operation {op_id} failed: {message}")]
    Failed { op_id: OperationId, message: String },

    /// Invalid operation state transition.
    #[error("invalid state transition for operation {op_id}: {details}")]
    InvalidStateTransition { op_id: OperationId, details: String },

    /// The operation arguments were invalid.
    #[error("invalid arguments for operation {op_id}: {details}")]
    InvalidArguments { op_id: OperationId, details: String },
}

/// Errors that can occur during verification.
#[derive(Debug, Error)]
pub enum VerificationError {
    /// A history error occurred.
    #[error("history error: {0}")]
    History(#[from] HistoryError),

    /// An operation error occurred.
    #[error("operation error: {0}")]
    Operation(#[from] OperationError),

    /// The verification model detected a violation.
    #[error("model violation: {model} - {description}")]
    ModelViolation { model: String, description: String },

    /// The history is not linearizable.
    #[error("linearizability violation: {0}")]
    LinearizabilityViolation(String),

    /// The history violates sequential consistency.
    #[error("sequential consistency violation: {0}")]
    SequentialConsistencyViolation(String),

    /// Configuration error.
    #[error("configuration error: {0}")]
    Configuration(String),
}

impl HistoryError {
    /// Creates a new corrupted history error.
    pub fn corrupted(reason: impl Into<String>) -> Self {
        Self::Corrupted {
            reason: reason.into(),
        }
    }

    /// Creates a new storage error.
    pub fn storage(message: impl Into<String>) -> Self {
        Self::Storage(message.into())
    }
}

impl OperationError {
    /// Creates a new timeout error.
    pub fn timeout(op_id: OperationId, elapsed_ms: u64) -> Self {
        Self::Timeout { op_id, elapsed_ms }
    }

    /// Creates a new retryable error.
    pub fn retryable(op_id: OperationId, message: impl Into<String>) -> Self {
        Self::Retryable {
            op_id,
            message: message.into(),
        }
    }

    /// Creates a new failed error.
    pub fn failed(op_id: OperationId, message: impl Into<String>) -> Self {
        Self::Failed {
            op_id,
            message: message.into(),
        }
    }

    /// Creates a new invalid state transition error.
    pub fn invalid_state_transition(op_id: OperationId, details: impl Into<String>) -> Self {
        Self::InvalidStateTransition {
            op_id,
            details: details.into(),
        }
    }

    /// Creates a new invalid arguments error.
    pub fn invalid_arguments(op_id: OperationId, details: impl Into<String>) -> Self {
        Self::InvalidArguments {
            op_id,
            details: details.into(),
        }
    }

    /// Returns true if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Retryable { .. } | Self::Timeout { .. })
    }
}

impl VerificationError {
    /// Creates a new model violation error.
    pub fn model_violation(model: impl Into<String>, description: impl Into<String>) -> Self {
        Self::ModelViolation {
            model: model.into(),
            description: description.into(),
        }
    }

    /// Creates a new linearizability violation error.
    pub fn linearizability_violation(description: impl Into<String>) -> Self {
        Self::LinearizabilityViolation(description.into())
    }

    /// Creates a new sequential consistency violation error.
    pub fn sequential_consistency_violation(description: impl Into<String>) -> Self {
        Self::SequentialConsistencyViolation(description.into())
    }

    /// Creates a new configuration error.
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_error_display() {
        let err = HistoryError::corrupted("invalid checksum");
        assert_eq!(err.to_string(), "corrupted history: invalid checksum");

        let err = HistoryError::storage("disk full");
        assert_eq!(err.to_string(), "storage error: disk full");

        let run_id = RunId::new();
        let err = HistoryError::RunNotFound(run_id);
        assert!(err.to_string().contains("run not found"));
    }

    #[test]
    fn test_operation_error_display() {
        let op_id = OperationId(42);

        let err = OperationError::timeout(op_id, 5000);
        assert_eq!(
            err.to_string(),
            "operation OperationId(42) timed out after 5000ms"
        );

        let err = OperationError::retryable(op_id, "connection reset");
        assert_eq!(
            err.to_string(),
            "retryable error in operation OperationId(42): connection reset"
        );

        let err = OperationError::failed(op_id, "invalid key");
        assert_eq!(
            err.to_string(),
            "operation OperationId(42) failed: invalid key"
        );
    }

    #[test]
    fn test_operation_error_is_retryable() {
        let op_id = OperationId(1);

        assert!(OperationError::timeout(op_id, 1000).is_retryable());
        assert!(OperationError::retryable(op_id, "temp").is_retryable());
        assert!(!OperationError::failed(op_id, "perm").is_retryable());
        assert!(!OperationError::Cancelled(op_id).is_retryable());
    }

    #[test]
    fn test_verification_error_display() {
        let err = VerificationError::model_violation("Register", "read returned stale value");
        assert_eq!(
            err.to_string(),
            "model violation: Register - read returned stale value"
        );

        let err = VerificationError::linearizability_violation("no valid linearization found");
        assert_eq!(
            err.to_string(),
            "linearizability violation: no valid linearization found"
        );

        let err = VerificationError::configuration("missing model specification");
        assert_eq!(
            err.to_string(),
            "configuration error: missing model specification"
        );
    }

    #[test]
    fn test_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let history_err: HistoryError = io_err.into();
        assert!(matches!(history_err, HistoryError::Io(_)));

        let history_err = HistoryError::storage("test");
        let verification_err: VerificationError = history_err.into();
        assert!(matches!(verification_err, VerificationError::History(_)));
    }
}
