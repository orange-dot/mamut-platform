//! Hierarchical Minimizer implementation.
//!
//! This minimizer uses a hierarchical approach to minimize histories:
//! 1. First, try to minimize by process (remove entire processes)
//! 2. Then, minimize by time windows (remove time-based chunks)
//! 3. Finally, minimize individual operations
//!
//! This approach is often more efficient than flat DDMIN for histories
//! with clear process or temporal structure.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Instant;

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use tracing::{debug, info, trace};

use mamut_checker::{Checker, History, Operation, ProcessId};

use crate::ddmin::DeltaDebugMinimizer;
use crate::error::MinimizerError;
use crate::result::{MinimizationStats, MinimizedHistory};
use crate::traits::{Minimizer, MinimizerConfig};

/// Configuration specific to hierarchical minimization.
#[derive(Debug, Clone)]
pub struct HierarchicalConfig {
    /// Base minimizer configuration.
    pub base: MinimizerConfig,

    /// Whether to perform process-level minimization.
    pub minimize_by_process: bool,

    /// Whether to perform time-window minimization.
    pub minimize_by_time: bool,

    /// Whether to perform individual operation minimization.
    pub minimize_individual: bool,

    /// Number of time windows to use for time-based minimization.
    pub time_windows: usize,

    /// Minimum window duration as a fraction of total duration.
    pub min_window_fraction: f64,
}

impl Default for HierarchicalConfig {
    fn default() -> Self {
        Self {
            base: MinimizerConfig::default(),
            minimize_by_process: true,
            minimize_by_time: true,
            minimize_individual: true,
            time_windows: 4,
            min_window_fraction: 0.1,
        }
    }
}

impl HierarchicalConfig {
    /// Create a new hierarchical configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the base minimizer configuration.
    pub fn with_base(mut self, base: MinimizerConfig) -> Self {
        self.base = base;
        self
    }

    /// Enable or disable process-level minimization.
    pub fn with_process_minimization(mut self, enabled: bool) -> Self {
        self.minimize_by_process = enabled;
        self
    }

    /// Enable or disable time-window minimization.
    pub fn with_time_minimization(mut self, enabled: bool) -> Self {
        self.minimize_by_time = enabled;
        self
    }

    /// Enable or disable individual operation minimization.
    pub fn with_individual_minimization(mut self, enabled: bool) -> Self {
        self.minimize_individual = enabled;
        self
    }

    /// Set the number of time windows.
    pub fn with_time_windows(mut self, windows: usize) -> Self {
        self.time_windows = windows;
        self
    }
}

/// Hierarchical Minimizer that minimizes in stages.
///
/// The hierarchical approach exploits the structure of concurrent histories:
/// - Processes often contribute independently to failures
/// - Temporal locality often matters for failures
/// - Coarse-grained removal is faster than fine-grained
///
/// By minimizing hierarchically, we can often achieve significant reduction
/// with fewer checker invocations than flat DDMIN.
pub struct HierarchicalMinimizer<V> {
    config: HierarchicalConfig,
    _phantom: std::marker::PhantomData<V>,
}

impl<V> HierarchicalMinimizer<V> {
    /// Create a new hierarchical minimizer.
    pub fn new(config: HierarchicalConfig) -> Self {
        Self {
            config,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create a minimizer with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(HierarchicalConfig::default())
    }
}

impl<V> HierarchicalMinimizer<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
{
    /// Group operations by process.
    fn group_by_process(operations: &[Operation<V>]) -> HashMap<ProcessId, Vec<Operation<V>>> {
        let mut groups: HashMap<ProcessId, Vec<Operation<V>>> = HashMap::new();
        for op in operations {
            groups.entry(op.process_id).or_default().push(op.clone());
        }
        groups
    }

    /// Group operations by time window.
    fn group_by_time_window(
        operations: &[Operation<V>],
        num_windows: usize,
    ) -> BTreeMap<usize, Vec<Operation<V>>> {
        if operations.is_empty() || num_windows == 0 {
            return BTreeMap::new();
        }

        // Find time range
        let min_time = operations.iter().map(|op| op.call_time).min().unwrap_or(0);
        let max_time = operations
            .iter()
            .filter_map(|op| op.return_time)
            .max()
            .unwrap_or_else(|| operations.iter().map(|op| op.call_time).max().unwrap_or(0));

        let duration = max_time.saturating_sub(min_time) + 1;
        let window_size = (duration + num_windows as u64 - 1) / num_windows as u64;

        let mut windows: BTreeMap<usize, Vec<Operation<V>>> = BTreeMap::new();
        for op in operations {
            let window_idx = ((op.call_time - min_time) / window_size) as usize;
            windows.entry(window_idx).or_default().push(op.clone());
        }

        windows
    }

