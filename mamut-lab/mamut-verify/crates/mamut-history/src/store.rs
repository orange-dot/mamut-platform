//! EventStoreDB-backed history store implementation.
//!
//! This module provides the main entry point for the history persistence layer,
//! combining the writer, reader, checkpoint manager, and index into a unified
//! interface for storing and querying test execution history.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use eventstore::{Client, ClientSettings};
use futures::StreamExt;
use mamut_core::{OperationId, ProcessId, RunId};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::checkpoint::{Checkpoint, CheckpointManager, FileCheckpointStorage};
use crate::events::{CheckEvent, FaultEvent, HistoryEvent, OperationEvent, OperationResult, OperationType, TestRunEvent};
use crate::index::HistoryIndex;
use crate::reader::{ChunkSpec, EventStream, HistoryReader, ReadOptions, ResolvedEvent};
use crate::wal::WriteAheadLog;
use crate::writer::{BatchConfig, WriteCoordinator, WriteMetrics};
use crate::{HistoryError, Result};

/// Configuration for connecting to EventStoreDB
#[derive(Debug, Clone)]
pub struct EventStoreConfig {
    /// Connection string for EventStoreDB
    pub connection_string: String,
    /// Whether to use TLS
    pub use_tls: bool,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Operation timeout
    pub operation_timeout: Duration,
    /// Batch configuration for writes
    pub batch_config: BatchConfig,
    /// Path for WAL storage (optional)
    pub wal_path: Option<String>,
    /// Path for checkpoint storage
    pub checkpoint_path: Option<String>,
    /// Whether to enable the in-memory index
    pub enable_index: bool,
}

impl Default for EventStoreConfig {
    fn default() -> Self {
        Self {
            connection_string: "esdb://localhost:2113?tls=false".to_string(),
            use_tls: false,
            connection_timeout: Duration::from_secs(10),
            operation_timeout: Duration::from_secs(30),
            batch_config: BatchConfig::default(),
            wal_path: None,
            checkpoint_path: None,
            enable_index: true,
        }
    }
}

impl EventStoreConfig {
    /// Create a config for a local development instance
    pub fn local() -> Self {
        Self::default()
    }

    /// Create a config for a secure connection
    pub fn secure(connection_string: impl Into<String>) -> Self {
        Self {
            connection_string: connection_string.into(),
            use_tls: true,
            ..Default::default()
        }
    }

    /// Set the WAL path
    pub fn with_wal_path(mut self, path: impl Into<String>) -> Self {
        self.wal_path = Some(path.into());
        self
    }

    /// Set the checkpoint path
    pub fn with_checkpoint_path(mut self, path: impl Into<String>) -> Self {
        self.checkpoint_path = Some(path.into());
        self
    }

    /// Enable or disable the in-memory index
    pub fn with_index(mut self, enable: bool) -> Self {
        self.enable_index = enable;
        self
    }

    /// Set the batch configuration
    pub fn with_batch_config(mut self, config: BatchConfig) -> Self {
        self.batch_config = config;
        self
    }
}

/// Stream naming conventions for different event types
pub struct StreamNames;

impl StreamNames {
    /// Stream for test run events
    pub fn test_run(run_id: &RunId) -> String {
        format!("test-run-{}", run_id)
    }

    /// Stream for all operations in a test
    pub fn operations(run_id: &RunId) -> String {
        format!("operations-{}", run_id)
    }

    /// Stream for operations by a specific process
    pub fn process_operations(run_id: &RunId, process_id: &ProcessId) -> String {
        format!("process-{}-{}", run_id, process_id.0)
    }

    /// Stream for fault events
    pub fn faults(run_id: &RunId) -> String {
        format!("faults-{}", run_id)
    }

    /// Stream for check events
    pub fn checks(run_id: &RunId) -> String {
        format!("checks-{}", run_id)
    }

    /// Global checkpoints stream
    pub fn checkpoints() -> String {
        "checkpoints".to_string()
    }

    /// Category projection for all test runs
    pub fn all_test_runs() -> String {
        "$ce-test-run".to_string()
    }
}

/// The main history store backed by EventStoreDB
pub struct HistoryStore {
    _client: Client,
    writer: WriteCoordinator,
    reader: HistoryReader,
    index: Option<Arc<RwLock<HistoryIndex>>>,
    checkpoint_manager: Option<CheckpointManager>,
    _config: EventStoreConfig,
    /// Handle to the background flush task
    flush_handle: Option<tokio::task::JoinHandle<()>>,
}

