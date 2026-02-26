//! History reader with streaming and chunked reading support.
//!
//! This module provides efficient reading capabilities:
//! - Streaming read with lazy loading
//! - Chunked reading for parallel analysis
//! - Time range queries
//! - Position-based seeking

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use chrono::{DateTime, Utc};
use eventstore::{Client, ReadStreamOptions, StreamPosition as EsStreamPosition};
use futures::stream::{Stream, StreamExt};
use mamut_core::{OperationId, ProcessId};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::warn;

use crate::events::HistoryEvent;
use crate::index::HistoryIndex;
use crate::{HistoryError, Result};

/// Position in a stream
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamPosition {
    /// Start of the stream
    Start,
    /// End of the stream
    End,
    /// Specific position
    Position(u64),
}

impl From<StreamPosition> for EsStreamPosition {
    fn from(pos: StreamPosition) -> Self {
        match pos {
            StreamPosition::Start => EsStreamPosition::Start,
            StreamPosition::End => EsStreamPosition::End,
            StreamPosition::Position(p) => EsStreamPosition::Position(p),
        }
    }
}

/// Options for reading from a stream
#[derive(Debug, Clone)]
pub struct ReadOptions {
    /// Starting position
    pub from: StreamPosition,
    /// Maximum number of events to read
    pub max_count: Option<u64>,
    /// Whether to read forwards or backwards
    pub direction: ReadDirection,
    /// Whether to resolve link events
    pub resolve_links: bool,
    /// Filter by time range (start)
    pub time_from: Option<DateTime<Utc>>,
    /// Filter by time range (end)
    pub time_to: Option<DateTime<Utc>>,
    /// Filter by event types
    pub event_types: Option<Vec<String>>,
}

impl Default for ReadOptions {
    fn default() -> Self {
        Self {
            from: StreamPosition::Start,
            max_count: None,
            direction: ReadDirection::Forward,
            resolve_links: true,
            time_from: None,
            time_to: None,
            event_types: None,
        }
    }
}

/// Read direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadDirection {
    Forward,
    Backward,
}

/// Specification for a chunk of data to read
#[derive(Debug, Clone)]
pub struct ChunkSpec {
    /// Stream name
    pub stream_name: String,
    /// Starting position (inclusive)
    pub start_position: u64,
    /// Ending position (exclusive)
    pub end_position: u64,
    /// Chunk identifier
    pub chunk_id: usize,
}

impl ChunkSpec {
    /// Calculate the size of this chunk
    pub fn size(&self) -> u64 {
        self.end_position.saturating_sub(self.start_position)
    }
}

/// A resolved event with metadata
#[derive(Debug, Clone)]
pub struct ResolvedEvent {
    /// The event data
    pub event: HistoryEvent,
    /// Stream position
    pub position: u64,
    /// Event ID
    pub event_id: uuid::Uuid,
    /// Original event type
    pub event_type: String,
    /// Event timestamp from EventStoreDB
    pub created: DateTime<Utc>,
    /// Stream name
    pub stream_name: String,
}

/// Streaming event iterator
pub struct EventStream {
    inner: Pin<Box<dyn Stream<Item = Result<ResolvedEvent>> + Send>>,
}

impl EventStream {
    /// Create a new event stream
    pub fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<ResolvedEvent>> + Send + 'static,
    {
        Self {
            inner: Box::pin(stream),
        }
    }

    /// Collect all events into a vector
    pub async fn collect_all(mut self) -> Result<Vec<ResolvedEvent>> {
        let mut events = Vec::new();
        while let Some(result) = self.inner.next().await {
            events.push(result?);
        }
        Ok(events)
    }

    /// Take the first n events
    pub async fn take(mut self, n: usize) -> Result<Vec<ResolvedEvent>> {
        let mut events = Vec::with_capacity(n);
        let mut count = 0;
        while let Some(result) = self.inner.next().await {
            if count >= n {
                break;
            }
            events.push(result?);
            count += 1;
        }
        Ok(events)
    }

    /// Skip the first n events and return the rest
    pub async fn skip(mut self, n: usize) -> Self {
        let mut count = 0;
        let remaining = async_stream::stream! {
            while let Some(result) = self.inner.next().await {
                if count < n {
                    count += 1;
                    continue;
                }
                yield result;
            }
        };
        Self::new(remaining)
    }
}

impl Stream for EventStream {
    type Item = Result<ResolvedEvent>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

/// History reader for EventStoreDB
pub struct HistoryReader {
    client: Client,
    index: Option<Arc<RwLock<HistoryIndex>>>,
    default_batch_size: usize,
}

impl HistoryReader {
    /// Create a new history reader
    pub fn new(client: Client) -> Self {
        Self {
            client,
            index: None,
            default_batch_size: 1000,
        }
    }

    /// Create a history reader with an index for efficient queries
    pub fn with_index(client: Client, index: Arc<RwLock<HistoryIndex>>) -> Self {
        Self {
            client,
            index: Some(index),
            default_batch_size: 1000,
        }
    }

    /// Set the default batch size for reading
    pub fn set_batch_size(&mut self, size: usize) {
        self.default_batch_size = size;
    }