    /// Minimize by removing entire processes.
    async fn minimize_by_process(
        &self,
        operations: Vec<Operation<V>>,
        checker: &dyn Checker<Value = V>,
        stats: &mut MinimizationStats,
    ) -> Result<Vec<Operation<V>>, MinimizerError> {
        let process_groups = Self::group_by_process(&operations);
        let processes: Vec<ProcessId> = process_groups.keys().cloned().collect();

        if processes.len() <= 1 {
            debug!("Only one process, skipping process-level minimization");
            return Ok(operations);
        }

        debug!(
            num_processes = processes.len(),
            "Attempting process-level minimization"
        );

        let mut current_processes: HashSet<ProcessId> = processes.iter().cloned().collect();
        let mut made_progress = true;

        while made_progress && current_processes.len() > 1 {
            made_progress = false;

            for process in current_processes.clone() {
                // Try removing this process
                let mut remaining_processes = current_processes.clone();
                remaining_processes.remove(&process);

                let filtered_ops: Vec<Operation<V>> = operations
                    .iter()
                    .filter(|op| remaining_processes.contains(&op.process_id))
                    .cloned()
                    .collect();

                if filtered_ops.is_empty() {
                    continue;
                }

                let history = History::from_operations(filtered_ops.clone());
                stats.checks_performed += 1;

                if checker.check(&history).await.is_fail() {
                    trace!(removed_process = ?process, "Successfully removed process");
                    current_processes = remaining_processes;
                    stats.record_successful_removal();
                    made_progress = true;
                    break;
                } else {
                    stats.record_failed_removal();
                }
            }
        }

        let minimized: Vec<Operation<V>> = operations
            .into_iter()
            .filter(|op| current_processes.contains(&op.process_id))
            .collect();

        debug!(
            original_processes = processes.len(),
            remaining_processes = current_processes.len(),
            "Process-level minimization complete"
        );

        Ok(minimized)
    }

    /// Minimize by removing time windows.
    async fn minimize_by_time_window(
        &self,
        operations: Vec<Operation<V>>,
        checker: &dyn Checker<Value = V>,
        stats: &mut MinimizationStats,
    ) -> Result<Vec<Operation<V>>, MinimizerError> {
        let windows = Self::group_by_time_window(&operations, self.config.time_windows);

        if windows.len() <= 1 {
            debug!("Only one time window, skipping time-based minimization");
            return Ok(operations);
        }

        debug!(
            num_windows = windows.len(),
            "Attempting time-window minimization"
        );

        let window_indices: Vec<usize> = windows.keys().cloned().collect();
        let mut current_windows: HashSet<usize> = window_indices.iter().cloned().collect();
        let mut made_progress = true;

        while made_progress && current_windows.len() > 1 {
            made_progress = false;

            for &window_idx in &window_indices {
                if !current_windows.contains(&window_idx) {
                    continue;
                }

                // Try removing this window
                let mut remaining_windows = current_windows.clone();
                remaining_windows.remove(&window_idx);

                let filtered_ops: Vec<Operation<V>> = windows
                    .iter()
                    .filter(|(idx, _)| remaining_windows.contains(idx))
                    .flat_map(|(_, ops)| ops.clone())
                    .collect();

                if filtered_ops.is_empty() {
                    continue;
                }

                let history = History::from_operations(filtered_ops);
                stats.checks_performed += 1;

                if checker.check(&history).await.is_fail() {
                    trace!(removed_window = window_idx, "Successfully removed time window");
                    current_windows = remaining_windows;
                    stats.record_successful_removal();
                    made_progress = true;
                    break;
                } else {
                    stats.record_failed_removal();
                }
            }
        }

        let minimized: Vec<Operation<V>> = windows
            .into_iter()
            .filter(|(idx, _)| current_windows.contains(idx))
            .flat_map(|(_, ops)| ops)
            .collect();

        debug!(
            original_windows = window_indices.len(),
            remaining_windows = current_windows.len(),
            "Time-window minimization complete"
        );

        Ok(minimized)
    }

