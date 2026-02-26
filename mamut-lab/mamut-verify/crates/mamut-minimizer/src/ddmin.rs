//! Delta Debugging Minimizer implementation.
//!
//! Implements the classic DDMIN algorithm from "Simplifying and Isolating
//! Failure-Inducing Input" by Zeller & Hildebrandt (2002).
//!
//! The algorithm works by systematically removing chunks of the history
//! and verifying that the failure is still exhibited. It uses binary
//! search to efficiently find the minimal failing subset.

use std::time::Instant;

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use tracing::{debug, info, trace, warn};

use mamut_checker::{Checker, History, Operation};

use crate::error::MinimizerError;
use crate::result::{MinimizationStats, MinimizedHistory};
use crate::traits::{Minimizer, MinimizerConfig};

/// Delta Debugging Minimizer using the classic DDMIN algorithm.
///
/// The algorithm operates in two phases:
/// 1. **Reduce to subset**: Try to reduce the history to a smaller subset
///    that still exhibits the failure.
/// 2. **Reduce to complement**: Try to remove chunks from the remaining
///    history.
///
/// The algorithm increases granularity (number of chunks) when removal
/// fails and resets to 2 chunks when removal succeeds.
pub struct DeltaDebugMinimizer<V> {
    config: MinimizerConfig,
    _phantom: std::marker::PhantomData<V>,
}

