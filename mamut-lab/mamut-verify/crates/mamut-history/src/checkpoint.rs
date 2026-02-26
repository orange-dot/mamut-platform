//! Checkpoint management for resumable processing.
//!
//! This module provides checkpoint management capabilities for:
//! - Tracking stream positions across multiple streams
//! - Persisting checkpoints for crash recovery
//! - Resuming processing from the last known good state

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use eventstore::{AppendToStreamOptions, Client, ExpectedRevision, ReadStreamOptions, StreamPosition};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::ids::CheckpointId;
use crate::{HistoryError, Result};

/// A checkpoint representing a consistent point in the event streams
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Unique identifier for this checkpoint
    pub id: CheckpointId,
    /// Name/label for this checkpoint
    pub name: String,
    /// Timestamp when the checkpoint was created
    pub created_at: DateTime<Utc>,
    /// Stream positions at this checkpoint
    pub stream_positions: HashMap<String, u64>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Checkpoint {
    /// Create a new checkpoint
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: CheckpointId::new(),
            name: name.into(),
            created_at: Utc::now(),
            stream_positions: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a stream position to the checkpoint
    pub fn with_stream_position(mut self, stream_name: impl Into<String>, position: u64) -> Self {
        self.stream_positions.insert(stream_name.into(), position);
        self
    }

    /// Add metadata to the checkpoint
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Get the position for a specific stream
    pub fn get_position(&self, stream_name: &str) -> Option<u64> {
        self.stream_positions.get(stream_name).copied()
    }

    /// Check if this checkpoint includes a specific stream
    pub fn has_stream(&self, stream_name: &str) -> bool {
        self.stream_positions.contains_key(stream_name)
    }
}

/// Trait for checkpoint storage backends
#[async_trait]
pub trait CheckpointStorage: Send + Sync {
    /// Save a checkpoint
    async fn save(&self, checkpoint: &Checkpoint) -> Result<()>;

    /// Load a checkpoint by ID
    async fn load(&self, id: &CheckpointId) -> Result<Option<Checkpoint>>;

    /// Load a checkpoint by name
    async fn load_by_name(&self, name: &str) -> Result<Option<Checkpoint>>;

    /// Load the latest checkpoint
    async fn load_latest(&self) -> Result<Option<Checkpoint>>;

    /// List all checkpoints
    async fn list(&self) -> Result<Vec<Checkpoint>>;

    /// Delete a checkpoint
    async fn delete(&self, id: &CheckpointId) -> Result<()>;

    /// Delete checkpoints older than a certain time
    async fn delete_older_than(&self, cutoff: DateTime<Utc>) -> Result<usize>;
}

/// File-based checkpoint storage
pub struct FileCheckpointStorage {
    base_path: PathBuf,
}

impl FileCheckpointStorage {
    /// Create a new file-based checkpoint storage
    pub async fn new(base_path: impl AsRef<Path>) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        tokio::fs::create_dir_all(&base_path).await?;
        Ok(Self { base_path })
    }

    fn checkpoint_path(&self, id: &CheckpointId) -> PathBuf {
        self.base_path.join(format!("{}.json", id.0))
    }

    fn latest_path(&self) -> PathBuf {
        self.base_path.join("latest.json")
    }
}

#[async_trait]
impl CheckpointStorage for FileCheckpointStorage {
    async fn save(&self, checkpoint: &Checkpoint) -> Result<()> {
        let path = self.checkpoint_path(&checkpoint.id);
        let json = serde_json::to_string_pretty(checkpoint)?;

        // Write to temp file first
        let temp_path = path.with_extension("tmp");
        tokio::fs::write(&temp_path, &json).await?;
        tokio::fs::rename(&temp_path, &path).await?;

        // Update latest pointer
        let latest_json = serde_json::to_string(&checkpoint.id)?;
        let latest_temp = self.latest_path().with_extension("tmp");
        tokio::fs::write(&latest_temp, &latest_json).await?;
        tokio::fs::rename(&latest_temp, self.latest_path()).await?;

        debug!(id = %checkpoint.id.0, name = checkpoint.name, "Checkpoint saved");
        Ok(())
    }