impl HistoryStore {
    /// Connect to EventStoreDB and create a new history store
    pub async fn connect(config: EventStoreConfig) -> Result<Self> {
        info!(
            connection = config.connection_string,
            "Connecting to EventStoreDB"
        );

        // Parse connection settings
        let settings: ClientSettings = config
            .connection_string
            .parse()
            .map_err(|e| HistoryError::ConnectionError(format!("Invalid connection string: {}", e)))?;

        // Create client
        let client = Client::new(settings)
            .map_err(|e| HistoryError::ConnectionError(e.to_string()))?;

        // Create writer with optional WAL
        let writer = if let Some(wal_path) = &config.wal_path {
            let wal = WriteAheadLog::new(wal_path).await?;
            WriteCoordinator::with_wal(client.clone(), config.batch_config.clone(), wal)
        } else {
            WriteCoordinator::new(client.clone(), config.batch_config.clone())
        };

        // Create index if enabled
        let index = if config.enable_index {
            Some(Arc::new(RwLock::new(HistoryIndex::new())))
        } else {
            None
        };

        // Create reader
        let reader = if let Some(idx) = &index {
            HistoryReader::with_index(client.clone(), idx.clone())
        } else {
            HistoryReader::new(client.clone())
        };

        // Create checkpoint manager if path is provided
        let checkpoint_manager = if let Some(path) = &config.checkpoint_path {
            let storage = FileCheckpointStorage::new(path).await?;
            Some(CheckpointManager::new(Arc::new(storage)))
        } else {
            None
        };

        let mut store = Self {
            _client: client,
            writer,
            reader,
            index,
            checkpoint_manager,
            _config: config,
            flush_handle: None,
        };

        // Start background flush task
        store.flush_handle = Some(store.writer.start_background_flush());

        info!("Connected to EventStoreDB");
        Ok(store)
    }

    /// Append a test run event
    pub async fn append_test_event(&self, run_id: &RunId, event: TestRunEvent) -> Result<()> {
        let stream_name = StreamNames::test_run(run_id);
        let history_event = HistoryEvent::TestRun(event);

        self.writer
            .append(&stream_name, history_event.clone())
            .await?;

        // Update index if enabled
        if let Some(index) = &self.index {
            let mut index = index.write().await;
            // We don't have the exact position here, but we can use a placeholder
            // In a real implementation, we'd get the position from the write result
            index.index_event(&history_event, 0);
        }

        Ok(())
    }

    /// Append an operation event
    pub async fn append_operation_event(
        &self,
        run_id: &RunId,
        event: OperationEvent,
    ) -> Result<()> {
        let stream_name = StreamNames::operations(run_id);
        let history_event = HistoryEvent::Operation(event.clone());

        self.writer
            .append(&stream_name, history_event.clone())
            .await?;

        // Also append to process-specific stream
        let process_stream = StreamNames::process_operations(run_id, &event.process_id());
        self.writer
            .append(&process_stream, history_event.clone())
            .await?;

        // Update index if enabled
        if let Some(index) = &self.index {
            let mut index = index.write().await;
            index.index_event(&history_event, 0);
        }

        Ok(())
    }

    /// Append a fault event
    pub async fn append_fault_event(&self, run_id: &RunId, event: FaultEvent) -> Result<()> {
        let stream_name = StreamNames::faults(run_id);
        let history_event = HistoryEvent::Fault(event);

        self.writer
            .append(&stream_name, history_event.clone())
            .await?;

        if let Some(index) = &self.index {
            let mut index = index.write().await;
            index.index_event(&history_event, 0);
        }

        Ok(())
    }

    /// Append a check event
    pub async fn append_check_event(&self, run_id: &RunId, event: CheckEvent) -> Result<()> {
        let stream_name = StreamNames::checks(run_id);
        let history_event = HistoryEvent::Check(event);

        self.writer
            .append(&stream_name, history_event.clone())
            .await?;

        if let Some(index) = &self.index {
            let mut index = index.write().await;
            index.index_event(&history_event, 0);
        }

        Ok(())
    }

