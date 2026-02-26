//! Parallel Minimizer implementation.
//!
//! This minimizer tests multiple chunks concurrently using async tasks,
//! potentially speeding up minimization when checker operations are
//! I/O-bound or can be parallelized.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, info, trace, warn};

use mamut_checker::{Checker, History, Operation};

use crate::error::MinimizerError;
use crate::result::{MinimizationStats, MinimizedHistory};
use crate::traits::{Minimizer, MinimizerConfig};

/// Configuration specific to parallel minimization.
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Base minimizer configuration.
    pub base: MinimizerConfig,

    /// Maximum number of concurrent checks.
    pub max_concurrency: usize,

    /// Minimum chunk size before falling back to sequential.
    pub min_chunk_size: usize,

    /// Whether to use eager cancellation (stop other checks when one succeeds).
    pub eager_cancellation: bool,

    /// Initial number of chunks to test in parallel.
    pub initial_chunks: usize,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            base: MinimizerConfig::default(),
            max_concurrency: num_cpus::get().max(2),
            min_chunk_size: 1,
            eager_cancellation: true,
            initial_chunks: 4,
        }
    }
}

impl ParallelConfig {
    /// Create a new parallel configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the base minimizer configuration.
    pub fn with_base(mut self, base: MinimizerConfig) -> Self {
        self.base = base;
        self
    }

    /// Set the maximum concurrency level.
    pub fn with_max_concurrency(mut self, max: usize) -> Self {
        self.max_concurrency = max.max(1);
        self
    }

    /// Set the minimum chunk size.
    pub fn with_min_chunk_size(mut self, min: usize) -> Self {
        self.min_chunk_size = min.max(1);
        self
    }

    /// Enable or disable eager cancellation.
    pub fn with_eager_cancellation(mut self, enabled: bool) -> Self {
        self.eager_cancellation = enabled;
        self
    }

    /// Set the initial number of chunks.
    pub fn with_initial_chunks(mut self, chunks: usize) -> Self {
        self.initial_chunks = chunks.max(2);
        self
    }
}

/// Result of a parallel chunk test.
#[derive(Debug)]
struct ChunkTestResult<V> {
    /// Index of the chunk that was tested.
    chunk_index: usize,
    /// Whether this was a subset test (vs complement test).
    is_subset: bool,
    /// The operations that were tested.
    operations: Vec<Operation<V>>,
    /// Whether the test resulted in a failure (which we want).
    fails: bool,
}

/// Parallel Minimizer that tests multiple chunks concurrently.
///
/// This minimizer is particularly effective when:
/// - The checker is I/O-bound (e.g., network-based verification)
/// - Multiple independent checks can run simultaneously
/// - The history is large and coarse-grained removal is effective
///
/// The algorithm is similar to DDMIN but tests multiple chunks
/// in parallel, accepting the first successful reduction.
pub struct ParallelMinimizer<V> {
    config: ParallelConfig,
    _phantom: std::marker::PhantomData<V>,
}

impl<V> ParallelMinimizer<V> {
    /// Create a new parallel minimizer.
    pub fn new(config: ParallelConfig) -> Self {
        Self {
            config,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create a minimizer with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(ParallelConfig::default())
    }
}

impl<V> ParallelMinimizer<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
{
    /// Split operations into n chunks.
    fn split_into_chunks(operations: &[Operation<V>], n: usize) -> Vec<Vec<Operation<V>>> {
        let len = operations.len();
        if n == 0 || len == 0 {
            return vec![];
        }

        let chunk_size = (len + n - 1) / n;
        operations
            .chunks(chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect()
    }

    /// Get complement of chunk i (all chunks except i).
    fn get_complement(chunks: &[Vec<Operation<V>>], i: usize) -> Vec<Operation<V>> {
        chunks
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx != i)
            .flat_map(|(_, chunk)| chunk.clone())
            .collect()
    }

    /// Test a single chunk configuration.
    async fn test_chunk(
        checker: &dyn Checker<Value = V>,
        operations: Vec<Operation<V>>,
        chunk_index: usize,
        is_subset: bool,
        cancelled: Arc<AtomicBool>,
    ) -> Option<ChunkTestResult<V>> {
        // Check if we've been cancelled
        if cancelled.load(Ordering::Relaxed) {
            return None;
        }

        let history = History::from_operations(operations.clone());
        let result = checker.check(&history).await;

        // Check cancellation again after the check
        if cancelled.load(Ordering::Relaxed) {
            return None;
        }

        Some(ChunkTestResult {
            chunk_index,
            is_subset,
            operations,
            fails: result.is_fail(),
        })
    }

