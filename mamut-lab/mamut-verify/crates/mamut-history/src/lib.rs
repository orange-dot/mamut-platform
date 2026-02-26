//! # mamut-history
//!
//! History persistence layer for KurrentDB/EventStoreDB.
//!
//! This crate provides a comprehensive event sourcing infrastructure for
//! persisting and querying test execution history, operation traces, fault
//! injection events, and verification check results.
//!
//! ## Architecture
//!
//! - **Events**: Strongly-typed event schemas for test runs, operations, faults, and checks
//! - **Writer**: Batched write coordinator with backpressure control
//! - **Reader**: Streaming history reader with lazy loading and chunked parallel access
//! - **WAL**: Write-ahead log for crash recovery
//! - **Checkpoint**: Checkpoint management for resumable processing
//! - **Index**: In-memory indices for efficient queries
//! - **Store**: EventStoreDB-backed history store implementation
//!
//! ## Example
//!
//! ```rust,ignore
//! use mamut_history::{HistoryStore, EventStoreConfig, TestRunEvent};
//! use mamut_core::RunId;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = EventStoreConfig::default();
//!     let store = HistoryStore::connect(config).await?;
//!
//!     // Append events
//!     let run_id = RunId::new();
//!     store.append_test_event(&run_id, TestRunEvent::TestStarted {
//!         run_id: run_id.clone(),
//!         timestamp: chrono::Utc::now(),
//!         config: Default::default(),
//!     }).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod checkpoint;
pub mod events;
pub mod index;
pub mod reader;
pub mod store;
pub mod wal;
pub mod writer;

// Re-exports for convenient access
pub use checkpoint::{Checkpoint, CheckpointManager, CheckpointStorage};
pub use events::{
    CheckEvent, FaultEvent, HistoryEvent, OperationEvent, TestConfig, TestRunEvent,
};
pub use index::HistoryIndex;
pub use reader::{ChunkSpec, HistoryReader, ReadOptions, StreamPosition};
pub use store::{EventStoreConfig, HistoryStore};
pub use wal::{WalEntry, WalReader, WalWriter, WriteAheadLog};
pub use writer::{BatchConfig, StreamWriteBuffer, WriteCoordinator};

// Re-export core types for convenience
pub use mamut_core::{OperationId, ProcessId, RunId};

/// Additional identifier types specific to history module
pub mod ids {
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    /// Unique identifier for a fault injection
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct FaultId(pub Uuid);

    impl FaultId {
        pub fn new() -> Self {
            Self(Uuid::new_v4())
        }
    }

    impl Default for FaultId {
        fn default() -> Self {
            Self::new()
        }
    }

    impl std::fmt::Display for FaultId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    /// Unique identifier for a verification check
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct CheckId(pub Uuid);

    impl CheckId {
        pub fn new() -> Self {
            Self(Uuid::new_v4())
        }
    }

    impl Default for CheckId {
        fn default() -> Self {
            Self::new()
        }
    }

    impl std::fmt::Display for CheckId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    /// Unique identifier for a checkpoint
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct CheckpointId(pub Uuid);

    impl CheckpointId {
        pub fn new() -> Self {
            Self(Uuid::new_v4())
        }
    }

    impl Default for CheckpointId {
        fn default() -> Self {
            Self::new()
        }
    }

    impl std::fmt::Display for CheckpointId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
}

/// Error types for the history module
#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("EventStore connection error: {0}")]
    ConnectionError(String),

    #[error("EventStore write error: {0}")]
    WriteError(String),

    #[error("EventStore read error: {0}")]
    ReadError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("WAL error: {0}")]
    WalError(String),

    #[error("Checkpoint error: {0}")]
    CheckpointError(String),

    #[error("Index error: {0}")]
    IndexError(String),

    #[error("Stream not found: {0}")]
    StreamNotFound(String),

    #[error("Invalid position: {0}")]
    InvalidPosition(String),

    #[error("Timeout waiting for operation")]
    Timeout,

    #[error("Backpressure limit reached")]
    BackpressureLimitReached,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, HistoryError>;
