//! Write-ahead log for crash recovery.
//!
//! This module provides a durable write-ahead log that ensures events
//! are persisted locally before being sent to EventStoreDB, enabling
//! crash recovery and exactly-once delivery semantics.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as TokioBufReader};
use tracing::{debug, info, warn};

use crate::events::HistoryEvent;
use crate::{HistoryError, Result};

/// Entry in the write-ahead log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntry {
    /// Unique sequence number for this entry
    pub sequence: u64,
    /// Stream name for EventStoreDB
    pub stream_name: String,
    /// The event data
    pub event: HistoryEvent,
    /// Timestamp when the entry was written
    pub written_at: DateTime<Utc>,
    /// Whether this entry has been committed to EventStoreDB
    pub committed: bool,
    /// CRC32 checksum for integrity verification
    pub checksum: u32,
}

impl WalEntry {
    /// Create a new WAL entry
    pub fn new(sequence: u64, stream_name: String, event: HistoryEvent) -> Self {
        let mut entry = Self {
            sequence,
            stream_name,
            event,
            written_at: Utc::now(),
            committed: false,
            checksum: 0,
        };
        entry.checksum = entry.calculate_checksum();
        entry
    }

    /// Calculate the CRC32 checksum for this entry
    pub fn calculate_checksum(&self) -> u32 {
        // Create a version without the checksum field for hashing
        let data = format!(
            "{}:{}:{}:{}:{}",
            self.sequence,
            self.stream_name,
            serde_json::to_string(&self.event).unwrap_or_default(),
            self.written_at.timestamp_nanos_opt().unwrap_or(0),
            self.committed
        );
        crc32fast::hash(data.as_bytes())
    }

    /// Verify the integrity of this entry
    pub fn verify(&self) -> bool {
        let expected = {
            let mut temp = self.clone();
            temp.checksum = 0;
            temp.calculate_checksum()
        };
        self.checksum == expected
    }
}

/// Write-ahead log writer
pub struct WalWriter {
    path: PathBuf,
    file: Option<File>,
    sequence: AtomicU64,
    sync_on_write: bool,
}

impl WalWriter {
    /// Create a new WAL writer
    pub async fn new(path: impl AsRef<Path>, sync_on_write: bool) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Open or create the file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;

        // Find the last sequence number
        let sequence = Self::find_last_sequence(&path).await.unwrap_or(0);

        Ok(Self {
            path,
            file: Some(file),
            sequence: AtomicU64::new(sequence),
            sync_on_write,
        })
    }

    /// Append an entry to the WAL
    pub async fn append(&mut self, stream_name: &str, event: &HistoryEvent) -> Result<u64> {
        let sequence = self.sequence.fetch_add(1, Ordering::SeqCst) + 1;
        let entry = WalEntry::new(sequence, stream_name.to_string(), event.clone());

        let json = serde_json::to_string(&entry)?;

        if let Some(file) = &mut self.file {
            file.write_all(json.as_bytes()).await?;
            file.write_all(b"\n").await?;

            if self.sync_on_write {
                file.sync_all().await?;
            }
        }

        debug!(sequence = sequence, stream = stream_name, "WAL entry written");
        Ok(sequence)
    }

    /// Flush the WAL to disk
    pub async fn flush(&mut self) -> Result<()> {
        if let Some(file) = &mut self.file {
            file.flush().await?;
            file.sync_all().await?;
        }
        Ok(())
    }

    /// Get the current sequence number
    pub fn current_sequence(&self) -> u64 {
        self.sequence.load(Ordering::SeqCst)
    }

    /// Find the last sequence number in an existing WAL file
    async fn find_last_sequence(path: &Path) -> Result<u64> {
        if !path.exists() {
            return Ok(0);
        }

        let file = File::open(path).await?;
        let reader = TokioBufReader::new(file);
        let mut lines = reader.lines();
        let mut last_sequence = 0u64;

        while let Some(line) = lines.next_line().await? {
            if line.is_empty() {
                continue;
            }

            match serde_json::from_str::<WalEntry>(&line) {
                Ok(entry) => {
                    last_sequence = last_sequence.max(entry.sequence);
                }
                Err(e) => {
                    warn!(error = %e, "Failed to parse WAL entry");
                }
            }
        }

        Ok(last_sequence)
    }
}