    async fn load(&self, id: &CheckpointId) -> Result<Option<Checkpoint>> {
        let path = self.checkpoint_path(id);
        if !path.exists() {
            return Ok(None);
        }

        let content = tokio::fs::read_to_string(&path).await?;
        let checkpoint: Checkpoint = serde_json::from_str(&content)?;
        Ok(Some(checkpoint))
    }

    async fn load_by_name(&self, name: &str) -> Result<Option<Checkpoint>> {
        // Scan all checkpoints to find by name
        let checkpoints = self.list().await?;
        Ok(checkpoints.into_iter().find(|c| c.name == name))
    }

    async fn load_latest(&self) -> Result<Option<Checkpoint>> {
        let latest_path = self.latest_path();
        if !latest_path.exists() {
            return Ok(None);
        }

        let id_json = tokio::fs::read_to_string(&latest_path).await?;
        let id: CheckpointId = serde_json::from_str(&id_json)?;
        self.load(&id).await
    }

    async fn list(&self) -> Result<Vec<Checkpoint>> {
        let mut checkpoints = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.base_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if filename != "latest" {
                    if let Ok(content) = tokio::fs::read_to_string(&path).await {
                        if let Ok(checkpoint) = serde_json::from_str::<Checkpoint>(&content) {
                            checkpoints.push(checkpoint);
                        }
                    }
                }
            }
        }

        // Sort by creation time
        checkpoints.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(checkpoints)
    }

    async fn delete(&self, id: &CheckpointId) -> Result<()> {
        let path = self.checkpoint_path(id);
        if path.exists() {
            tokio::fs::remove_file(&path).await?;
            debug!(id = %id.0, "Checkpoint deleted");
        }
        Ok(())
    }

    async fn delete_older_than(&self, cutoff: DateTime<Utc>) -> Result<usize> {
        let checkpoints = self.list().await?;
        let mut deleted = 0;

        for checkpoint in checkpoints {
            if checkpoint.created_at < cutoff {
                self.delete(&checkpoint.id).await?;
                deleted += 1;
            }
        }

        if deleted > 0 {
            info!(count = deleted, "Deleted old checkpoints");
        }

        Ok(deleted)
    }
}

/// EventStoreDB-based checkpoint storage
pub struct EventStoreCheckpointStorage {
    client: Client,
    stream_name: String,
}

impl EventStoreCheckpointStorage {
    /// Create a new EventStoreDB-based checkpoint storage
    pub fn new(client: Client, stream_name: impl Into<String>) -> Self {
        Self {
            client,
            stream_name: stream_name.into(),
        }
    }
}