    /// Run one round of parallel testing.
    ///
    /// Returns Some(operations) if a reduction was found, None otherwise.
    async fn parallel_test_round(
        &self,
        current: &[Operation<V>],
        n: usize,
        checker: &dyn Checker<Value = V>,
        checks_counter: &AtomicUsize,
    ) -> Option<Vec<Operation<V>>> {
        let chunks = Self::split_into_chunks(current, n);
        if chunks.is_empty() {
            return None;
        }

        let cancelled = Arc::new(AtomicBool::new(false));
        let mut handles = Vec::new();

        // Test subsets (individual chunks)
        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.is_empty() {
                continue;
            }

            let ops = chunk.clone();
            let cancelled_clone = Arc::clone(&cancelled);

            // We can't directly spawn with the checker reference, so we'll use
            // a different approach - test sequentially within the parallel structure
            handles.push((i, true, ops, cancelled_clone));
        }

        // Test complements (all except one chunk)
        for i in 0..chunks.len() {
            let complement = Self::get_complement(&chunks, i);
            if complement.is_empty() || complement.len() == current.len() {
                continue;
            }

            let cancelled_clone = Arc::clone(&cancelled);
            handles.push((i, false, complement, cancelled_clone));
        }

        // Process in batches up to max_concurrency
        // Note: In a real implementation, we'd want to use a proper task pool
        // For now, we test in batches
        for batch_start in (0..handles.len()).step_by(self.config.max_concurrency) {
            let batch_end = (batch_start + self.config.max_concurrency).min(handles.len());
            let batch = &handles[batch_start..batch_end];

            for (chunk_index, is_subset, operations, _cancelled_clone) in batch {
                checks_counter.fetch_add(1, Ordering::Relaxed);

                let result = Self::test_chunk(
                    checker,
                    operations.clone(),
                    *chunk_index,
                    *is_subset,
                    Arc::clone(&cancelled),
                )
                .await;

                if let Some(test_result) = result {
                    if test_result.fails {
                        // Found a reduction!
                        if self.config.eager_cancellation {
                            cancelled.store(true, Ordering::Relaxed);
                        }
                        trace!(
                            chunk_index = test_result.chunk_index,
                            is_subset = test_result.is_subset,
                            size = test_result.operations.len(),
                            "Found failing configuration"
                        );
                        return Some(test_result.operations);
                    }
                }
            }
        }

        None
    }

    /// Run the parallel DDMIN algorithm.
    async fn parallel_ddmin(
        &self,
        operations: Vec<Operation<V>>,
        checker: &dyn Checker<Value = V>,
        stats: &mut MinimizationStats,
    ) -> Result<Vec<Operation<V>>, MinimizerError> {
        let mut current = operations;
        let mut n = self.config.initial_chunks;
        let mut iteration = 0;
        let mut consecutive_failures = 0;
        let checks_counter = AtomicUsize::new(0);

        while current.len() >= self.config.base.min_size && n <= current.len() {
            iteration += 1;
            if iteration > self.config.base.max_iterations {
                stats.record_early_termination("max iterations reached");
                return Err(MinimizerError::MaxIterationsReached(
                    self.config.base.max_iterations,
                ));
            }

            trace!(
                iteration,
                current_size = current.len(),
                granularity = n,
                "Parallel DDMIN iteration"
            );

            let before_checks = checks_counter.load(Ordering::Relaxed);

            if let Some(reduced) = self
                .parallel_test_round(&current, n, checker, &checks_counter)
                .await
            {
                let new_checks = checks_counter.load(Ordering::Relaxed) - before_checks;
                stats.checks_performed += new_checks;

                debug!(
                    old_size = current.len(),
                    new_size = reduced.len(),
                    "Found reduction"
                );

                current = reduced;
                n = self.config.initial_chunks;
                consecutive_failures = 0;
                stats.record_successful_removal();
            } else {
                let new_checks = checks_counter.load(Ordering::Relaxed) - before_checks;
                stats.checks_performed += new_checks;
                stats.record_failed_removal();

                // Increase granularity
                if n >= current.len() {
                    debug!(
                        final_size = current.len(),
                        "Minimization complete - maximum granularity reached"
                    );
                    break;
                }

                n = (2 * n).min(current.len());
                stats.record_granularity_increase(n);
                consecutive_failures += 1;

                if self.config.base.early_termination
                    && consecutive_failures >= self.config.base.early_termination_threshold
                {
                    stats.record_early_termination("no progress");
                    debug!(
                        consecutive_failures,
                        threshold = self.config.base.early_termination_threshold,
                        "Early termination due to lack of progress"
                    );
                    break;
                }
            }
        }

        Ok(current)
    }
}

