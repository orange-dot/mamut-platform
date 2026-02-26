//! Result types for minimization operations.

use std::fmt;
use std::time::Duration;

use mamut_checker::{History, Operation};
use serde::{Deserialize, Serialize};

/// The result of a successful minimization.
///
/// Contains the minimized history along with statistics about the
/// minimization process.
#[derive(Debug, Clone)]
pub struct MinimizedHistory<V> {
    /// The minimized history that still exhibits the failure.
    pub history: History<V>,

    /// Operations that were removed during minimization.
    pub removed: Vec<Operation<V>>,

    /// The ratio of original size to minimized size (0.0 to 1.0).
    /// Lower values indicate more aggressive reduction.
    pub reduction_ratio: f64,

    /// Number of minimization steps/iterations performed.
    pub steps: usize,

    /// Statistics about the minimization process.
    pub stats: MinimizationStats,
}

impl<V> MinimizedHistory<V> {
    /// Create a new minimized history result.
    pub fn new(
        history: History<V>,
        removed: Vec<Operation<V>>,
        original_size: usize,
        steps: usize,
    ) -> Self {
        let minimized_size = history.len();
        let reduction_ratio = if original_size > 0 {
            minimized_size as f64 / original_size as f64
        } else {
            1.0
        };

        Self {
            history,
            removed,
            reduction_ratio,
            steps,
            stats: MinimizationStats {
                original_size,
                minimized_size,
                operations_removed: original_size.saturating_sub(minimized_size),
                ..Default::default()
            },
        }
    }

    /// Add timing information to the result.
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.stats.duration = Some(duration);
        self
    }

    /// Add the number of checks performed.
    pub fn with_checks_performed(mut self, checks: usize) -> Self {
        self.stats.checks_performed = checks;
        self
    }

    /// Get the original size of the history.
    pub fn original_size(&self) -> usize {
        self.stats.original_size
    }

    /// Get the minimized size of the history.
    pub fn minimized_size(&self) -> usize {
        self.stats.minimized_size
    }

    /// Get the percentage reduction achieved.
    pub fn reduction_percentage(&self) -> f64 {
        (1.0 - self.reduction_ratio) * 100.0
    }
}

impl<V: fmt::Debug> fmt::Display for MinimizedHistory<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MinimizedHistory {{ {} -> {} operations ({:.1}% reduction), {} steps",
            self.stats.original_size,
            self.stats.minimized_size,
            self.reduction_percentage(),
            self.steps
        )?;
        if let Some(duration) = self.stats.duration {
            write!(f, ", {:?}", duration)?;
        }
        write!(f, " }}")
    }
}

/// Statistics about the minimization process.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MinimizationStats {
    /// Size of the original history.
    pub original_size: usize,

    /// Size of the minimized history.
    pub minimized_size: usize,

    /// Number of operations removed.
    pub operations_removed: usize,

    /// Time taken for minimization.
    #[serde(skip)]
    pub duration: Option<Duration>,

    /// Number of check operations performed.
    pub checks_performed: usize,

    /// Number of successful chunk removals.
    pub successful_removals: usize,

    /// Number of failed chunk removal attempts.
    pub failed_removals: usize,

    /// Number of times we had to increase granularity.
    pub granularity_increases: usize,

    /// Maximum granularity reached during minimization.
    pub max_granularity: usize,

    /// Whether early termination was triggered.
    pub early_terminated: bool,

    /// Reason for early termination, if applicable.
    pub termination_reason: Option<String>,
}

impl MinimizationStats {
    /// Create new empty stats.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a successful chunk removal.
    pub fn record_successful_removal(&mut self) {
        self.successful_removals += 1;
    }

    /// Record a failed chunk removal attempt.
    pub fn record_failed_removal(&mut self) {
        self.failed_removals += 1;
    }

    /// Record a granularity increase.
    pub fn record_granularity_increase(&mut self, new_granularity: usize) {
        self.granularity_increases += 1;
        if new_granularity > self.max_granularity {
            self.max_granularity = new_granularity;
        }
    }

    /// Record early termination.
    pub fn record_early_termination(&mut self, reason: &str) {
        self.early_terminated = true;
        self.termination_reason = Some(reason.to_string());
    }

    /// Get the total number of removal attempts.
    pub fn total_attempts(&self) -> usize {
        self.successful_removals + self.failed_removals
    }

    /// Get the success rate of removal attempts.
    pub fn success_rate(&self) -> f64 {
        let total = self.total_attempts();
        if total == 0 {
            0.0
        } else {
            self.successful_removals as f64 / total as f64
        }
    }
}

impl fmt::Display for MinimizationStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Stats {{ {}/{} operations removed, {} checks, {:.1}% success rate",
            self.operations_removed,
            self.original_size,
            self.checks_performed,
            self.success_rate() * 100.0
        )?;
        if self.early_terminated {
            if let Some(ref reason) = self.termination_reason {
                write!(f, ", early terminated: {}", reason)?;
            }
        }
        write!(f, " }}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mamut_checker::{OperationId, OperationKind, ProcessId};

    fn create_test_operation(id: u64) -> Operation<i32> {
        Operation::new(
            OperationId(id),
            ProcessId(1),
            OperationKind::Write(id as i32),
            id,
            id as i32,
        )
    }

    #[test]
    fn test_minimized_history_creation() {
        let operations: Vec<Operation<i32>> = (1..=5).map(create_test_operation).collect();
        let history = History::from_operations(operations[0..3].to_vec());
        let removed = operations[3..].to_vec();

        let result = MinimizedHistory::new(history, removed, 5, 10);

        assert_eq!(result.original_size(), 5);
        assert_eq!(result.minimized_size(), 3);
        assert_eq!(result.steps, 10);
        assert!((result.reduction_ratio - 0.6).abs() < 0.01);
        assert!((result.reduction_percentage() - 40.0).abs() < 0.1);
    }

    #[test]
    fn test_minimization_stats() {
        let mut stats = MinimizationStats::new();
        stats.original_size = 100;
        stats.minimized_size = 20;
        stats.operations_removed = 80;

        stats.record_successful_removal();
        stats.record_successful_removal();
        stats.record_failed_removal();

        assert_eq!(stats.total_attempts(), 3);
        assert!((stats.success_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_early_termination() {
        let mut stats = MinimizationStats::new();
        stats.record_early_termination("max iterations reached");

        assert!(stats.early_terminated);
        assert_eq!(
            stats.termination_reason,
            Some("max iterations reached".to_string())
        );
    }
}