#[async_trait]
impl CheckpointStorage for EventStoreCheckpointStorage {
    async fn save(&self, checkpoint: &Checkpoint) -> Result<()> {
        let event_data = eventstore::EventData::json("CheckpointCreated", checkpoint).map_err(
            |e| {
                HistoryError::SerializationError(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            },
        )?;

        let options = AppendToStreamOptions::default().expected_revision(ExpectedRevision::Any);

        self.client
            .append_to_stream(&self.stream_name, &options, event_data)
            .await
            .map_err(|e| HistoryError::WriteError(e.to_string()))?;

        debug!(id = %checkpoint.id.0, "Checkpoint saved to EventStoreDB");
        Ok(())
    }

    async fn load(&self, id: &CheckpointId) -> Result<Option<Checkpoint>> {
        let checkpoints = self.list().await?;
        Ok(checkpoints.into_iter().find(|c| c.id.0 == id.0))
    }

    async fn load_by_name(&self, name: &str) -> Result<Option<Checkpoint>> {
        let checkpoints = self.list().await?;
        Ok(checkpoints.into_iter().find(|c| c.name == name))
    }

    async fn load_latest(&self) -> Result<Option<Checkpoint>> {
        let options = ReadStreamOptions::default()
            .position(StreamPosition::End)
            .backwards()
            .max_count(1);

        let mut stream = self
            .client
            .read_stream(&self.stream_name, &options)
            .await
            .map_err(|e| HistoryError::ReadError(e.to_string()))?;

        use futures::StreamExt;
        if let Some(event) = stream.next().await {
            match event {
                Ok(resolved) => {
                    let checkpoint: Checkpoint = resolved
                        .get_original_event()
                        .as_json()
                        .map_err(|e| HistoryError::ReadError(e.to_string()))?;
                    return Ok(Some(checkpoint));
                }
                Err(e) => return Err(HistoryError::ReadError(e.to_string())),
            }
        }

        Ok(None)
    }

    async fn list(&self) -> Result<Vec<Checkpoint>> {
        let options = ReadStreamOptions::default().position(StreamPosition::Start);

        let mut stream = self
            .client
            .read_stream(&self.stream_name, &options)
            .await
            .map_err(|e| HistoryError::ReadError(e.to_string()))?;

        let mut checkpoints = Vec::new();
        use futures::StreamExt;
        while let Some(event) = stream.next().await {
            match event {
                Ok(resolved) => {
                    if let Ok(checkpoint) = resolved.get_original_event().as_json::<Checkpoint>() {
                        checkpoints.push(checkpoint);
                    }
                }
                Err(e) => return Err(HistoryError::ReadError(e.to_string())),
            }
        }

        Ok(checkpoints)
    }

    async fn delete(&self, _id: &CheckpointId) -> Result<()> {
        // EventStoreDB doesn't support deleting individual events
        // We could mark it as deleted with a tombstone event
        warn!("Delete not supported for EventStoreDB checkpoint storage");
        Ok(())
    }

    async fn delete_older_than(&self, _cutoff: DateTime<Utc>) -> Result<usize> {
        // Would need to implement stream truncation
        warn!("delete_older_than not supported for EventStoreDB checkpoint storage");
        Ok(0)
    }
}

/// Checkpoint manager that coordinates checkpoint operations
pub struct CheckpointManager {
    storage: Arc<dyn CheckpointStorage>,
    /// Current stream positions being tracked
    current_positions: Arc<RwLock<HashMap<String, u64>>>,
    /// Auto-checkpoint interval (in events)
    auto_checkpoint_interval: Option<u64>,
    /// Events since last checkpoint
    events_since_checkpoint: Arc<RwLock<u64>>,
}

impl CheckpointManager {
    /// Create a new checkpoint manager
    pub fn new(storage: Arc<dyn CheckpointStorage>) -> Self {
        Self {
            storage,
            current_positions: Arc::new(RwLock::new(HashMap::new())),
            auto_checkpoint_interval: None,
            events_since_checkpoint: Arc::new(RwLock::new(0)),
        }
    }

    /// Enable auto-checkpointing after a certain number of events
    pub fn with_auto_checkpoint(mut self, interval: u64) -> Self {
        self.auto_checkpoint_interval = Some(interval);
        self
    }

    /// Update the position for a stream
    pub async fn update_position(&self, stream_name: &str, position: u64) {
        let mut positions = self.current_positions.write().await;
        positions.insert(stream_name.to_string(), position);

        // Check if we should auto-checkpoint
        if let Some(interval) = self.auto_checkpoint_interval {
            let mut count = self.events_since_checkpoint.write().await;
            *count += 1;
            if *count >= interval {
                *count = 0;
                drop(positions);
                drop(count);
                if let Err(e) = self.create_checkpoint("auto").await {
                    warn!(error = %e, "Failed to create auto-checkpoint");
                }
            }
        }
    }

    /// Get the current position for a stream
    pub async fn get_position(&self, stream_name: &str) -> Option<u64> {
        let positions = self.current_positions.read().await;
        positions.get(stream_name).copied()
    }

    /// Get all current positions
    pub async fn get_all_positions(&self) -> HashMap<String, u64> {
        let positions = self.current_positions.read().await;
        positions.clone()
    }

    /// Create a checkpoint with the current positions
    pub async fn create_checkpoint(&self, name: &str) -> Result<Checkpoint> {
        let positions = self.current_positions.read().await;

        let checkpoint = Checkpoint {
            id: CheckpointId::new(),
            name: name.to_string(),
            created_at: Utc::now(),
            stream_positions: positions.clone(),
            metadata: HashMap::new(),
        };

        self.storage.save(&checkpoint).await?;
        info!(
            id = %checkpoint.id.0,
            name = checkpoint.name,
            streams = positions.len(),
            "Checkpoint created"
        );

        Ok(checkpoint)
    }

