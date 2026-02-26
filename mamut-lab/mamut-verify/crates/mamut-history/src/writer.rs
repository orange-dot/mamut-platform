//! Write coordinator with batching support for EventStoreDB.
//!
//! This module provides a buffered, batched write system with:
//! - Per-stream write buffers
//! - Configurable batch size, byte limits, and flush intervals
//! - Backpressure control via semaphore

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use eventstore::{AppendToStreamOptions, Client, EventData, ExpectedRevision};
use tokio::sync::{mpsc, Mutex, OwnedSemaphorePermit, RwLock, Semaphore};
use tokio::time::{interval, Instant};
use tracing::{debug, error, info, warn};

use crate::events::HistoryEvent;
use crate::wal::WriteAheadLog;
use crate::{HistoryError, Result};

/// Configuration for write batching
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum number of events per batch
    pub max_batch_size: usize,
    /// Maximum bytes per batch
    pub max_batch_bytes: usize,
    /// Maximum time to wait before flushing
    pub flush_interval: Duration,
    /// Maximum number of concurrent batches in flight
    pub max_in_flight_batches: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 1000,
            max_batch_bytes: 900 * 1024, // 900KB
            flush_interval: Duration::from_millis(100),
            max_in_flight_batches: 10,
        }
    }
}

/// A buffered event waiting to be written
#[derive(Debug)]
struct BufferedEvent {
    _event: HistoryEvent,
    event_data: EventData,
    size_bytes: usize,
    _buffered_at: Instant,
}

/// Write buffer for a single stream
#[derive(Debug)]
pub struct StreamWriteBuffer {
    stream_name: String,
    events: Vec<BufferedEvent>,
    total_bytes: usize,
    _created_at: Instant,
    last_flush: Instant,
}

impl StreamWriteBuffer {
    /// Create a new write buffer for a stream
    pub fn new(stream_name: String) -> Self {
        let now = Instant::now();
        Self {
            stream_name,
            events: Vec::new(),
            total_bytes: 0,
            _created_at: now,
            last_flush: now,
        }
    }

    /// Add an event to the buffer
    pub fn push(&mut self, event: HistoryEvent, event_data: EventData, size_bytes: usize) {
        self.events.push(BufferedEvent {
            _event: event,
            event_data,
            size_bytes,
            _buffered_at: Instant::now(),
        });
        self.total_bytes += size_bytes;
    }

    /// Check if the buffer should be flushed based on size
    pub fn should_flush_by_size(&self, config: &BatchConfig) -> bool {
        self.events.len() >= config.max_batch_size || self.total_bytes >= config.max_batch_bytes
    }

    /// Check if the buffer should be flushed based on time
    pub fn should_flush_by_time(&self, config: &BatchConfig) -> bool {
        !self.events.is_empty() && self.last_flush.elapsed() >= config.flush_interval
    }

    /// Take all events from the buffer
    pub fn take_events(&mut self) -> Vec<BufferedEvent> {
        self.total_bytes = 0;
        self.last_flush = Instant::now();
        std::mem::take(&mut self.events)
    }

    /// Get the number of buffered events
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Get total buffered bytes
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }
}

/// Metrics for the write coordinator
#[derive(Debug, Default)]
pub struct WriteMetrics {
    pub events_written: u64,
    pub batches_flushed: u64,
    pub bytes_written: u64,
    pub flush_errors: u64,
    pub backpressure_waits: u64,
}

