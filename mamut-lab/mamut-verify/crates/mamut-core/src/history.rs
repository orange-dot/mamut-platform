//! History container and storage traits.
//!
//! This module provides the `History` type that collects operations from a
//! verification run, along with the `HistoryStore` trait for persisting and
//! loading histories.

use crate::error::HistoryError;
use crate::operation::Operation;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for a test run.
///
/// Each verification run is assigned a unique RunId that can be used to
/// identify and retrieve the history of operations from that run.
///
/// # Examples
///
/// ```
/// use mamut_core::history::RunId;
///
/// let run_id = RunId::new();
/// println!("Run ID: {}", run_id);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunId(Uuid);

impl RunId {
    /// Creates a new unique RunId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates a RunId from a UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the inner UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Creates a RunId from a string representation.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not a valid UUID.
    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl Default for RunId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for RunId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<RunId> for Uuid {
    fn from(run_id: RunId) -> Self {
        run_id.0
    }
}

/// Metadata associated with a history.
///
/// This captures information about the test run that produced the history,
/// including timing, configuration, and environment details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryMetadata {
    /// Human-readable name or description of the test.
    pub name: Option<String>,

    /// When the test run started (Unix timestamp in milliseconds).
    pub started_at: u64,

    /// When the test run ended (Unix timestamp in milliseconds).
    pub ended_at: Option<u64>,

    /// Number of processes in the test.
    pub num_processes: u32,

    /// Number of nodes in the test.
    pub num_nodes: u32,

    /// The seed used for random number generation (if applicable).
    pub seed: Option<u64>,

    /// Additional key-value metadata.
    #[serde(default)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

impl HistoryMetadata {
    /// Creates new metadata with the given start time.
    pub fn new(started_at: u64) -> Self {
        Self {
            name: None,
            started_at,
            ended_at: None,
            num_processes: 0,
            num_nodes: 0,
            seed: None,
            extra: std::collections::HashMap::new(),
        }
    }

    /// Sets the name of the test.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the end time.
    pub fn with_end_time(mut self, ended_at: u64) -> Self {
        self.ended_at = Some(ended_at);
        self
    }

    /// Sets the number of processes.
    pub fn with_num_processes(mut self, num_processes: u32) -> Self {
        self.num_processes = num_processes;
        self
    }

    /// Sets the number of nodes.
    pub fn with_num_nodes(mut self, num_nodes: u32) -> Self {
        self.num_nodes = num_nodes;
        self
    }

    /// Sets the random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Adds extra metadata.
    pub fn with_extra(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extra.insert(key.into(), value);
        self
    }

    /// Returns the duration of the run in milliseconds, if ended.
    pub fn duration_ms(&self) -> Option<u64> {
        self.ended_at.map(|end| end.saturating_sub(self.started_at))
    }
}

impl Default for HistoryMetadata {
    fn default() -> Self {
        Self::new(0)
    }
}

/// A complete history of operations from a verification run.
///
/// The history captures all operations that were executed during a test,
/// along with metadata about the run. This is the primary input to
/// verification algorithms like linearizability checkers.
///
/// # Type Parameters
///
/// * `V` - The type of operation arguments stored in the history.
///
/// # Examples
///
/// ```
/// use mamut_core::history::{History, RunId, HistoryMetadata};
/// use mamut_core::operation::{Operation, OperationId, OperationPhase, OVC};
/// use mamut_core::node::ProcessId;
/// use serde_json::json;
///
/// let mut history = History::<serde_json::Value>::new(RunId::new());
/// history.metadata.name = Some("test_linearizability".to_string());
///
/// let op = Operation::new(
///     OperationId(1),
///     ProcessId(0),
///     "put",
///     json!({"key": "x", "value": 1}),
///     OVC::new(100),
/// );
/// history.push(op);
///
/// assert_eq!(history.len(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct History<V> {
    /// The unique identifier for this run.
    pub run_id: RunId,

    /// The operations recorded during the run.
    pub operations: Vec<Operation<V>>,

    /// Metadata about the run.
    pub metadata: HistoryMetadata,
}

impl<V> History<V> {
    /// Creates a new empty history for the given run.
    pub fn new(run_id: RunId) -> Self {
        Self {
            run_id,
            operations: Vec::new(),
            metadata: HistoryMetadata::default(),
        }
    }

    /// Creates a new history with the given metadata.
    pub fn with_metadata(run_id: RunId, metadata: HistoryMetadata) -> Self {
        Self {
            run_id,
            operations: Vec::new(),
            metadata,
        }
    }