    /// Final pass: minimize individual operations using DDMIN.
    async fn minimize_individual(
        &self,
        operations: Vec<Operation<V>>,
        checker: &dyn Checker<Value = V>,
        stats: &mut MinimizationStats,
    ) -> Result<Vec<Operation<V>>, MinimizerError> {
        let history = History::from_operations(operations);

        // Use the DDMIN minimizer for fine-grained minimization
        let ddmin = DeltaDebugMinimizer::new(self.config.base.clone());

        match ddmin.minimize(&history, checker).await {
            Ok(result) => {
                // Merge stats
                stats.checks_performed += result.stats.checks_performed;
                stats.successful_removals += result.stats.successful_removals;
                stats.failed_removals += result.stats.failed_removals;
                stats.granularity_increases += result.stats.granularity_increases;
                stats.max_granularity = stats
                    .max_granularity
                    .max(result.stats.max_granularity);

                Ok(result.history.operations)
            }
            Err(MinimizerError::NoFailure) => {
                // History no longer fails after earlier phases - this shouldn't happen
                // but return what we have
                Ok(history.operations)
            }
            Err(e) => Err(e),
        }
    }
}

#[async_trait]
impl<V> Minimizer for HierarchicalMinimizer<V>
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
            "Starting hierarchical minimization"
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

        let mut operations = history.operations.clone();

        // Phase 1: Minimize by process
        if self.config.minimize_by_process {
            debug!(
                size = operations.len(),
                "Phase 1: Process-level minimization"
            );
            operations = self
                .minimize_by_process(operations, checker, &mut stats)
                .await?;
        }

        // Phase 2: Minimize by time window
        if self.config.minimize_by_time {
            debug!(
                size = operations.len(),
                "Phase 2: Time-window minimization"
            );
            operations = self
                .minimize_by_time_window(operations, checker, &mut stats)
                .await?;
        }

        // Phase 3: Minimize individual operations
        if self.config.minimize_individual {
            debug!(
                size = operations.len(),
                "Phase 3: Individual operation minimization"
            );
            operations = self
                .minimize_individual(operations, checker, &mut stats)
                .await?;
        }

        // Calculate removed operations
        let minimized_set: HashSet<_> = operations.iter().map(|op| op.id).collect();
        let removed: Vec<Operation<V>> = history
            .operations
            .iter()
            .filter(|op| !minimized_set.contains(&op.id))
            .cloned()
            .collect();

        let minimized_history = History::from_operations(operations);

        // Final verification
        if self.config.base.verify_final {
            stats.checks_performed += 1;
            let final_result = checker.check(&minimized_history).await;
            if !final_result.is_fail() {
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
            "Hierarchical minimization complete"
        );

        let steps = stats.successful_removals + stats.granularity_increases;
        let mut result = MinimizedHistory::new(minimized_history, removed, original_size, steps);
        result.stats = stats;

        Ok(result)
    }

    fn name(&self) -> &str {
        "hierarchical"
    }

    fn description(&self) -> &str {
        "Hierarchical minimizer that minimizes by process, time window, then individual operations"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mamut_checker::{OperationId, OperationKind};

    fn create_operation(id: u64, process: u64, call_time: u64) -> Operation<i32> {
        Operation::new(
            OperationId(id),
            ProcessId(process),
            OperationKind::Write(id as i32),
            call_time,
            id as i32,
        )
        .with_return_time(call_time + 10)
    }

    #[test]
    fn test_group_by_process() {
        let ops = vec![
            create_operation(1, 1, 0),
            create_operation(2, 2, 5),
            create_operation(3, 1, 10),
            create_operation(4, 2, 15),
            create_operation(5, 3, 20),
        ];

        let groups = HierarchicalMinimizer::<i32>::group_by_process(&ops);
        assert_eq!(groups.len(), 3);
        assert_eq!(groups.get(&ProcessId(1)).unwrap().len(), 2);
        assert_eq!(groups.get(&ProcessId(2)).unwrap().len(), 2);
        assert_eq!(groups.get(&ProcessId(3)).unwrap().len(), 1);
    }

    #[test]
    fn test_group_by_time_window() {
        let ops = vec![
            create_operation(1, 1, 0),
            create_operation(2, 1, 10),
            create_operation(3, 1, 20),
            create_operation(4, 1, 30),
            create_operation(5, 1, 40),
        ];

        let windows = HierarchicalMinimizer::<i32>::group_by_time_window(&ops, 2);
        assert_eq!(windows.len(), 2);

        let windows = HierarchicalMinimizer::<i32>::group_by_time_window(&ops, 5);
        assert!(windows.len() <= 5);
    }

    #[test]
    fn test_config_builder() {
        let config = HierarchicalConfig::new()
            .with_process_minimization(false)
            .with_time_windows(8)
            .with_individual_minimization(true);

        assert!(!config.minimize_by_process);
        assert!(config.minimize_by_time);
        assert!(config.minimize_individual);
        assert_eq!(config.time_windows, 8);
    }
}
