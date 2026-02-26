//! Error types for the Nemesis fault injection framework.

use thiserror::Error;

use crate::capability::LinuxCapability;

/// Result type alias for Nemesis operations.
pub type Result<T> = std::result::Result<T, NemesisError>;

/// Errors that can occur during fault injection operations.
#[derive(Debug, Error)]
pub enum NemesisError {
    /// A required Linux capability is not available.
    #[error("Missing required capability: {0}")]
    MissingCapability(LinuxCapability),

    /// The fault injection target was not found.
    #[error("Target not found: {0}")]
    TargetNotFound(String),

    /// A command execution failed.
    #[error("Command failed: {command} - {reason}")]
    CommandFailed {
        command: String,
        reason: String,
    },

    /// The fault is already active.
    #[error("Fault already active: {0}")]
    AlreadyActive(String),

    /// The fault is not active.
    #[error("Fault not active: {0}")]
    NotActive(String),

    /// Recovery from a fault failed.
    #[error("Recovery failed: {0}")]
    RecoveryFailed(String),

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// The operation timed out.
    #[error("Operation timed out after {0:?}")]
    Timeout(std::time::Duration),

    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The fault handle is invalid or expired.
    #[error("Invalid fault handle: {0}")]
    InvalidHandle(String),

    /// The scheduler encountered an error.
    #[error("Scheduler error: {0}")]
    SchedulerError(String),

    /// A network-related fault error.
    #[error("Network fault error: {0}")]
    NetworkFault(String),

    /// A process-related fault error.
    #[error("Process fault error: {0}")]
    ProcessFault(String),

    /// A clock-related fault error.
    #[error("Clock fault error: {0}")]
    ClockFault(String),

    /// The operation is not supported on this platform.
    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(String),

    /// An internal error occurred.
    #[error("Internal error: {0}")]
    Internal(String),
}

impl NemesisError {
    /// Creates a new command failed error.
    pub fn command_failed(command: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::CommandFailed {
            command: command.into(),
            reason: reason.into(),
        }
    }

    /// Checks if this error is recoverable.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            NemesisError::Timeout(_)
                | NemesisError::TargetNotFound(_)
                | NemesisError::NotActive(_)
        )
    }

    /// Checks if this error is due to missing permissions.
    pub fn is_permission_error(&self) -> bool {
        matches!(self, NemesisError::MissingCapability(_))
    }
}