/// Write coordinator that manages batched writes to EventStoreDB
pub struct WriteCoordinator {
    client: Client,
    config: BatchConfig,
    buffers: Arc<RwLock<HashMap<String, StreamWriteBuffer>>>,
    backpressure_semaphore: Arc<Semaphore>,
    wal: Option<Arc<Mutex<WriteAheadLog>>>,
    metrics: Arc<RwLock<WriteMetrics>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl WriteCoordinator {
    /// Create a new write coordinator
    pub fn new(client: Client, config: BatchConfig) -> Self {
        let max_permits = config.max_in_flight_batches;
        Self {
            client,
            config,
            buffers: Arc::new(RwLock::new(HashMap::new())),
            backpressure_semaphore: Arc::new(Semaphore::new(max_permits)),
            wal: None,
            metrics: Arc::new(RwLock::new(WriteMetrics::default())),
            shutdown_tx: None,
        }
    }

    /// Create a write coordinator with WAL support
    pub fn with_wal(client: Client, config: BatchConfig, wal: WriteAheadLog) -> Self {
        let max_permits = config.max_in_flight_batches;
        Self {
            client,
            config,
            buffers: Arc::new(RwLock::new(HashMap::new())),
            backpressure_semaphore: Arc::new(Semaphore::new(max_permits)),
            wal: Some(Arc::new(Mutex::new(wal))),
            metrics: Arc::new(RwLock::new(WriteMetrics::default())),
            shutdown_tx: None,
        }
    }

    /// Start the background flush task
    pub fn start_background_flush(&mut self) -> tokio::task::JoinHandle<()> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        let buffers = self.buffers.clone();
        let client = self.client.clone();
        let config = self.config.clone();
        let semaphore = self.backpressure_semaphore.clone();
        let metrics = self.metrics.clone();
        let wal = self.wal.clone();

        tokio::spawn(async move {
            let mut flush_interval = interval(config.flush_interval);

            loop {
                tokio::select! {
                    _ = flush_interval.tick() => {
                        Self::flush_expired_buffers(
                            &buffers,
                            &client,
                            &config,
                            &semaphore,
                            &metrics,
                            &wal,
                        ).await;
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Write coordinator shutting down, flushing remaining buffers");
                        Self::flush_all_buffers(
                            &buffers,
                            &client,
                            &config,
                            &semaphore,
                            &metrics,
                            &wal,
                        ).await;
                        break;
                    }
                }
            }
        })
    }