    /// Returns the number of operations in the history.
    #[inline]
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    /// Returns true if the history contains no operations.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// Adds an operation to the history.
    pub fn push(&mut self, operation: Operation<V>) {
        self.operations.push(operation);
    }

    /// Returns an iterator over the operations.
    pub fn iter(&self) -> impl Iterator<Item = &Operation<V>> {
        self.operations.iter()
    }

    /// Returns a mutable iterator over the operations.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Operation<V>> {
        self.operations.iter_mut()
    }

    /// Returns operations filtered by process.
    pub fn operations_by_process(
        &self,
        process: crate::node::ProcessId,
    ) -> impl Iterator<Item = &Operation<V>> {
        self.operations.iter().filter(move |op| op.process == process)
    }

    /// Returns all unique process IDs in the history.
    pub fn processes(&self) -> Vec<crate::node::ProcessId> {
        let mut processes: Vec<_> = self
            .operations
            .iter()
            .map(|op| op.process)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        processes.sort();
        processes
    }

    /// Clears all operations from the history.
    pub fn clear(&mut self) {
        self.operations.clear();
    }

    /// Reserves capacity for at least `additional` more operations.
    pub fn reserve(&mut self, additional: usize) {
        self.operations.reserve(additional);
    }
}

impl<V> Default for History<V> {
    fn default() -> Self {
        Self::new(RunId::new())
    }
}

impl<V> IntoIterator for History<V> {
    type Item = Operation<V>;
    type IntoIter = std::vec::IntoIter<Operation<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.operations.into_iter()
    }
}

impl<'a, V> IntoIterator for &'a History<V> {
    type Item = &'a Operation<V>;
    type IntoIter = std::slice::Iter<'a, Operation<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.operations.iter()
    }
}

impl<V> std::ops::Index<usize> for History<V> {
    type Output = Operation<V>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.operations[index]
    }
}

impl<V> Extend<Operation<V>> for History<V> {
    fn extend<T: IntoIterator<Item = Operation<V>>>(&mut self, iter: T) {
        self.operations.extend(iter);
    }
}

/// A trait for storing and retrieving histories.
///
/// Implementations of this trait provide persistence for operation histories,
/// allowing them to be saved during test execution and loaded later for
/// analysis or replay.
///
/// # Type Parameters
///
/// * `Value` - The type of operation arguments that can be stored.
///
/// # Examples
///
/// ```ignore
/// use mamut_core::history::{HistoryStore, History, RunId};
/// use mamut_core::operation::Operation;
///
/// async fn example(store: impl HistoryStore<Value = serde_json::Value>) {
///     let run_id = RunId::new();
///
///     // Append operations during test
///     store.append(some_operation).await.unwrap();
///
///     // Flush to ensure durability
///     store.flush().await.unwrap();
///
///     // Load for verification
///     let history = store.load(run_id).await.unwrap();
/// }
/// ```
#[async_trait]
pub trait HistoryStore: Send + Sync {
    /// The type of operation arguments stored.
    type Value: Serialize + DeserializeOwned + Send + Sync;

    /// Appends an operation to the current run's history.
    ///
    /// Operations are typically buffered and written in batches for
    /// performance. Call `flush` to ensure all operations are persisted.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation could not be appended.
    async fn append(&self, op: Operation<Self::Value>) -> Result<(), HistoryError>;

    /// Loads a complete history for the given run.
    ///
    /// # Errors
    ///
    /// Returns `HistoryError::RunNotFound` if no history exists for the run.
    /// Returns other errors for I/O or deserialization failures.
    async fn load(&self, run_id: RunId) -> Result<History<Self::Value>, HistoryError>;

    /// Flushes any buffered operations to storage.
    ///
    /// This ensures all previously appended operations are durably stored.
    ///
    /// # Errors
    ///
    /// Returns an error if the flush operation fails.
    async fn flush(&self) -> Result<(), HistoryError>;

    /// Returns the current run ID.
    fn current_run_id(&self) -> RunId;

    /// Lists all available run IDs.
    ///
    /// # Errors
    ///
    /// Returns an error if the listing operation fails.
    async fn list_runs(&self) -> Result<Vec<RunId>, HistoryError> {
        // Default implementation returns empty list
        Ok(Vec::new())
    }

    /// Deletes the history for a given run.
    ///
    /// # Errors
    ///
    /// Returns an error if the deletion fails.
    async fn delete(&self, run_id: RunId) -> Result<(), HistoryError> {
        // Default implementation does nothing
        let _ = run_id;
        Ok(())
    }
}