impl<V> DeltaDebugMinimizer<V> {
    /// Create a new delta debugging minimizer with the given configuration.
    pub fn new(config: MinimizerConfig) -> Self {
        Self {
            config,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create a minimizer with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(MinimizerConfig::default())
    }
}

impl<V> DeltaDebugMinimizer<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
{
    /// Split operations into n chunks of approximately equal size.
    fn split_into_chunks(operations: &[Operation<V>], n: usize) -> Vec<Vec<Operation<V>>> {
        let len = operations.len();
        if n == 0 || len == 0 {
            return vec![];
        }

        let chunk_size = (len + n - 1) / n; // Ceiling division
        operations
            .chunks(chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect()
    }

    /// Get the i-th chunk from the split.
    fn get_chunk(operations: &[Operation<V>], n: usize, i: usize) -> Vec<Operation<V>> {
        let chunks = Self::split_into_chunks(operations, n);
        chunks.into_iter().nth(i).unwrap_or_default()
    }

    /// Get all chunks except the i-th one (the complement of chunk i).
    fn get_complement(operations: &[Operation<V>], n: usize, i: usize) -> Vec<Operation<V>> {
        let chunks = Self::split_into_chunks(operations, n);
        chunks
            .into_iter()
            .enumerate()
            .filter(|(idx, _)| *idx != i)
            .flat_map(|(_, chunk)| chunk)
            .collect()
    }

    /// Run the DDMIN algorithm.
    async fn ddmin(
        &self,
        operations: Vec<Operation<V>>,
        checker: &dyn Checker<Value = V>,
        stats: &mut MinimizationStats,
    ) -> Result<Vec<Operation<V>>, MinimizerError> {
        let mut current = operations;
        let mut n = 2; // Start with 2 chunks
        let mut iteration = 0;
        let mut consecutive_failures = 0;

        while current.len() >= self.config.min_size && n <= current.len() {
            iteration += 1;
            if iteration > self.config.max_iterations {
                stats.record_early_termination("max iterations reached");
                return Err(MinimizerError::MaxIterationsReached(self.config.max_iterations));
            }

            trace!(
                iteration,
                current_size = current.len(),
                granularity = n,
                "DDMIN iteration"
            );

            let mut reduced = false;

            // Try reducing to subsets (individual chunks)
            for i in 0..n {
                let chunk = Self::get_chunk(&current, n, i);
                if chunk.is_empty() {
                    continue;
                }

                let history = History::from_operations(chunk.clone());
                stats.checks_performed += 1;

                let result = checker.check(&history).await;
                if result.is_fail() {
                    debug!(
                        chunk_index = i,
                        chunk_size = chunk.len(),
                        "Reduced to subset"
                    );
                    current = chunk;
                    n = 2; // Reset granularity
                    reduced = true;
                    consecutive_failures = 0;
                    stats.record_successful_removal();
                    break;
                }
                stats.record_failed_removal();
            }

            if reduced {
                continue;
            }

            // Try reducing to complements (removing individual chunks)
            for i in 0..n {
                let complement = Self::get_complement(&current, n, i);
                if complement.is_empty() || complement.len() == current.len() {
                    continue;
                }

                let history = History::from_operations(complement.clone());
                stats.checks_performed += 1;

                let result = checker.check(&history).await;
                if result.is_fail() {
                    debug!(
                        chunk_index = i,
                        removed = current.len() - complement.len(),
                        "Reduced to complement"
                    );
                    current = complement;
                    n = n.max(2) - 1; // Reduce granularity
                    n = n.max(2);
                    reduced = true;
                    consecutive_failures = 0;
                    stats.record_successful_removal();
                    break;
                }
                stats.record_failed_removal();
            }

            if !reduced {
                // Increase granularity
                if n >= current.len() {
                    // Cannot increase granularity further, we're done
                    debug!(
                        final_size = current.len(),
                        "Minimization complete - maximum granularity reached"
                    );
                    break;
                }
                n = (2 * n).min(current.len());
                stats.record_granularity_increase(n);
                consecutive_failures += 1;

                if self.config.early_termination
                    && consecutive_failures >= self.config.early_termination_threshold
                {
                    stats.record_early_termination("no progress");
                    debug!(
                        consecutive_failures,
                        threshold = self.config.early_termination_threshold,
                        "Early termination due to lack of progress"
                    );
                    break;
                }
            }
        }

        Ok(current)
    }

    /// Perform binary search to find minimal prefix that fails.
    ///
    /// This optimization can quickly find the minimal failing prefix
    /// before running the full DDMIN algorithm.
    async fn binary_search_prefix(
        &self,
        operations: &[Operation<V>],
        checker: &dyn Checker<Value = V>,
        stats: &mut MinimizationStats,
    ) -> Option<usize> {
        let len = operations.len();
        if len == 0 {
            return None;
        }

        // First check if the full history fails
        let full_history = History::from_operations(operations.to_vec());
        stats.checks_performed += 1;
        if !checker.check(&full_history).await.is_fail() {
            return None;
        }

        // Binary search for minimal prefix
        let mut lo = 1;
        let mut hi = len;
        let mut result = len;

        while lo <= hi {
            let mid = (lo + hi) / 2;
            let prefix = operations[..mid].to_vec();
            let history = History::from_operations(prefix);
            stats.checks_performed += 1;

            if checker.check(&history).await.is_fail() {
                result = mid;
                hi = mid - 1;
                trace!(prefix_length = mid, "Prefix still fails");
            } else {
                lo = mid + 1;
                trace!(prefix_length = mid, "Prefix passes");
            }
        }

        Some(result)
    }
}

#[async_trait]
impl<V> Minimizer for DeltaDebugMinimizer<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
{
    type Value = V;