    /// Read events from a stream with options
    pub fn read_stream(&self, stream_name: &str, options: ReadOptions) -> EventStream {
        let client = self.client.clone();
        let stream_name = stream_name.to_string();
        let _batch_size = self.default_batch_size;

        let stream = async_stream::stream! {
            let read_options = ReadStreamOptions::default()
                .resolve_link_tos();

            let position = match options.from {
                StreamPosition::Start => EsStreamPosition::Start,
                StreamPosition::End => EsStreamPosition::End,
                StreamPosition::Position(p) => EsStreamPosition::Position(p),
            };

            let read_result = match options.direction {
                ReadDirection::Forward => {
                    client.read_stream(&stream_name, &read_options.position(position)).await
                }
                ReadDirection::Backward => {
                    client.read_stream(&stream_name, &read_options.position(position).backwards()).await
                }
            };

            let mut stream = match read_result {
                Ok(stream) => stream,
                Err(e) => {
                    yield Err(HistoryError::ReadError(e.to_string()));
                    return;
                }
            };

            let mut count = 0u64;
            let max_count = options.max_count.unwrap_or(u64::MAX);

            while let Some(event_result) = stream.next().await {
                if count >= max_count {
                    break;
                }

                match event_result {
                    Ok(resolved_event) => {
                        let event = resolved_event.get_original_event();

                        // Deserialize the event
                        let history_event: HistoryEvent = match event.as_json() {
                            Ok(e) => e,
                            Err(e) => {
                                warn!(
                                    stream = stream_name,
                                    position = ?event.revision,
                                    error = %e,
                                    "Failed to deserialize event"
                                );
                                continue;
                            }
                        };

                        // Apply time filter if specified
                        let event_time = history_event.timestamp();
                        if let Some(from) = options.time_from {
                            if event_time < from {
                                continue;
                            }
                        }
                        if let Some(to) = options.time_to {
                            if event_time > to {
                                continue;
                            }
                        }

                        // Apply event type filter if specified
                        if let Some(ref types) = options.event_types {
                            if !types.contains(&history_event.event_type().to_string()) {
                                continue;
                            }
                        }

                        let resolved = ResolvedEvent {
                            event: history_event,
                            position: event.revision,
                            event_id: event.id,
                            event_type: event.event_type.to_string(),
                            created: event.created.unwrap_or_else(Utc::now),
                            stream_name: stream_name.clone(),
                        };

                        count += 1;
                        yield Ok(resolved);
                    }
                    Err(e) => {
                        yield Err(HistoryError::ReadError(e.to_string()));
                        return;
                    }
                }
            }
        };

        EventStream::new(stream)
    }

    /// Read all events from a stream
    pub fn read_all(&self, stream_name: &str) -> EventStream {
        self.read_stream(stream_name, ReadOptions::default())
    }

