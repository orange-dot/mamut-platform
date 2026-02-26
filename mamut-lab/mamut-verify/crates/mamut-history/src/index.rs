//! In-memory index for efficient history queries.
//!
//! This module provides indexing capabilities for:
//! - Operation lookup by ID
//! - Process-to-operations mapping
//! - Time-based range queries

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, Utc};
use mamut_core::{OperationId, ProcessId};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::events::{HistoryEvent, OperationEvent};

/// Statistics about the index
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexStats {
    /// Total number of indexed operations
    pub total_operations: usize,
    /// Number of unique processes
    pub unique_processes: usize,
    /// Number of time index entries
    pub time_entries: usize,
    /// Memory usage estimate in bytes
    pub estimated_memory_bytes: usize,
}

/// In-memory index for efficient history queries
#[derive(Debug, Default)]
pub struct HistoryIndex {
    /// Maps operation ID to stream position
    operation_positions: HashMap<OperationId, u64>,
    /// Maps process ID to list of operation positions
    process_operations: HashMap<ProcessId, Vec<u64>>,
    /// Maps timestamp (as unix millis) to list of positions
    time_index: BTreeMap<u64, Vec<u64>>,
    /// Reverse lookup: position to operation ID
    position_to_operation: HashMap<u64, OperationId>,
    /// Set of all known processes
    known_processes: HashSet<ProcessId>,
    /// Index statistics
    stats: IndexStats,
}

impl HistoryIndex {
    /// Create a new empty index
    pub fn new() -> Self {
        Self::default()
    }

    /// Index an event at a given stream position
    pub fn index_event(&mut self, event: &HistoryEvent, position: u64) {
        match event {
            HistoryEvent::Operation(op_event) => {
                self.index_operation_event(op_event, position);
            }
            _ => {
                // Index by timestamp for all events
                let timestamp_millis = event.timestamp().timestamp_millis() as u64;
                self.time_index
                    .entry(timestamp_millis)
                    .or_default()
                    .push(position);
            }
        }

        self.update_stats();
    }

    /// Index an operation event
    fn index_operation_event(&mut self, event: &OperationEvent, position: u64) {
        let operation_id = event.operation_id();
        let process_id = event.process_id();
        let timestamp_millis = event.timestamp().timestamp_millis() as u64;

        // Index by operation ID (only for Invoked events to avoid duplicates)
        if matches!(event, OperationEvent::Invoked { .. }) {
            self.operation_positions.insert(operation_id, position);
            self.position_to_operation.insert(position, operation_id);
        }

        // Index by process ID
        self.process_operations
            .entry(process_id)
            .or_default()
            .push(position);

        // Track known processes
        self.known_processes.insert(process_id);

        // Index by timestamp
        self.time_index
            .entry(timestamp_millis)
            .or_default()
            .push(position);

        self.stats.total_operations += 1;
    }

    /// Update index statistics
    fn update_stats(&mut self) {
        self.stats.unique_processes = self.known_processes.len();
        self.stats.time_entries = self.time_index.len();

        // Rough memory estimation
        let op_mem = self.operation_positions.len() * (std::mem::size_of::<OperationId>() + 8);
        let proc_mem = self
            .process_operations
            .values()
            .map(|v| v.len() * 8)
            .sum::<usize>()
            + self.process_operations.len() * std::mem::size_of::<ProcessId>();
        let time_mem = self
            .time_index
            .values()
            .map(|v| v.len() * 8)
            .sum::<usize>()
            + self.time_index.len() * 16;

        self.stats.estimated_memory_bytes = op_mem + proc_mem + time_mem;
    }

    /// Get the position of an operation by ID
    pub fn get_operation_position(&self, operation_id: &OperationId) -> Option<u64> {
        self.operation_positions.get(operation_id).copied()
    }