    /// Append multiple events in a batch
    pub async fn append_batch(&self, run_id: &RunId, events: Vec<HistoryEvent>) -> Result<()> {
        for event in events {
            match &event {
                HistoryEvent::TestRun(_) => {
                    let stream_name = StreamNames::test_run(run_id);
                    self.writer.append(&stream_name, event.clone()).await?;
                }
                HistoryEvent::Operation(e) => {
                    let stream_name = StreamNames::operations(run_id);
                    self.writer.append(&stream_name, event.clone()).await?;

                    let process_stream = StreamNames::process_operations(run_id, &e.process_id());
                    self.writer.append(&process_stream, event.clone()).await?;
                }
                HistoryEvent::Fault(_) => {
                    let stream_name = StreamNames::faults(run_id);
                    self.writer.append(&stream_name, event.clone()).await?;
                }
                HistoryEvent::Check(_) => {
                    let stream_name = StreamNames::checks(run_id);
                    self.writer.append(&stream_name, event.clone()).await?;
                }
            }

            if let Some(index) = &self.index {
                let mut index = index.write().await;
                index.index_event(&event, 0);
            }
        }

        Ok(())
    }

    /// Read test run events
    pub fn read_test_events(&self, run_id: &RunId) -> EventStream {
        let stream_name = StreamNames::test_run(run_id);
        self.reader.read_all(&stream_name)
    }

    /// Read all operations for a test
    pub fn read_operations(&self, run_id: &RunId) -> EventStream {
        let stream_name = StreamNames::operations(run_id);
        self.reader.read_all(&stream_name)
    }

    /// Read operations with options
    pub fn read_operations_with_options(
        &self,
        run_id: &RunId,
        options: ReadOptions,
    ) -> EventStream {
        let stream_name = StreamNames::operations(run_id);
        self.reader.read_stream(&stream_name, options)
    }

    /// Read operations for a specific process
    pub fn read_process_operations(&self, run_id: &RunId, process_id: &ProcessId) -> EventStream {
        let stream_name = StreamNames::process_operations(run_id, process_id);
        self.reader.read_all(&stream_name)
    }

    /// Read fault events
    pub fn read_faults(&self, run_id: &RunId) -> EventStream {
        let stream_name = StreamNames::faults(run_id);
        self.reader.read_all(&stream_name)
    }

    /// Read check events
    pub fn read_checks(&self, run_id: &RunId) -> EventStream {
        let stream_name = StreamNames::checks(run_id);
        self.reader.read_all(&stream_name)
    }

    /// Read operations in a time range
    pub fn read_operations_in_time_range(
        &self,
        run_id: &RunId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> EventStream {
        let stream_name = StreamNames::operations(run_id);
        self.reader.read_time_range(&stream_name, from, to)
    }

    /// Calculate chunks for parallel reading
    pub async fn calculate_operation_chunks(
        &self,
        run_id: &RunId,
        chunk_size: u64,
    ) -> Result<Vec<ChunkSpec>> {
        let stream_name = StreamNames::operations(run_id);
        self.reader.calculate_chunks(&stream_name, chunk_size).await
    }

    /// Read a chunk of operations
    pub fn read_operation_chunk(&self, chunk: &ChunkSpec) -> EventStream {
        self.reader.read_chunk(chunk)
    }

    /// Read multiple chunks in parallel
    pub async fn read_operation_chunks_parallel(
        &self,
        chunks: Vec<ChunkSpec>,
        max_parallelism: usize,
    ) -> Result<Vec<Vec<ResolvedEvent>>> {
        self.reader
            .read_chunks_parallel(chunks, max_parallelism)
            .await
    }

    /// Create a checkpoint
    pub async fn create_checkpoint(&self, name: &str) -> Result<Option<Checkpoint>> {
        if let Some(manager) = &self.checkpoint_manager {
            let checkpoint = manager.create_checkpoint(name).await?;
            Ok(Some(checkpoint))
        } else {
            Ok(None)
        }
    }

    /// Restore from a checkpoint
    pub async fn restore_checkpoint(&self, checkpoint: &Checkpoint) -> Result<()> {
        if let Some(manager) = &self.checkpoint_manager {
            manager.restore(checkpoint).await?;
        }
        Ok(())
    }

    /// Restore from the latest checkpoint
    pub async fn restore_latest_checkpoint(&self) -> Result<Option<Checkpoint>> {
        if let Some(manager) = &self.checkpoint_manager {
            manager.restore_latest().await
        } else {
            Ok(None)
        }
    }

    /// Get the in-memory index
    pub fn index(&self) -> Option<Arc<RwLock<HistoryIndex>>> {
        self.index.clone()
    }

    /// Build index from existing streams
    pub async fn build_index(&self, run_id: &RunId) -> Result<()> {
        if let Some(index) = &self.index {
            let stream_name = StreamNames::operations(run_id);
            let mut stream = self.reader.read_all(&stream_name);
            let mut index = index.write().await;

            while let Some(result) = stream.next().await {
                match result {
                    Ok(resolved) => {
                        index.index_event(&resolved.event, resolved.position);
                    }
                    Err(e) => {
                        warn!(error = %e, "Error reading event during index build");
                    }
                }
            }

            info!(
                run_id = %run_id,
                operations = index.len(),
                "Index built for test run"
            );
        }

        Ok(())
    }

    /// Get write metrics
    pub async fn write_metrics(&self) -> WriteMetrics {
        self.writer.metrics().await
    }

    /// Flush all pending writes
    pub async fn flush(&self) -> Result<()> {
        self.writer.flush_all().await
    }

    /// Get the number of operations in a test
    pub async fn get_operation_count(&self, run_id: &RunId) -> Result<u64> {
        let stream_name = StreamNames::operations(run_id);
        self.reader.get_stream_length(&stream_name).await
    }

    /// Check if a test run exists
    pub async fn test_run_exists(&self, run_id: &RunId) -> Result<bool> {
        let stream_name = StreamNames::test_run(run_id);
        self.reader.stream_exists(&stream_name).await
    }

    /// Gracefully shutdown the store
    pub async fn shutdown(mut self) -> Result<()> {
        info!("Shutting down history store");

        // Cancel background task
        if let Some(handle) = self.flush_handle.take() {
            handle.abort();
        }

        // Flush remaining writes
        self.writer.flush_all().await?;

        info!("History store shutdown complete");
        Ok(())
    }
}

/// Helper struct for recording operations during a test
pub struct OperationRecorder {
    store: Arc<HistoryStore>,
    run_id: RunId,
}

impl OperationRecorder {
    /// Create a new operation recorder
    pub fn new(store: Arc<HistoryStore>, run_id: RunId) -> Self {
        Self { store, run_id }
    }