    /// Read events in a time range
    pub fn read_time_range(
        &self,
        stream_name: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> EventStream {
        let options = ReadOptions {
            time_from: Some(from),
            time_to: Some(to),
            ..Default::default()
        };
        self.read_stream(stream_name, options)
    }

    /// Read events starting from a position
    pub fn read_from_position(&self, stream_name: &str, position: u64) -> EventStream {
        let options = ReadOptions {
            from: StreamPosition::Position(position),
            ..Default::default()
        };
        self.read_stream(stream_name, options)
    }

    /// Read the last n events from a stream
    pub fn read_last(&self, stream_name: &str, count: u64) -> EventStream {
        let options = ReadOptions {
            from: StreamPosition::End,
            max_count: Some(count),
            direction: ReadDirection::Backward,
            ..Default::default()
        };
        self.read_stream(stream_name, options)
    }

    /// Calculate chunk specifications for parallel reading
    pub async fn calculate_chunks(
        &self,
        stream_name: &str,
        chunk_size: u64,
    ) -> Result<Vec<ChunkSpec>> {
        // Get stream length
        let stream_length = self.get_stream_length(stream_name).await?;

        if stream_length == 0 {
            return Ok(Vec::new());
        }

        let num_chunks = (stream_length + chunk_size - 1) / chunk_size;
        let mut chunks = Vec::with_capacity(num_chunks as usize);

        for i in 0..num_chunks {
            let start = i * chunk_size;
            let end = ((i + 1) * chunk_size).min(stream_length);

            chunks.push(ChunkSpec {
                stream_name: stream_name.to_string(),
                start_position: start,
                end_position: end,
                chunk_id: i as usize,
            });
        }

        Ok(chunks)
    }

    /// Read a specific chunk
    pub fn read_chunk(&self, chunk: &ChunkSpec) -> EventStream {
        let options = ReadOptions {
            from: StreamPosition::Position(chunk.start_position),
            max_count: Some(chunk.size()),
            ..Default::default()
        };
        self.read_stream(&chunk.stream_name, options)
    }

    /// Read multiple chunks in parallel
    pub async fn read_chunks_parallel(
        &self,
        chunks: Vec<ChunkSpec>,
        max_parallelism: usize,
    ) -> Result<Vec<Vec<ResolvedEvent>>> {
        use futures::stream::FuturesUnordered;

        let mut results: Vec<Option<Vec<ResolvedEvent>>> = vec![None; chunks.len()];
        let mut futures = FuturesUnordered::new();
        let mut chunk_iter = chunks.into_iter().enumerate();
        let mut active = 0;

        // Start initial batch
        for _ in 0..max_parallelism {
            if let Some((idx, chunk)) = chunk_iter.next() {
                let stream = self.read_chunk(&chunk);
                futures.push(async move { (idx, stream.collect_all().await) });
                active += 1;
            }
        }

        // Process results and start new tasks
        while active > 0 {
            if let Some((idx, result)) = futures.next().await {
                results[idx] = Some(result?);
                active -= 1;

                // Start next chunk if available
                if let Some((idx, chunk)) = chunk_iter.next() {
                    let stream = self.read_chunk(&chunk);
                    futures.push(async move { (idx, stream.collect_all().await) });
                    active += 1;
                }
            }
        }

        // Convert results, filtering out None values
        Ok(results.into_iter().flatten().collect())
    }

    /// Get the length of a stream
    pub async fn get_stream_length(&self, stream_name: &str) -> Result<u64> {
        // Read the last event to get the position
        let options = ReadStreamOptions::default()
            .position(EsStreamPosition::End)
            .backwards()
            .max_count(1);

        let mut stream = self
            .client
            .read_stream(stream_name, &options)
            .await
            .map_err(|e| HistoryError::ReadError(e.to_string()))?;

        match stream.next().await {
            Some(Ok(event)) => Ok(event.get_original_event().revision + 1),
            Some(Err(e)) => Err(HistoryError::ReadError(e.to_string())),
            None => Ok(0),
        }
    }

    /// Check if a stream exists
    pub async fn stream_exists(&self, stream_name: &str) -> Result<bool> {
        match self.get_stream_length(stream_name).await {
            Ok(_) => Ok(true),
            Err(HistoryError::StreamNotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Read events by operation ID using the index
    pub async fn read_by_operation_id(
        &self,
        stream_name: &str,
        operation_id: &OperationId,
    ) -> Result<Option<ResolvedEvent>> {
        let index = self
            .index
            .as_ref()
            .ok_or_else(|| HistoryError::IndexError("No index available".to_string()))?;

        let position = {
            let index = index.read().await;
            index.get_operation_position(operation_id)
        };

        if let Some(pos) = position {
            let options = ReadOptions {
                from: StreamPosition::Position(pos),
                max_count: Some(1),
                ..Default::default()
            };
            let mut stream = self.read_stream(stream_name, options);
            if let Some(result) = stream.inner.next().await {
                return Ok(Some(result?));
            }
        }

        Ok(None)
    }

    /// Read events by process ID using the index
    pub async fn read_by_process_id(
        &self,
        stream_name: &str,
        process_id: &ProcessId,
    ) -> Result<Vec<ResolvedEvent>> {
        let index = self
            .index
            .as_ref()
            .ok_or_else(|| HistoryError::IndexError("No index available".to_string()))?;

        let positions = {
            let index = index.read().await;
            index.get_process_operations(process_id)
        };

        let mut events = Vec::new();
        for pos in positions {
            let options = ReadOptions {
                from: StreamPosition::Position(pos),
                max_count: Some(1),
                ..Default::default()
            };
            let mut stream = self.read_stream(stream_name, options);
            if let Some(result) = stream.inner.next().await {
                events.push(result?);
            }
        }

        Ok(events)
    }

    /// Read events in a timestamp range using the index
    pub async fn read_by_time_range_indexed(
        &self,
        stream_name: &str,
        from: u64,
        to: u64,
    ) -> Result<Vec<ResolvedEvent>> {
        let index = self
            .index
            .as_ref()
            .ok_or_else(|| HistoryError::IndexError("No index available".to_string()))?;

        let positions = {
            let index = index.read().await;
            index.get_positions_in_time_range(from, to)
        };

        let mut events = Vec::new();
        for pos in positions {
            let options = ReadOptions {
                from: StreamPosition::Position(pos),
                max_count: Some(1),
                ..Default::default()
            };
            let mut stream = self.read_stream(stream_name, options);
            if let Some(result) = stream.inner.next().await {
                events.push(result?);
            }
        }

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_options_default() {
        let options = ReadOptions::default();
        assert_eq!(options.from, StreamPosition::Start);
        assert!(options.max_count.is_none());
        assert_eq!(options.direction, ReadDirection::Forward);
        assert!(options.resolve_links);
    }

    #[test]
    fn test_chunk_spec_size() {
        let chunk = ChunkSpec {
            stream_name: "test".to_string(),
            start_position: 100,
            end_position: 200,
            chunk_id: 0,
        };
        assert_eq!(chunk.size(), 100);
    }

    #[test]
    fn test_stream_position_conversion() {
        let pos = StreamPosition::Position(42);
        let es_pos: EsStreamPosition = pos.into();
        match es_pos {
            EsStreamPosition::Position(p) => assert_eq!(p, 42),
            _ => panic!("Expected Position variant"),
        }
    }
}
