//! Transport-specific error types.
//!
//! This module defines errors that can occur during gRPC communication
//! between the controller and agents.

use thiserror::Error;
use tonic::Status;

/// Errors that can occur during transport operations.
#[derive(Debug, Error)]
pub enum TransportError {
    /// Failed to connect to the remote endpoint.
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    /// The connection was lost.
    #[error("connection lost: {0}")]
    ConnectionLost(String),

    /// Request timed out.
    #[error("request timed out after {elapsed_ms}ms")]
    Timeout { elapsed_ms: u64 },

    /// The remote endpoint returned an error.
    #[error("remote error: {code} - {message}")]
    RemoteError { code: String, message: String },

    /// Failed to serialize or deserialize a message.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Invalid request or response.
    #[error("invalid message: {0}")]
    InvalidMessage(String),

    /// The server is shutting down.
    #[error("server is shutting down")]
    ShuttingDown,

    /// The operation was cancelled.
    #[error("operation cancelled")]
    Cancelled,

    /// Internal error.
    #[error("internal error: {0}")]
    Internal(String),

    /// gRPC transport error.
    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::transport::Error),

    /// gRPC status error.
    #[error("gRPC status: {0}")]
    Status(#[from] Status),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl TransportError {
    /// Creates a new connection failed error.
    pub fn connection_failed(message: impl Into<String>) -> Self {
        Self::ConnectionFailed(message.into())
    }

    /// Creates a new connection lost error.
    pub fn connection_lost(message: impl Into<String>) -> Self {
        Self::ConnectionLost(message.into())
    }

    /// Creates a new timeout error.
    pub fn timeout(elapsed_ms: u64) -> Self {
        Self::Timeout { elapsed_ms }
    }

    /// Creates a new remote error.
    pub fn remote_error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::RemoteError {
            code: code.into(),
            message: message.into(),
        }
    }

    /// Creates a new serialization error.
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization(message.into())
    }

    /// Creates a new invalid message error.
    pub fn invalid_message(message: impl Into<String>) -> Self {
        Self::InvalidMessage(message.into())
    }

    /// Creates a new internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    /// Returns true if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Timeout { .. }
                | Self::ConnectionLost(_)
                | Self::ConnectionFailed(_)
                | Self::Cancelled
        )
    }

    /// Returns true if this error indicates the server is unavailable.
    pub fn is_unavailable(&self) -> bool {
        match self {
            Self::Status(status) => status.code() == tonic::Code::Unavailable,
            Self::ConnectionFailed(_) | Self::ConnectionLost(_) | Self::ShuttingDown => true,
            _ => false,
        }
    }
}

impl From<TransportError> for Status {
    fn from(err: TransportError) -> Self {
        match err {
            TransportError::Timeout { elapsed_ms } => {
                Status::deadline_exceeded(format!("timed out after {}ms", elapsed_ms))
            }
            TransportError::ConnectionFailed(msg) | TransportError::ConnectionLost(msg) => {
                Status::unavailable(msg)
            }
            TransportError::Serialization(msg) => {
                Status::invalid_argument(format!("serialization error: {}", msg))
            }
            TransportError::InvalidMessage(msg) => Status::invalid_argument(msg),
            TransportError::ShuttingDown => Status::unavailable("server is shutting down"),
            TransportError::Cancelled => Status::cancelled("operation cancelled"),
            TransportError::Internal(msg) => Status::internal(msg),
            TransportError::RemoteError { code, message } => {
                Status::unknown(format!("{}: {}", code, message))
            }
            TransportError::Grpc(e) => Status::unavailable(e.to_string()),
            TransportError::Status(s) => s,
            TransportError::Io(e) => Status::internal(e.to_string()),
        }
    }
}

/// Result type alias for transport operations.
pub type TransportResult<T> = Result<T, TransportError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = TransportError::timeout(5000);
        assert_eq!(err.to_string(), "request timed out after 5000ms");

        let err = TransportError::connection_failed("host unreachable");
        assert_eq!(err.to_string(), "connection failed: host unreachable");

        let err = TransportError::remote_error("NOT_FOUND", "key not found");
        assert_eq!(err.to_string(), "remote error: NOT_FOUND - key not found");
    }

    #[test]
    fn test_error_retryable() {
        assert!(TransportError::timeout(1000).is_retryable());
        assert!(TransportError::connection_lost("reset").is_retryable());
        assert!(TransportError::connection_failed("refused").is_retryable());
        assert!(TransportError::Cancelled.is_retryable());

        assert!(!TransportError::internal("bug").is_retryable());
        assert!(!TransportError::invalid_message("bad").is_retryable());
    }

    #[test]
    fn test_error_to_status() {
        let err = TransportError::timeout(5000);
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::DeadlineExceeded);

        let err = TransportError::ShuttingDown;
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::Unavailable);

        let err = TransportError::invalid_message("bad format");
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
    }
}