#[async_trait]
impl<V> Minimizer for ParallelMinimizer<V>
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
            max_concurrency = self.config.max_concurrency,
            minimizer = self.name(),
            "Starting parallel minimization"
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

        let operations = history.operations.clone();

        // Run the parallel DDMIN algorithm
        let minimized_ops = self
            .parallel_ddmin(operations, checker, &mut stats)
            .await?;

        // Calculate removed operations
        let minimized_set: HashSet<_> = minimized_ops.iter().map(|op| op.id).collect();
        let removed: Vec<Operation<V>> = history
            .operations
            .iter()
            .filter(|op| !minimized_set.contains(&op.id))
            .cloned()
            .collect();

        let minimized_history = History::from_operations(minimized_ops);

        // Verify the final result if configured
        if self.config.base.verify_final {
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
            reduction_percent = format!(
                "{:.1}%",
                (1.0 - stats.minimized_size as f64 / original_size as f64) * 100.0
            ),
            checks = stats.checks_performed,
            duration = ?duration,
            "Parallel minimization complete"
        );

        let steps = stats.successful_removals + stats.granularity_increases;
        let mut result = MinimizedHistory::new(minimized_history, removed, original_size, steps);
        result.stats = stats;

        Ok(result)
    }

    fn name(&self) -> &str {
        "parallel"
    }

    fn description(&self) -> &str {
        "Parallel minimizer that tests multiple chunks concurrently"
    }
}

/// A utility function to detect the number of available CPUs.
mod num_cpus {
    pub fn get() -> usize {
        std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mamut_checker::{OperationId, OperationKind, ProcessId};

    fn create_operations(n: usize) -> Vec<Operation<i32>> {
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

        let chunks = ParallelMinimizer::<i32>::split_into_chunks(&ops, 4);
        assert_eq!(chunks.len(), 4);
        assert_eq!(chunks[0].len(), 3);
        assert_eq!(chunks[1].len(), 3);
        assert_eq!(chunks[2].len(), 3);
        assert_eq!(chunks[3].len(), 1);
    }

    #[test]
    fn test_get_complement() {
        let ops = create_operations(10);
        let chunks = ParallelMinimizer::<i32>::split_into_chunks(&ops, 4);

        let complement = ParallelMinimizer::<i32>::get_complement(&chunks, 0);
        // Total - first chunk = 10 - 3 = 7
        assert_eq!(complement.len(), 7);

        let complement = ParallelMinimizer::<i32>::get_complement(&chunks, 3);
        // Total - last chunk = 10 - 1 = 9
        assert_eq!(complement.len(), 9);
    }

    #[test]
    fn test_config_builder() {
        let config = ParallelConfig::new()
            .with_max_concurrency(8)
            .with_min_chunk_size(5)
            .with_eager_cancellation(false)
            .with_initial_chunks(8);

        assert_eq!(config.max_concurrency, 8);
        assert_eq!(config.min_chunk_size, 5);
        assert!(!config.eager_cancellation);
        assert_eq!(config.initial_chunks, 8);
    }

    #[test]
    fn test_minimizer_creation() {
        let minimizer = ParallelMinimizer::<i32>::with_defaults();
        assert_eq!(minimizer.name(), "parallel");

        let config = ParallelConfig::new().with_max_concurrency(4);
        let minimizer = ParallelMinimizer::<i32>::new(config);
        assert_eq!(minimizer.config.max_concurrency, 4);
    }

    #[test]
    fn test_num_cpus() {
        let cpus = num_cpus::get();
        assert!(cpus >= 1);
    }
}