/// An in-memory history store for testing.
///
/// This store keeps all operations in memory and is useful for unit tests
/// and scenarios where persistence is not required.
#[derive(Debug)]
pub struct InMemoryHistoryStore<V> {
    run_id: RunId,
    operations: std::sync::RwLock<Vec<Operation<V>>>,
    metadata: std::sync::RwLock<HistoryMetadata>,
    histories: std::sync::RwLock<std::collections::HashMap<RunId, History<V>>>,
}

impl<V> InMemoryHistoryStore<V>
where
    V: Clone,
{
    /// Creates a new in-memory store for the given run.
    pub fn new(run_id: RunId) -> Self {
        Self {
            run_id,
            operations: std::sync::RwLock::new(Vec::new()),
            metadata: std::sync::RwLock::new(HistoryMetadata::default()),
            histories: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Sets the metadata for the current run.
    pub fn set_metadata(&self, metadata: HistoryMetadata) {
        *self.metadata.write().unwrap() = metadata;
    }

    /// Returns the number of operations in the current run.
    pub fn len(&self) -> usize {
        self.operations.read().unwrap().len()
    }

    /// Returns true if the current run has no operations.
    pub fn is_empty(&self) -> bool {
        self.operations.read().unwrap().is_empty()
    }
}

impl<V> Default for InMemoryHistoryStore<V>
where
    V: Clone,
{
    fn default() -> Self {
        Self::new(RunId::new())
    }
}

#[async_trait]
impl<V> HistoryStore for InMemoryHistoryStore<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + 'static,
{
    type Value = V;

    async fn append(&self, op: Operation<V>) -> Result<(), HistoryError> {
        self.operations.write().unwrap().push(op);
        Ok(())
    }

    async fn load(&self, run_id: RunId) -> Result<History<V>, HistoryError> {
        if run_id == self.run_id {
            let operations = self.operations.read().unwrap().clone();
            let metadata = self.metadata.read().unwrap().clone();
            Ok(History {
                run_id,
                operations,
                metadata,
            })
        } else {
            self.histories
                .read()
                .unwrap()
                .get(&run_id)
                .cloned()
                .ok_or(HistoryError::RunNotFound(run_id))
        }
    }

    async fn flush(&self) -> Result<(), HistoryError> {
        // In-memory store doesn't need to flush
        Ok(())
    }

    fn current_run_id(&self) -> RunId {
        self.run_id
    }

    async fn list_runs(&self) -> Result<Vec<RunId>, HistoryError> {
        let mut runs: Vec<_> = self.histories.read().unwrap().keys().copied().collect();
        runs.push(self.run_id);
        Ok(runs)
    }

    async fn delete(&self, run_id: RunId) -> Result<(), HistoryError> {
        if run_id == self.run_id {
            self.operations.write().unwrap().clear();
        } else {
            self.histories.write().unwrap().remove(&run_id);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::ProcessId;
    use crate::operation::{OperationId, OperationPhase, OVC};
    use serde_json::json;

    #[test]
    fn test_run_id_creation() {
        let id1 = RunId::new();
        let id2 = RunId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_run_id_parse() {
        let original = RunId::new();
        let parsed = RunId::parse(&original.to_string()).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_run_id_display() {
        let id = RunId::new();
        let display = format!("{}", id);
        // UUID format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
        assert_eq!(display.len(), 36);
        assert!(display.contains('-'));
    }

    #[test]
    fn test_history_metadata_builder() {
        let metadata = HistoryMetadata::new(1000)
            .with_name("test")
            .with_end_time(2000)
            .with_num_processes(4)
            .with_num_nodes(2)
            .with_seed(42)
            .with_extra("version", json!("1.0"));

        assert_eq!(metadata.name, Some("test".to_string()));
        assert_eq!(metadata.started_at, 1000);
        assert_eq!(metadata.ended_at, Some(2000));
        assert_eq!(metadata.num_processes, 4);
        assert_eq!(metadata.num_nodes, 2);
        assert_eq!(metadata.seed, Some(42));
        assert_eq!(metadata.extra.get("version"), Some(&json!("1.0")));
        assert_eq!(metadata.duration_ms(), Some(1000));
    }

    #[test]
    fn test_history_creation() {
        let run_id = RunId::new();
        let history: History<serde_json::Value> = History::new(run_id);

        assert_eq!(history.run_id, run_id);
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn test_history_push_and_iterate() {
        let mut history: History<serde_json::Value> = History::new(RunId::new());

        for i in 0..3 {
            let op = Operation::new(
                OperationId(i),
                ProcessId(0),
                "test",
                json!(i),
                OVC::new(i),
            );
            history.push(op);
        }

        assert_eq!(history.len(), 3);

        let ids: Vec<_> = history.iter().map(|op| op.id.0).collect();
        assert_eq!(ids, vec![0, 1, 2]);
    }

    #[test]
    fn test_history_operations_by_process() {
        let mut history: History<serde_json::Value> = History::new(RunId::new());

        for i in 0..6 {
            let op = Operation::new(
                OperationId(i),
                ProcessId((i % 2) as u32),
                "test",
                json!(i),
                OVC::new(i),
            );
            history.push(op);
        }

        let p0_ops: Vec<_> = history.operations_by_process(ProcessId(0)).collect();
        assert_eq!(p0_ops.len(), 3);

        let p1_ops: Vec<_> = history.operations_by_process(ProcessId(1)).collect();
        assert_eq!(p1_ops.len(), 3);
    }

    #[test]
    fn test_history_processes() {
        let mut history: History<serde_json::Value> = History::new(RunId::new());

        for i in 0..6 {
            let op = Operation::new(
                OperationId(i),
                ProcessId((i % 3) as u32),
                "test",
                json!(i),
                OVC::new(i),
            );
            history.push(op);
        }

        let processes = history.processes();
        assert_eq!(processes, vec![ProcessId(0), ProcessId(1), ProcessId(2)]);
    }

    #[test]
    fn test_history_indexing() {
        let mut history: History<serde_json::Value> = History::new(RunId::new());

        let op = Operation::new(OperationId(42), ProcessId(0), "test", json!(null), OVC::new(0));
        history.push(op);

        assert_eq!(history[0].id, OperationId(42));
    }

    #[test]
    fn test_history_extend() {
        let mut history: History<serde_json::Value> = History::new(RunId::new());

        let ops = (0..3).map(|i| {
            Operation::new(OperationId(i), ProcessId(0), "test", json!(i), OVC::new(i))
        });

        history.extend(ops);
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn test_history_into_iter() {
        let mut history: History<serde_json::Value> = History::new(RunId::new());

        for i in 0..3 {
            let op = Operation::new(OperationId(i), ProcessId(0), "test", json!(i), OVC::new(i));
            history.push(op);
        }

        let ops: Vec<_> = history.into_iter().collect();
        assert_eq!(ops.len(), 3);
    }

    #[test]
    fn test_history_serialization() {
        let mut history: History<serde_json::Value> = History::new(RunId::new());
        history.metadata.name = Some("test".to_string());

        let op = Operation::new(
            OperationId(1),
            ProcessId(0),
            "put",
            json!({"key": "x"}),
            OVC::new(100),
        );
        history.push(op);

        let json = serde_json::to_string(&history).unwrap();
        let deserialized: History<serde_json::Value> = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.run_id, history.run_id);
        assert_eq!(deserialized.len(), 1);
        assert_eq!(deserialized.metadata.name, Some("test".to_string()));
    }

    #[tokio::test]
    async fn test_in_memory_store_append_and_load() {
        let run_id = RunId::new();
        let store = InMemoryHistoryStore::<serde_json::Value>::new(run_id);

        let op = Operation::new(
            OperationId(1),
            ProcessId(0),
            "put",
            json!({"key": "x"}),
            OVC::new(100),
        );

        store.append(op.clone()).await.unwrap();
        assert_eq!(store.len(), 1);

        let history = store.load(run_id).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, OperationId(1));
    }

    #[tokio::test]
    async fn test_in_memory_store_run_not_found() {
        let store = InMemoryHistoryStore::<serde_json::Value>::new(RunId::new());
        let other_run = RunId::new();

        let result = store.load(other_run).await;
        assert!(matches!(result, Err(HistoryError::RunNotFound(_))));
    }

    #[tokio::test]
    async fn test_in_memory_store_current_run_id() {
        let run_id = RunId::new();
        let store = InMemoryHistoryStore::<serde_json::Value>::new(run_id);

        assert_eq!(store.current_run_id(), run_id);
    }

    #[tokio::test]
    async fn test_in_memory_store_flush() {
        let store = InMemoryHistoryStore::<serde_json::Value>::new(RunId::new());

        // Flush should succeed (no-op for in-memory store)
        store.flush().await.unwrap();
    }

    #[tokio::test]
    async fn test_in_memory_store_delete() {
        let run_id = RunId::new();
        let store = InMemoryHistoryStore::<serde_json::Value>::new(run_id);

        let op = Operation::new(
            OperationId(1),
            ProcessId(0),
            "test",
            json!(null),
            OVC::new(0),
        );

        store.append(op).await.unwrap();
        assert!(!store.is_empty());

        store.delete(run_id).await.unwrap();
        assert!(store.is_empty());
    }
}
