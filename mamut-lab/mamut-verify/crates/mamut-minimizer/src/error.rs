//! Error types for the minimizer module.

use thiserror::Error;

/// Errors that can occur during history minimization.
#[derive(Debug, Error)]
pub enum MinimizerError {
    /// The history is empty and cannot be minimized.
    #[error("Cannot minimize empty history")]
    EmptyHistory,

    /// The initial history does not fail the check (nothing to minimize).
    #[error("History does not exhibit failure - nothing to minimize")]
    NoFailure,

    /// The checker returned an error during verification.
    #[error("Checker error: {0}")]
    CheckerError(String),

    /// Maximum iterations reached without convergence.
    #[error("Maximum iterations ({0}) reached without convergence")]
    MaxIterationsReached(usize),

    /// Timeout during minimization.
    #[error("Minimization timed out after {0:?}")]
    Timeout(std::time::Duration),

    /// Invalid configuration provided.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Serialization error during minimization.
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// The minimizer was cancelled.
    #[error("Minimization was cancelled")]
    Cancelled,

    /// An internal error occurred.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias for minimizer operations.
pub type Result<T> = std::result::Result<T, MinimizerError>;
