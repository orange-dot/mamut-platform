//! Mamut Checker - Consistency checking framework for concurrent histories
//!
//! This crate provides implementations of various consistency checkers including:
//! - Linearizability (using the WGL algorithm)
//! - Sequential Consistency
//! - Causal Consistency
//! - Custom Invariant Checking
//!
//! # Example
//!
//! ```rust,ignore
//! use mamut_checker::{Checker, LinearizabilityChecker};
//! use mamut_core::History;
//!
//! let checker = LinearizabilityChecker::new(my_model)
//!     .with_timeout(Duration::from_secs(30));
//! let result = checker.check(&history).await;
//! ```

pub mod causal;
pub mod invariant;
pub mod linearizability;
pub mod result;
pub mod sequential;
pub mod tlaplus;
pub mod traits;

// Re-export main types
pub use causal::CausalConsistencyChecker;
pub use invariant::{Invariant, InvariantChecker, InvariantContext};
pub use linearizability::{LinearizabilityChecker, LinearizabilityConfig, OperationExtractor};
pub use result::{CheckResult, CheckStats, CheckStatus, Counterexample, Violation, ViolationType};
pub use sequential::SequentialConsistencyChecker;
pub use traits::{Checker, Model, SimpleModel};

// Re-export core types for convenience
pub use mamut_core::{History, Operation, OperationId, ProcessId};

/// Expected result from executing an operation on a model.
#[derive(Debug, Clone, PartialEq)]
pub enum ExpectedResult {
    /// Operation should succeed with optional value match.
    Ok,
    /// Operation should fail.
    Err(String),
    /// Any result is acceptable.
    Any,
    /// Custom validation function result.
    Custom(bool, String),
}

impl ExpectedResult {
    /// Check if this result indicates success.
    pub fn is_ok(&self) -> bool {
        matches!(self, ExpectedResult::Ok | ExpectedResult::Any)
    }

    /// Check if this result indicates an expected error.
    pub fn is_err(&self) -> bool {
        matches!(self, ExpectedResult::Err(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expected_result() {
        assert!(ExpectedResult::Ok.is_ok());
        assert!(ExpectedResult::Any.is_ok());
        assert!(!ExpectedResult::Err("error".into()).is_ok());

        assert!(ExpectedResult::Err("error".into()).is_err());
        assert!(!ExpectedResult::Ok.is_err());
    }
}
