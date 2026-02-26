//! Core traits for history minimization.

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

use mamut_checker::{Checker, History};

use crate::error::MinimizerError;
use crate::result::MinimizedHistory;

/// Configuration for minimization behavior.
#[derive(Debug, Clone)]
pub struct MinimizerConfig {
    /// Maximum number of iterations before giving up.
    pub max_iterations: usize,

    /// Enable early termination when no progress is being made.
    pub early_termination: bool,

    /// Number of consecutive failed attempts before early termination.
    pub early_termination_threshold: usize,

    /// Minimum size below which we stop minimizing.
    pub min_size: usize,

    /// Whether to verify the final minimized history.
    pub verify_final: bool,

    /// Enable verbose logging of minimization progress.
    pub verbose: bool,
}

impl Default for MinimizerConfig {
    fn default() -> Self {
        Self {
            max_iterations: 1000,
            early_termination: true,
            early_termination_threshold: 10,
            min_size: 1,
            verify_final: true,
            verbose: false,
        }
    }
}

impl MinimizerConfig {
    /// Create a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum number of iterations.
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    /// Enable or disable early termination.
    pub fn with_early_termination(mut self, enabled: bool) -> Self {
        self.early_termination = enabled;
        self
    }

    /// Set the early termination threshold.
    pub fn with_early_termination_threshold(mut self, threshold: usize) -> Self {
        self.early_termination_threshold = threshold;
        self
    }

    /// Set the minimum size for minimization.
    pub fn with_min_size(mut self, min: usize) -> Self {
        self.min_size = min;
        self
    }

    /// Enable or disable final verification.
    pub fn with_verify_final(mut self, verify: bool) -> Self {
        self.verify_final = verify;
        self
    }

    /// Enable or disable verbose logging.
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}

/// A trait for minimizing histories while preserving failure behavior.
///
/// Minimizers take a history that fails a consistency check and attempt to
/// produce the smallest possible history that still exhibits the same failure.
/// This is useful for debugging and understanding the root cause of failures.
///
/// # Type Parameters
///
/// * `Value` - The type of values in the operations. Must be serializable
///   for persistence and cloneable for manipulation.
///
/// # Example
///
/// ```rust,ignore
/// use mamut_minimizer::{Minimizer, DeltaDebugMinimizer, MinimizerConfig};
/// use mamut_checker::{History, Checker};
///
/// let minimizer = DeltaDebugMinimizer::new(MinimizerConfig::default());
/// let result = minimizer.minimize(&failing_history, &checker).await?;
/// println!("Reduced from {} to {} operations", result.original_size(), result.minimized_size());
/// ```
#[async_trait]
pub trait Minimizer: Send + Sync {
    /// The type of values in operations that this minimizer handles.
    type Value: Serialize + DeserializeOwned + Send + Sync + Clone + 'static;

    /// Minimize a history while preserving its failure behavior.
    ///
    /// Takes a history that fails the given checker and produces a minimal
    /// (or near-minimal) history that still fails the same check.
    ///
    /// # Arguments
    ///
    /// * `history` - The history to minimize. Should fail the checker.
    /// * `checker` - The consistency checker to use for verification.
    ///
    /// # Returns
    ///
    /// A `MinimizedHistory` containing the reduced history and statistics,
    /// or an error if minimization fails.
    ///
    /// # Errors
    ///
    /// * `MinimizerError::EmptyHistory` - The input history is empty.
    /// * `MinimizerError::NoFailure` - The history doesn't fail the checker.
    /// * `MinimizerError::MaxIterationsReached` - Iteration limit exceeded.
    async fn minimize(
        &self,
        history: &History<Self::Value>,
        checker: &dyn Checker<Value = Self::Value>,
    ) -> Result<MinimizedHistory<Self::Value>, MinimizerError>;

    /// Get the name of this minimizer for logging purposes.
    fn name(&self) -> &str;

    /// Get a description of the minimization strategy.
    fn description(&self) -> &str {
        "No description available"
    }
}

/// A predicate that determines whether a history exhibits the failure.
///
/// This trait allows for custom failure detection beyond just using a Checker.
#[async_trait]
pub trait FailurePredicate<V>: Send + Sync {
    /// Check if the history exhibits the failure we're trying to minimize.
    ///
    /// Returns `true` if the history still shows the failure, `false` otherwise.
    async fn exhibits_failure(&self, history: &History<V>) -> bool;
}

/// A wrapper that uses a Checker as a failure predicate.
pub struct CheckerPredicate<'a, V> {
    checker: &'a dyn Checker<Value = V>,
}

impl<'a, V> CheckerPredicate<'a, V> {
    /// Create a new checker-based failure predicate.
    pub fn new(checker: &'a dyn Checker<Value = V>) -> Self {
        Self { checker }
    }
}

#[async_trait]
impl<V> FailurePredicate<V> for CheckerPredicate<'_, V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
{
    async fn exhibits_failure(&self, history: &History<V>) -> bool {
        let result = self.checker.check(history).await;
        result.is_fail()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = MinimizerConfig::default();
        assert_eq!(config.max_iterations, 1000);
        assert!(config.early_termination);
        assert_eq!(config.early_termination_threshold, 10);
        assert_eq!(config.min_size, 1);
        assert!(config.verify_final);
        assert!(!config.verbose);
    }

    #[test]
    fn test_config_builder() {
        let config = MinimizerConfig::new()
            .with_max_iterations(500)
            .with_early_termination(false)
            .with_min_size(5)
            .with_verbose(true);

        assert_eq!(config.max_iterations, 500);
        assert!(!config.early_termination);
        assert_eq!(config.min_size, 5);
        assert!(config.verbose);
    }
}