    /// Create a checkpoint with additional metadata
    pub async fn create_checkpoint_with_metadata(
        &self,
        name: &str,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<Checkpoint> {
        let positions = self.current_positions.read().await;

        let checkpoint = Checkpoint {
            id: CheckpointId::new(),
            name: name.to_string(),
            created_at: Utc::now(),
            stream_positions: positions.clone(),
            metadata,
        };

        self.storage.save(&checkpoint).await?;
        Ok(checkpoint)
    }

    /// Restore from a checkpoint
    pub async fn restore(&self, checkpoint: &Checkpoint) -> Result<()> {
        let mut positions = self.current_positions.write().await;
        *positions = checkpoint.stream_positions.clone();

        info!(
            id = %checkpoint.id.0,
            name = checkpoint.name,
            streams = positions.len(),
            "Restored from checkpoint"
        );

        Ok(())
    }

    /// Restore from the latest checkpoint
    pub async fn restore_latest(&self) -> Result<Option<Checkpoint>> {
        if let Some(checkpoint) = self.storage.load_latest().await? {
            self.restore(&checkpoint).await?;
            return Ok(Some(checkpoint));
        }
        Ok(None)
    }

    /// List all available checkpoints
    pub async fn list_checkpoints(&self) -> Result<Vec<Checkpoint>> {
        self.storage.list().await
    }

    /// Delete a checkpoint
    pub async fn delete_checkpoint(&self, id: &CheckpointId) -> Result<()> {
        self.storage.delete(id).await
    }

    /// Delete old checkpoints, keeping the most recent ones
    pub async fn cleanup_old_checkpoints(&self, keep_count: usize) -> Result<usize> {
        let checkpoints = self.storage.list().await?;
        if checkpoints.len() <= keep_count {
            return Ok(0);
        }

        let mut deleted = 0;
        for checkpoint in checkpoints.iter().take(checkpoints.len() - keep_count) {
            self.storage.delete(&checkpoint.id).await?;
            deleted += 1;
        }

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_creation() {
        let checkpoint = Checkpoint::new("test-checkpoint")
            .with_stream_position("stream-1", 100)
            .with_stream_position("stream-2", 200)
            .with_metadata("key", serde_json::json!("value"));

        assert_eq!(checkpoint.name, "test-checkpoint");
        assert_eq!(checkpoint.get_position("stream-1"), Some(100));
        assert_eq!(checkpoint.get_position("stream-2"), Some(200));
        assert!(checkpoint.has_stream("stream-1"));
        assert!(!checkpoint.has_stream("stream-3"));
    }

    #[tokio::test]
    async fn test_file_checkpoint_storage() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = FileCheckpointStorage::new(temp_dir.path()).await.unwrap();

        let checkpoint = Checkpoint::new("test").with_stream_position("stream-1", 42);

        storage.save(&checkpoint).await.unwrap();

        let loaded = storage.load(&checkpoint.id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().name, "test");

        let latest = storage.load_latest().await.unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().id.0, checkpoint.id.0);

        let list = storage.list().await.unwrap();
        assert_eq!(list.len(), 1);

        storage.delete(&checkpoint.id).await.unwrap();
        let loaded = storage.load(&checkpoint.id).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_checkpoint_manager() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = Arc::new(FileCheckpointStorage::new(temp_dir.path()).await.unwrap());
        let manager = CheckpointManager::new(storage);

        manager.update_position("stream-1", 100).await;
        manager.update_position("stream-2", 200).await;

        let checkpoint = manager.create_checkpoint("manual").await.unwrap();
        assert_eq!(checkpoint.get_position("stream-1"), Some(100));
        assert_eq!(checkpoint.get_position("stream-2"), Some(200));

        // Modify positions
        manager.update_position("stream-1", 150).await;

        // Restore
        manager.restore(&checkpoint).await.unwrap();
        assert_eq!(manager.get_position("stream-1").await, Some(100));
    }
}