/// Write-ahead log reader
pub struct WalReader {
    path: PathBuf,
}

impl WalReader {
    /// Create a new WAL reader
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    /// Read all entries from the WAL
    pub async fn read_all(&self) -> Result<Vec<WalEntry>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.path).await?;
        let reader = TokioBufReader::new(file);
        let mut lines = reader.lines();
        let mut entries = Vec::new();

        while let Some(line) = lines.next_line().await? {
            if line.is_empty() {
                continue;
            }

            match serde_json::from_str::<WalEntry>(&line) {
                Ok(entry) => {
                    if entry.verify() {
                        entries.push(entry);
                    } else {
                        warn!(
                            sequence = entry.sequence,
                            "WAL entry failed integrity check"
                        );
                    }
                }
                Err(e) => {
                    warn!(error = %e, "Failed to parse WAL entry");
                }
            }
        }

        Ok(entries)
    }

    /// Read uncommitted entries from the WAL
    pub async fn read_uncommitted(&self) -> Result<Vec<WalEntry>> {
        let entries = self.read_all().await?;
        Ok(entries.into_iter().filter(|e| !e.committed).collect())
    }

    /// Read entries for a specific stream
    pub async fn read_stream(&self, stream_name: &str) -> Result<Vec<WalEntry>> {
        let entries = self.read_all().await?;
        Ok(entries
            .into_iter()
            .filter(|e| e.stream_name == stream_name)
            .collect())
    }

    /// Read entries starting from a sequence number
    pub async fn read_from_sequence(&self, sequence: u64) -> Result<Vec<WalEntry>> {
        let entries = self.read_all().await?;
        Ok(entries
            .into_iter()
            .filter(|e| e.sequence >= sequence)
            .collect())
    }
}

/// Complete write-ahead log with recovery support
pub struct WriteAheadLog {
    writer: WalWriter,
    reader: WalReader,
    /// Track committed sequences per stream
    committed_sequences: HashMap<String, u64>,
    /// Path for the commit marker file
    commit_marker_path: PathBuf,
}

impl WriteAheadLog {
    /// Create a new write-ahead log
    pub async fn new(base_path: impl AsRef<Path>) -> Result<Self> {
        let base_path = base_path.as_ref();
        let wal_path = base_path.join("wal.log");
        let commit_marker_path = base_path.join("commits.json");

        let writer = WalWriter::new(&wal_path, true).await?;
        let reader = WalReader::new(&wal_path);

        // Load committed sequences
        let committed_sequences = Self::load_commits(&commit_marker_path).await?;

        Ok(Self {
            writer,
            reader,
            committed_sequences,
            commit_marker_path,
        })
    }

    /// Append an event to the WAL
    pub async fn append(&mut self, stream_name: &str, event: &HistoryEvent) -> Result<u64> {
        self.writer.append(stream_name, event).await
    }

    /// Mark events as committed
    pub async fn commit(&mut self, stream_name: &str, count: u64) -> Result<()> {
        let current = self
            .committed_sequences
            .get(stream_name)
            .copied()
            .unwrap_or(0);
        self.committed_sequences
            .insert(stream_name.to_string(), current + count);

        // Persist commit markers
        self.save_commits().await?;

        debug!(
            stream = stream_name,
            count = count,
            total = current + count,
            "WAL entries committed"
        );

        Ok(())
    }

    /// Get uncommitted events for recovery
    pub async fn get_uncommitted(&self) -> Result<Vec<WalEntry>> {
        self.reader.read_uncommitted().await
    }

    /// Get uncommitted events for a specific stream
    pub async fn get_uncommitted_for_stream(&self, stream_name: &str) -> Result<Vec<WalEntry>> {
        let committed = self
            .committed_sequences
            .get(stream_name)
            .copied()
            .unwrap_or(0);
        let entries = self.reader.read_stream(stream_name).await?;

        Ok(entries
            .into_iter()
            .filter(|e| e.sequence > committed)
            .collect())
    }