    /// Record an operation invocation
    pub async fn record_invocation(
        &self,
        process_id: ProcessId,
        operation_type: OperationType,
        arguments: Vec<serde_json::Value>,
    ) -> Result<OperationId> {
        let operation_id = OperationId::new(rand::random());
        let event = OperationEvent::Invoked {
            operation_id,
            process_id,
            timestamp: Utc::now(),
            operation_type,
            arguments,
        };

        self.store
            .append_operation_event(&self.run_id, event)
            .await?;

        Ok(operation_id)
    }

    /// Record an operation return
    pub async fn record_return(
        &self,
        operation_id: OperationId,
        process_id: ProcessId,
        result: OperationResult,
        duration_ns: u64,
    ) -> Result<()> {
        let event = OperationEvent::Returned {
            operation_id,
            process_id,
            timestamp: Utc::now(),
            result,
            duration_ns,
        };

        self.store
            .append_operation_event(&self.run_id, event)
            .await
    }

    /// Record an operation failure
    pub async fn record_failure(
        &self,
        operation_id: OperationId,
        process_id: ProcessId,
        error: String,
        retryable: bool,
    ) -> Result<()> {
        let event = OperationEvent::Failed {
            operation_id,
            process_id,
            timestamp: Utc::now(),
            error,
            retryable,
        };

        self.store
            .append_operation_event(&self.run_id, event)
            .await
    }

    /// Record an operation timeout
    pub async fn record_timeout(
        &self,
        operation_id: OperationId,
        process_id: ProcessId,
        timeout_ms: u64,
    ) -> Result<()> {
        let event = OperationEvent::TimedOut {
            operation_id,
            process_id,
            timestamp: Utc::now(),
            timeout_ms,
        };

        self.store
            .append_operation_event(&self.run_id, event)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_names() {
        let run_id = RunId::new();
        let process_id = ProcessId(1);

        assert!(StreamNames::test_run(&run_id).starts_with("test-run-"));
        assert!(StreamNames::operations(&run_id).starts_with("operations-"));
        assert!(StreamNames::process_operations(&run_id, &process_id).contains("-1"));
        assert!(StreamNames::faults(&run_id).starts_with("faults-"));
        assert!(StreamNames::checks(&run_id).starts_with("checks-"));
    }

    #[test]
    fn test_event_store_config_default() {
        let config = EventStoreConfig::default();
        assert!(!config.use_tls);
        assert!(config.enable_index);
        assert!(config.wal_path.is_none());
    }

    #[test]
    fn test_event_store_config_builder() {
        let config = EventStoreConfig::default()
            .with_wal_path("/tmp/wal")
            .with_checkpoint_path("/tmp/checkpoints")
            .with_index(false);

        assert_eq!(config.wal_path, Some("/tmp/wal".to_string()));
        assert_eq!(config.checkpoint_path, Some("/tmp/checkpoints".to_string()));
        assert!(!config.enable_index);
    }
}