    async fn minimize(
        &self,
        history: &History<V>,
        checker: &dyn Checker<Value = V>,
    ) -> Result<MinimizedHistory<V>, MinimizerError> {
        let start = Instant::now();
        let original_size = history.len();

        if history.is_empty() {
            return Err(MinimizerError::EmptyHistory);
        }

        info!(
            original_size,
            minimizer = self.name(),
            "Starting delta debugging minimization"
        );

        let mut stats = MinimizationStats {
            original_size,
            ..Default::default()
        };

        // Verify the history actually fails
        stats.checks_performed += 1;
        let initial_result = checker.check(history).await;
        if !initial_result.is_fail() {
            return Err(MinimizerError::NoFailure);
        }

        let mut operations: Vec<Operation<V>> = history.operations.clone();

        // Try binary search for minimal prefix first
        if let Some(prefix_len) = self.binary_search_prefix(&operations, checker, &mut stats).await
        {
            if prefix_len < operations.len() {
                debug!(
                    original = operations.len(),
                    prefix = prefix_len,
                    "Found minimal prefix"
                );
                operations = operations[..prefix_len].to_vec();
            }
        }

        // Run the main DDMIN algorithm
        let minimized_ops = self.ddmin(operations.clone(), checker, &mut stats).await?;

        // Calculate removed operations
        let minimized_set: std::collections::HashSet<_> =
            minimized_ops.iter().map(|op| op.id).collect();
        let removed: Vec<Operation<V>> = history
            .operations
            .iter()
            .filter(|op| !minimized_set.contains(&op.id))
            .cloned()
            .collect();

        // Build the minimized history
        let minimized_history = History::from_operations(minimized_ops);

        // Verify the final result if configured
        if self.config.verify_final {
            stats.checks_performed += 1;
            let final_result = checker.check(&minimized_history).await;
            if !final_result.is_fail() {
                warn!("Final verification failed - minimized history no longer exhibits failure");
                return Err(MinimizerError::Internal(
                    "Minimized history no longer exhibits failure".into(),
                ));
            }
        }

        let duration = start.elapsed();
        stats.minimized_size = minimized_history.len();
        stats.operations_removed = original_size - stats.minimized_size;
        stats.duration = Some(duration);

        info!(
            original_size,
            minimized_size = stats.minimized_size,
            reduction_percent = format!("{:.1}%", (1.0 - stats.minimized_size as f64 / original_size as f64) * 100.0),
            checks = stats.checks_performed,
            duration = ?duration,
            "Minimization complete"
        );

        let steps = stats.successful_removals + stats.granularity_increases;
        let mut result = MinimizedHistory::new(minimized_history, removed, original_size, steps);
        result.stats = stats;

        Ok(result)
    }

    fn name(&self) -> &str {
        "ddmin"
    }

    fn description(&self) -> &str {
        "Delta debugging minimizer using the classic DDMIN algorithm"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_operations(n: usize) -> Vec<Operation<i32>> {
        use mamut_checker::{OperationId, OperationKind, ProcessId};

        (0..n)
            .map(|i| {
                Operation::new(
                    OperationId(i as u64),
                    ProcessId(1),
                    OperationKind::Write(i as i32),
                    i as u64,
                    i as i32,
                )
            })
            .collect()
    }

    #[test]
    fn test_split_into_chunks() {
        let ops = create_operations(10);

        let chunks = DeltaDebugMinimizer::<i32>::split_into_chunks(&ops, 2);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), 5);
        assert_eq!(chunks[1].len(), 5);

        let chunks = DeltaDebugMinimizer::<i32>::split_into_chunks(&ops, 3);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), 4);
        assert_eq!(chunks[1].len(), 4);
        assert_eq!(chunks[2].len(), 2);

        let chunks = DeltaDebugMinimizer::<i32>::split_into_chunks(&ops, 10);
        assert_eq!(chunks.len(), 10);
        for chunk in chunks {
            assert_eq!(chunk.len(), 1);
        }
    }

    #[test]
    fn test_get_chunk() {
        let ops = create_operations(10);

        let chunk0 = DeltaDebugMinimizer::<i32>::get_chunk(&ops, 2, 0);
        assert_eq!(chunk0.len(), 5);
        assert_eq!(chunk0[0].id, OperationId(0));

        let chunk1 = DeltaDebugMinimizer::<i32>::get_chunk(&ops, 2, 1);
        assert_eq!(chunk1.len(), 5);
        assert_eq!(chunk1[0].id, OperationId(5));
    }

    #[test]
    fn test_get_complement() {
        use mamut_checker::OperationId;

        let ops = create_operations(10);

        let complement0 = DeltaDebugMinimizer::<i32>::get_complement(&ops, 2, 0);
        assert_eq!(complement0.len(), 5);
        assert_eq!(complement0[0].id, OperationId(5));

        let complement1 = DeltaDebugMinimizer::<i32>::get_complement(&ops, 2, 1);
        assert_eq!(complement1.len(), 5);
        assert_eq!(complement1[0].id, OperationId(0));
    }

    #[test]
    fn test_minimizer_creation() {
        let minimizer = DeltaDebugMinimizer::<i32>::with_defaults();
        assert_eq!(minimizer.name(), "ddmin");

        let config = MinimizerConfig::new().with_max_iterations(500);
        let minimizer = DeltaDebugMinimizer::<i32>::new(config);
        assert_eq!(minimizer.config.max_iterations, 500);
    }
}