    /// Compact the WAL by removing committed entries
    pub async fn compact(&mut self) -> Result<usize> {
        let entries = self.reader.read_all().await?;
        let uncommitted: Vec<_> = entries
            .iter()
            .filter(|e| {
                let committed = self
                    .committed_sequences
                    .get(&e.stream_name)
                    .copied()
                    .unwrap_or(0);
                e.sequence > committed
            })
            .collect();

        let removed_count = entries.len() - uncommitted.len();

        if removed_count > 0 {
            // Rewrite the WAL with only uncommitted entries
            let temp_path = self.commit_marker_path.with_extension("tmp");
            let mut temp_file = File::create(&temp_path).await?;

            for entry in uncommitted {
                let json = serde_json::to_string(entry)?;
                temp_file.write_all(json.as_bytes()).await?;
                temp_file.write_all(b"\n").await?;
            }

            temp_file.sync_all().await?;

            // Atomically replace the old file
            let wal_path = self.commit_marker_path.with_file_name("wal.log");
            tokio::fs::rename(&temp_path, &wal_path).await?;

            // Recreate the writer
            self.writer = WalWriter::new(&wal_path, true).await?;

            info!(removed = removed_count, "WAL compacted");
        }

        Ok(removed_count)
    }

    /// Flush all pending writes
    pub async fn flush(&mut self) -> Result<()> {
        self.writer.flush().await
    }

    /// Get the current sequence number
    pub fn current_sequence(&self) -> u64 {
        self.writer.current_sequence()
    }

    /// Load committed sequences from disk
    async fn load_commits(path: &Path) -> Result<HashMap<String, u64>> {
        if !path.exists() {
            return Ok(HashMap::new());
        }

        let content = tokio::fs::read_to_string(path).await?;
        let commits: HashMap<String, u64> = serde_json::from_str(&content)?;
        Ok(commits)
    }

    /// Save committed sequences to disk
    async fn save_commits(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.committed_sequences)?;
        let temp_path = self.commit_marker_path.with_extension("tmp");

        tokio::fs::write(&temp_path, json.as_bytes()).await?;
        tokio::fs::rename(&temp_path, &self.commit_marker_path).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{TestConfig, TestRunEvent};
    use mamut_core::RunId;

    #[test]
    fn test_wal_entry_checksum() {
        let event = HistoryEvent::TestRun(TestRunEvent::TestStarted {
            run_id: RunId::new(),
            timestamp: Utc::now(),
            config: TestConfig::default(),
        });

        let entry = WalEntry::new(1, "test-stream".to_string(), event);
        assert!(entry.verify());
    }

    #[test]
    fn test_wal_entry_checksum_tamper_detection() {
        let event = HistoryEvent::TestRun(TestRunEvent::TestStarted {
            run_id: RunId::new(),
            timestamp: Utc::now(),
            config: TestConfig::default(),
        });

        let mut entry = WalEntry::new(1, "test-stream".to_string(), event);
        entry.sequence = 2; // Tamper with the entry
        assert!(!entry.verify());
    }

    #[tokio::test]
    async fn test_wal_writer_reader() {
        let temp_dir = tempfile::tempdir().unwrap();
        let wal_path = temp_dir.path().join("test.wal");

        // Write some entries
        {
            let mut writer = WalWriter::new(&wal_path, false).await.unwrap();
            let event = HistoryEvent::TestRun(TestRunEvent::TestStarted {
                run_id: RunId::new(),
                timestamp: Utc::now(),
                config: TestConfig::default(),
            });

            writer.append("stream-1", &event).await.unwrap();
            writer.append("stream-2", &event).await.unwrap();
            writer.flush().await.unwrap();
        }

        // Read them back
        let reader = WalReader::new(&wal_path);
        let entries = reader.read_all().await.unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].sequence, 1);
        assert_eq!(entries[1].sequence, 2);
    }
}