    /// Get all positions for operations by a process
    pub fn get_process_operations(&self, process_id: &ProcessId) -> Vec<u64> {
        self.process_operations
            .get(process_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get positions in a time range (timestamps in unix milliseconds)
    pub fn get_positions_in_time_range(&self, from_millis: u64, to_millis: u64) -> Vec<u64> {
        self.time_index
            .range(from_millis..=to_millis)
            .flat_map(|(_, positions)| positions.iter().copied())
            .collect()
    }

    /// Get positions in a DateTime range
    pub fn get_positions_in_datetime_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Vec<u64> {
        let from_millis = from.timestamp_millis() as u64;
        let to_millis = to.timestamp_millis() as u64;
        self.get_positions_in_time_range(from_millis, to_millis)
    }

    /// Get the operation ID at a position
    pub fn get_operation_at_position(&self, position: u64) -> Option<&OperationId> {
        self.position_to_operation.get(&position)
    }

    /// Get all known process IDs
    pub fn get_all_processes(&self) -> Vec<ProcessId> {
        self.known_processes.iter().copied().collect()
    }

    /// Get the number of operations for a process
    pub fn get_process_operation_count(&self, process_id: &ProcessId) -> usize {
        self.process_operations
            .get(process_id)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Get statistics about the index
    pub fn stats(&self) -> &IndexStats {
        &self.stats
    }

    /// Clear the index
    pub fn clear(&mut self) {
        self.operation_positions.clear();
        self.process_operations.clear();
        self.time_index.clear();
        self.position_to_operation.clear();
        self.known_processes.clear();
        self.stats = IndexStats::default();
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.operation_positions.is_empty()
    }

    /// Get the total number of indexed operations
    pub fn len(&self) -> usize {
        self.stats.total_operations
    }

    /// Build an index from a list of events with positions
    pub fn build_from_events(events: impl IntoIterator<Item = (HistoryEvent, u64)>) -> Self {
        let mut index = Self::new();
        for (event, position) in events {
            index.index_event(&event, position);
        }
        info!(
            operations = index.stats.total_operations,
            processes = index.stats.unique_processes,
            "Index built"
        );
        index
    }

    /// Merge another index into this one
    pub fn merge(&mut self, other: HistoryIndex) {
        for (op_id, pos) in other.operation_positions {
            self.operation_positions.insert(op_id, pos);
        }

        for (proc_id, positions) in other.process_operations {
            self.process_operations
                .entry(proc_id)
                .or_default()
                .extend(positions);
        }

        for (timestamp, positions) in other.time_index {
            self.time_index
                .entry(timestamp)
                .or_default()
                .extend(positions);
        }

        for (pos, op_id) in other.position_to_operation {
            self.position_to_operation.insert(pos, op_id);
        }

        self.known_processes.extend(other.known_processes);
        self.update_stats();
    }

    /// Sort process operations by position for consistent ordering
    pub fn sort_process_operations(&mut self) {
        for positions in self.process_operations.values_mut() {
            positions.sort_unstable();
        }
    }

    /// Get positions around a specific position (for context)
    pub fn get_surrounding_positions(&self, position: u64, context: u64) -> Vec<u64> {
        let start = position.saturating_sub(context);
        let end = position.saturating_add(context);

        let mut positions: Vec<u64> = self
            .position_to_operation
            .keys()
            .filter(|&&p| p >= start && p <= end)
            .copied()
            .collect();

        positions.sort_unstable();
        positions
    }

    /// Find the nearest position before a timestamp
    pub fn find_position_before(&self, timestamp_millis: u64) -> Option<u64> {
        self.time_index
            .range(..=timestamp_millis)
            .next_back()
            .and_then(|(_, positions)| positions.last().copied())
    }

    /// Find the nearest position after a timestamp
    pub fn find_position_after(&self, timestamp_millis: u64) -> Option<u64> {
        self.time_index
            .range(timestamp_millis..)
            .next()
            .and_then(|(_, positions)| positions.first().copied())
    }

    /// Export the index for persistence
    pub fn export(&self) -> IndexExport {
        IndexExport {
            operation_positions: self
                .operation_positions
                .iter()
                .map(|(k, v)| (*k, *v))
                .collect(),
            process_operations: self
                .process_operations
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
            time_index: self.time_index.clone(),
        }
    }

    /// Import a previously exported index
    pub fn import(export: IndexExport) -> Self {
        let mut index = Self::new();

        for (op_id, pos) in export.operation_positions {
            index.operation_positions.insert(op_id, pos);
            index.position_to_operation.insert(pos, op_id);
        }

        for (proc_id, positions) in export.process_operations {
            index.known_processes.insert(proc_id);
            index.process_operations.insert(proc_id, positions);
        }

        index.time_index = export.time_index;
        index.update_stats();

        index
    }
}

/// Serializable export of the index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexExport {
    pub operation_positions: Vec<(OperationId, u64)>,
    pub process_operations: Vec<(ProcessId, Vec<u64>)>,
    pub time_index: BTreeMap<u64, Vec<u64>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::OperationType;

    fn create_test_operation_event(process: u64, timestamp_ms: i64) -> HistoryEvent {
        HistoryEvent::Operation(OperationEvent::Invoked {
            operation_id: OperationId::new(rand::random()),
            process_id: ProcessId(process),
            timestamp: DateTime::from_timestamp_millis(timestamp_ms).unwrap(),
            operation_type: OperationType {
                name: "write".to_string(),
                target: "register".to_string(),
            },
            arguments: vec![],
        })
    }

    #[test]
    fn test_index_operation() {
        let mut index = HistoryIndex::new();
        let event = create_test_operation_event(1, 1000);

        index.index_event(&event, 0);

        assert_eq!(index.len(), 1);
        assert!(index.get_all_processes().contains(&ProcessId(1)));
    }

    #[test]
    fn test_process_operations() {
        let mut index = HistoryIndex::new();

        for i in 0..5 {
            let event = create_test_operation_event(1, 1000 + i * 100);
            index.index_event(&event, i as u64);
        }

        for i in 0..3 {
            let event = create_test_operation_event(2, 1050 + i * 100);
            index.index_event(&event, (5 + i) as u64);
        }

        let p1_ops = index.get_process_operations(&ProcessId(1));
        assert_eq!(p1_ops.len(), 5);

        let p2_ops = index.get_process_operations(&ProcessId(2));
        assert_eq!(p2_ops.len(), 3);
    }

    #[test]
    fn test_time_range_query() {
        let mut index = HistoryIndex::new();

        for i in 0..10 {
            let event = create_test_operation_event(1, 1000 + i * 100);
            index.index_event(&event, i as u64);
        }

        let positions = index.get_positions_in_time_range(1200, 1500);
        // Should include events at 1200, 1300, 1400, 1500
        assert_eq!(positions.len(), 4);
    }

    #[test]
    fn test_index_export_import() {
        let mut index = HistoryIndex::new();

        for i in 0..5 {
            let event = create_test_operation_event(1, 1000 + i * 100);
            index.index_event(&event, i as u64);
        }

        let export = index.export();
        let imported = HistoryIndex::import(export);

        assert_eq!(imported.len(), index.len());
        assert_eq!(
            imported.get_all_processes().len(),
            index.get_all_processes().len()
        );
    }

    #[test]
    fn test_find_position_before_after() {
        let mut index = HistoryIndex::new();

        for i in 0..5 {
            let event = create_test_operation_event(1, 1000 + i * 1000);
            index.index_event(&event, i as u64);
        }

        // Find position before 2500 (should be event at 2000, position 2)
        let before = index.find_position_before(2500);
        assert!(before.is_some());

        // Find position after 2500 (should be event at 3000, position 3)
        let after = index.find_position_after(2500);
        assert!(after.is_some());
    }

    #[test]
    fn test_merge_indices() {
        let mut index1 = HistoryIndex::new();
        let mut index2 = HistoryIndex::new();

        for i in 0..5 {
            let event = create_test_operation_event(1, 1000 + i * 100);
            index1.index_event(&event, i as u64);
        }

        for i in 0..3 {
            let event = create_test_operation_event(2, 2000 + i * 100);
            index2.index_event(&event, (10 + i) as u64);
        }

        index1.merge(index2);

        assert_eq!(index1.len(), 8);
        assert_eq!(index1.get_all_processes().len(), 2);
    }
}