    /// Append an event to the buffer
    pub async fn append(&self, stream_name: &str, event: HistoryEvent) -> Result<()> {
        // Serialize the event
        let event_type = event.event_type().to_string();
        let json = serde_json::to_vec(&event)?;
        let size_bytes = json.len();

        let event_data = EventData::json(event_type, &event).map_err(|e| {
            HistoryError::SerializationError(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )))
        })?;

        // Write to WAL first if enabled
        if let Some(wal) = &self.wal {
            let mut wal = wal.lock().await;
            wal.append(stream_name, &event).await?;
        }

        // Add to buffer
        let mut buffers = self.buffers.write().await;
        let buffer = buffers
            .entry(stream_name.to_string())
            .or_insert_with(|| StreamWriteBuffer::new(stream_name.to_string()));

        buffer.push(event, event_data, size_bytes);

        // Check if we should flush immediately
        if buffer.should_flush_by_size(&self.config) {
            let stream_name = stream_name.to_string();
            drop(buffers);
            self.flush_stream(&stream_name).await?;
        }

        Ok(())
    }

    /// Append multiple events to the buffer
    pub async fn append_batch(&self, stream_name: &str, events: Vec<HistoryEvent>) -> Result<()> {
        for event in events {
            self.append(stream_name, event).await?;
        }
        Ok(())
    }

    /// Force flush a specific stream
    pub async fn flush_stream(&self, stream_name: &str) -> Result<()> {
        // Acquire backpressure permit
        let permit = self
            .acquire_permit()
            .await
            .ok_or(HistoryError::BackpressureLimitReached)?;

        let events = {
            let mut buffers = self.buffers.write().await;
            if let Some(buffer) = buffers.get_mut(stream_name) {
                buffer.take_events()
            } else {
                return Ok(());
            }
        };

        if events.is_empty() {
            return Ok(());
        }

        self.write_batch(stream_name, events, permit).await
    }

    /// Force flush all streams
    pub async fn flush_all(&self) -> Result<()> {
        let stream_names: Vec<String> = {
            let buffers = self.buffers.read().await;
            buffers.keys().cloned().collect()
        };

        for stream_name in stream_names {
            self.flush_stream(&stream_name).await?;
        }

        Ok(())
    }

    /// Get current metrics
    pub async fn metrics(&self) -> WriteMetrics {
        let metrics = self.metrics.read().await;
        WriteMetrics {
            events_written: metrics.events_written,
            batches_flushed: metrics.batches_flushed,
            bytes_written: metrics.bytes_written,
            flush_errors: metrics.flush_errors,
            backpressure_waits: metrics.backpressure_waits,
        }
    }

    /// Shutdown the coordinator gracefully
    pub async fn shutdown(mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        self.flush_all().await
    }

    /// Acquire a backpressure permit
    async fn acquire_permit(&self) -> Option<OwnedSemaphorePermit> {
        match self.backpressure_semaphore.clone().try_acquire_owned() {
            Ok(permit) => Some(permit),
            Err(_) => {
                // Track backpressure
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.backpressure_waits += 1;
                }

                // Wait for a permit with timeout
                match tokio::time::timeout(
                    Duration::from_secs(30),
                    self.backpressure_semaphore.clone().acquire_owned(),
                )
                .await
                {
                    Ok(Ok(permit)) => Some(permit),
                    _ => None,
                }
            }
        }
    }

    /// Write a batch of events to EventStoreDB
    async fn write_batch(
        &self,
        stream_name: &str,
        events: Vec<BufferedEvent>,
        _permit: OwnedSemaphorePermit,
    ) -> Result<()> {
        let event_count = events.len();
        let total_bytes: usize = events.iter().map(|e| e.size_bytes).sum();

        let event_data: Vec<EventData> = events.into_iter().map(|e| e.event_data).collect();

        let options = AppendToStreamOptions::default().expected_revision(ExpectedRevision::Any);

        match self
            .client
            .append_to_stream(stream_name, &options, event_data)
            .await
        {
            Ok(result) => {
                debug!(
                    stream = stream_name,
                    events = event_count,
                    bytes = total_bytes,
                    position = ?result.next_expected_version,
                    "Batch written successfully"
                );

                // Update metrics
                let mut metrics = self.metrics.write().await;
                metrics.events_written += event_count as u64;
                metrics.batches_flushed += 1;
                metrics.bytes_written += total_bytes as u64;

                // Mark WAL entries as committed if using WAL
                if let Some(wal) = &self.wal {
                    let mut wal = wal.lock().await;
                    wal.commit(stream_name, event_count as u64).await?;
                }

                Ok(())
            }
            Err(e) => {
                error!(
                    stream = stream_name,
                    events = event_count,
                    error = %e,
                    "Failed to write batch"
                );

                let mut metrics = self.metrics.write().await;
                metrics.flush_errors += 1;

                Err(HistoryError::WriteError(e.to_string()))
            }
        }
    }

    /// Flush buffers that have exceeded the time threshold
    async fn flush_expired_buffers(
        buffers: &Arc<RwLock<HashMap<String, StreamWriteBuffer>>>,
        client: &Client,
        config: &BatchConfig,
        semaphore: &Arc<Semaphore>,
        metrics: &Arc<RwLock<WriteMetrics>>,
        wal: &Option<Arc<Mutex<WriteAheadLog>>>,
    ) {
        let streams_to_flush: Vec<String> = {
            let buffers = buffers.read().await;
            buffers
                .iter()
                .filter(|(_, buffer)| buffer.should_flush_by_time(config))
                .map(|(name, _)| name.clone())
                .collect()
        };

        for stream_name in streams_to_flush {
            let permit = match semaphore.clone().try_acquire_owned() {
                Ok(permit) => permit,
                Err(_) => {
                    debug!(stream = stream_name, "Skipping flush due to backpressure");
                    continue;
                }
            };

            let events = {
                let mut buffers = buffers.write().await;
                if let Some(buffer) = buffers.get_mut(&stream_name) {
                    buffer.take_events()
                } else {
                    continue;
                }
            };

            if events.is_empty() {
                continue;
            }

            let event_count = events.len();
            let total_bytes: usize = events.iter().map(|e| e.size_bytes).sum();
            let event_data: Vec<EventData> = events.into_iter().map(|e| e.event_data).collect();

            let options = AppendToStreamOptions::default().expected_revision(ExpectedRevision::Any);

            match client
                .append_to_stream(&stream_name, &options, event_data)
                .await
            {
                Ok(result) => {
                    debug!(
                        stream = stream_name,
                        events = event_count,
                        bytes = total_bytes,
                        position = ?result.next_expected_version,
                        "Timed flush completed"
                    );

                    let mut metrics = metrics.write().await;
                    metrics.events_written += event_count as u64;
                    metrics.batches_flushed += 1;
                    metrics.bytes_written += total_bytes as u64;

                    if let Some(wal) = wal {
                        let mut wal = wal.lock().await;
                        let _ = wal.commit(&stream_name, event_count as u64).await;
                    }
                }
                Err(e) => {
                    warn!(
                        stream = stream_name,
                        error = %e,
                        "Timed flush failed"
                    );

                    let mut metrics = metrics.write().await;
                    metrics.flush_errors += 1;
                }
            }

            drop(permit);
        }
    }

    /// Flush all buffers (used during shutdown)
    async fn flush_all_buffers(
        buffers: &Arc<RwLock<HashMap<String, StreamWriteBuffer>>>,
        client: &Client,
        _config: &BatchConfig,
        semaphore: &Arc<Semaphore>,
        metrics: &Arc<RwLock<WriteMetrics>>,
        wal: &Option<Arc<Mutex<WriteAheadLog>>>,
    ) {
        let stream_names: Vec<String> = {
            let buffers = buffers.read().await;
            buffers.keys().cloned().collect()
        };

        for stream_name in stream_names {
            // Wait for permit during shutdown
            let permit = match semaphore.clone().acquire_owned().await {
                Ok(permit) => permit,
                Err(_) => continue,
            };

            let events = {
                let mut buffers = buffers.write().await;
                if let Some(buffer) = buffers.get_mut(&stream_name) {
                    buffer.take_events()
                } else {
                    continue;
                }
            };

            if events.is_empty() {
                continue;
            }

            let event_count = events.len();
            let total_bytes: usize = events.iter().map(|e| e.size_bytes).sum();
            let event_data: Vec<EventData> = events.into_iter().map(|e| e.event_data).collect();

            let options = AppendToStreamOptions::default().expected_revision(ExpectedRevision::Any);

            match client
                .append_to_stream(&stream_name, &options, event_data)
                .await
            {
                Ok(_) => {
                    info!(
                        stream = stream_name,
                        events = event_count,
                        bytes = total_bytes,
                        "Shutdown flush completed"
                    );

                    let mut metrics = metrics.write().await;
                    metrics.events_written += event_count as u64;
                    metrics.batches_flushed += 1;
                    metrics.bytes_written += total_bytes as u64;

                    if let Some(wal) = wal {
                        let mut wal = wal.lock().await;
                        let _ = wal.commit(&stream_name, event_count as u64).await;
                    }
                }
                Err(e) => {
                    error!(
                        stream = stream_name,
                        error = %e,
                        "Shutdown flush failed"
                    );

                    let mut metrics = metrics.write().await;
                    metrics.flush_errors += 1;
                }
            }

            drop(permit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_config_default() {
        let config = BatchConfig::default();
        assert_eq!(config.max_batch_size, 1000);
        assert_eq!(config.max_batch_bytes, 900 * 1024);
        assert_eq!(config.flush_interval, Duration::from_millis(100));
        assert_eq!(config.max_in_flight_batches, 10);
    }

    #[test]
    fn test_stream_write_buffer() {
        let buffer = StreamWriteBuffer::new("test-stream".to_string());
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.total_bytes(), 0);
    }
}
