//! # mamut-minimizer
//!
//! Test case minimization and shrinking for failure reproduction using
//! delta debugging algorithms.
//!
//! This crate provides implementations of various minimization strategies
//! to reduce failing test histories to their minimal form while preserving
//! the failure behavior. This is essential for debugging and understanding
//! the root cause of consistency violations.
//!
//! ## Algorithms
//!
//! - **DeltaDebugMinimizer**: Classic DDMIN algorithm with binary search optimization
//! - **HierarchicalMinimizer**: Minimize by process, time window, then individual operations
//! - **ParallelMinimizer**: Test multiple chunks concurrently for faster minimization
//!
//! ## Example
//!
//! ```rust,ignore
//! use mamut_minimizer::{Minimizer, DeltaDebugMinimizer, MinimizerConfig};
//! use mamut_checker::{History, Checker};
//!
//! // Create a minimizer
//! let minimizer = DeltaDebugMinimizer::new(MinimizerConfig::default());
//!
//! // Minimize a failing history
//! let result = minimizer.minimize(&failing_history, &checker).await?;
//!
//! println!("Reduced from {} to {} operations ({:.1}% reduction)",
//!     result.original_size(),
//!     result.minimized_size(),
//!     result.reduction_percentage()
//! );
//!
//! // The minimized history still exhibits the failure
//! assert!(checker.check(&result.history).await.is_fail());
//! ```
//!
//! ## Choosing a Minimizer
//!
//! - Use **DeltaDebugMinimizer** for general-purpose minimization with good
//!   performance on most inputs.
//!
//! - Use **HierarchicalMinimizer** when the history has clear structure by
//!   process or time, allowing for more efficient coarse-grained removal.
//!
//! - Use **ParallelMinimizer** when the checker is I/O-bound or can handle
//!   concurrent checks, trading more checker invocations for wall-clock time.
//!
//! ## Configuration
//!
//! All minimizers support configuration through `MinimizerConfig`:
//!
//! ```rust,ignore
//! use mamut_minimizer::MinimizerConfig;
//!
//! let config = MinimizerConfig::new()
//!     .with_max_iterations(500)      // Limit iterations
//!     .with_early_termination(true)  // Stop when no progress
//!     .with_min_size(1)              // Minimum history size
//!     .with_verify_final(true);      // Verify result still fails
//! ```

pub mod ddmin;
pub mod error;
pub mod hierarchical;
pub mod parallel;
pub mod result;
pub mod traits;

// Re-export main types for convenient access
pub use ddmin::DeltaDebugMinimizer;
pub use error::{MinimizerError, Result};
pub use hierarchical::{HierarchicalConfig, HierarchicalMinimizer};
pub use parallel::{ParallelConfig, ParallelMinimizer};
pub use result::{MinimizationStats, MinimizedHistory};
pub use traits::{CheckerPredicate, FailurePredicate, Minimizer, MinimizerConfig};

// Re-export commonly used types from dependencies
pub use mamut_checker::{History, Operation};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reexports() {
        // Ensure all re-exports are accessible
        let _config = MinimizerConfig::default();
        let _h_config = HierarchicalConfig::default();
        let _p_config = ParallelConfig::default();
    }
}
