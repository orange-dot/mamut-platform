//! Replay generator for reproducing recorded histories

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt::Debug;
use std::marker::PhantomData;

use mamut_core::ProcessId;

use crate::context::GeneratorContext;
use crate::generated::{GeneratedOp, OperationMetadata};
use crate::traits::Generator;

/// A recorded operation for replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedOp<V> {
    /// The process that executed this operation
    pub process: ProcessId,
    /// The operation value
    pub value: V,
    /// Original timestamp
    pub timestamp: u64,
    /// Optional metadata
    pub metadata: Option<OperationMetadata>,
}

impl<V> RecordedOp<V> {
    /// Create a new recorded operation
    pub fn new(process: ProcessId, value: V, timestamp: u64) -> Self {
        Self {
            process,
            value,
            timestamp,
            metadata: None,
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, metadata: OperationMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Replay mode determining how operations are matched to processes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReplayMode {
    /// Strict: only return operations for the exact matching process
    #[default]
    Strict,
    /// Relaxed: return next operation regardless of process match
    Relaxed,
    /// Sequential: return operations in order, ignoring process
    Sequential,
}

/// Generator that replays a recorded history of operations
///
/// This generator is used to reproduce specific execution histories for
/// debugging and regression testing. It can operate in different modes
/// depending on how strictly process assignment should be followed.
pub struct ReplayGenerator<V> {
    /// Recorded operations to replay
    operations: VecDeque<RecordedOp<V>>,
    /// Original operations for reset
    original: Vec<RecordedOp<V>>,
    /// Replay mode
    mode: ReplayMode,
    /// Current position
    position: usize,
    /// Sequence counter
    sequence: u64,
    /// Generator name
    name: String,
    /// Allow partial replay (continue after mismatch)
    allow_partial: bool,
    /// Phantom
    _phantom: PhantomData<V>,
}

impl<V> ReplayGenerator<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + Debug,
{
    /// Create a new replay generator
    pub fn new(operations: Vec<RecordedOp<V>>) -> Self {
        Self {
            operations: operations.clone().into(),
            original: operations,
            mode: ReplayMode::default(),
            position: 0,
            sequence: 0,
            name: "ReplayGenerator".to_string(),
            allow_partial: false,
            _phantom: PhantomData,
        }
    }

    /// Set the generator name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the replay mode
    pub fn with_mode(mut self, mode: ReplayMode) -> Self {
        self.mode = mode;
        self
    }

    /// Allow partial replay
    pub fn allow_partial(mut self) -> Self {
        self.allow_partial = true;
        self
    }

    /// Get the number of remaining operations
    pub fn remaining_count(&self) -> usize {
        self.operations.len()
    }

    /// Peek at the next operation without consuming it
    pub fn peek(&self) -> Option<&RecordedOp<V>> {
        self.operations.front()
    }

    /// Get the current position in the replay
    pub fn position(&self) -> usize {
        self.position
    }

    /// Get the total number of operations in the original history
    pub fn total_operations(&self) -> usize {
        self.original.len()
    }

    /// Serialize to JSON for storage
    pub fn to_json(&self) -> Result<String, serde_json::Error>
    where
        V: Serialize,
    {
        serde_json::to_string_pretty(&self.original)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error>
    where
        V: DeserializeOwned,
    {
        let operations: Vec<RecordedOp<V>> = serde_json::from_str(json)?;
        Ok(Self::new(operations))
    }
}

#[async_trait]
impl<V> Generator for ReplayGenerator<V>
where
    V: Serialize + DeserializeOwned + Send + Sync + Clone + Debug + 'static,
{
    type Value = V;

    async fn next(
        &mut self,
        process: ProcessId,
        _ctx: &GeneratorContext,
    ) -> Option<GeneratedOp<Self::Value>> {
        match self.mode {
            ReplayMode::Strict => {
                // Find the next operation for this specific process
                let idx = self.operations.iter().position(|op| op.process == process)?;
                let recorded = self.operations.remove(idx)?;
                self.position += 1;
                self.sequence += 1;

                Some(
                    GeneratedOp::new(recorded.value)
                        .with_target(recorded.process)
                        .with_metadata(recorded.metadata.unwrap_or_default())
                        .from_generator(&self.name)
                        .with_sequence(self.sequence),
                )
            }
            ReplayMode::Relaxed => {
                // Try to find an operation for this process, otherwise take the first
                let idx = self
                    .operations
                    .iter()
                    .position(|op| op.process == process)
                    .or(if self.allow_partial && !self.operations.is_empty() {
                        Some(0)
                    } else {
                        None
                    })?;

                let recorded = self.operations.remove(idx)?;
                self.position += 1;
                self.sequence += 1;

                Some(
                    GeneratedOp::new(recorded.value)
                        .with_target(recorded.process)
                        .with_metadata(recorded.metadata.unwrap_or_default())
                        .from_generator(&self.name)
                        .with_sequence(self.sequence),
                )
            }
            ReplayMode::Sequential => {
                // Just take the next operation regardless of process
                let recorded = self.operations.pop_front()?;
                self.position += 1;
                self.sequence += 1;

                Some(
                    GeneratedOp::new(recorded.value)
                        .with_target(recorded.process)
                        .with_metadata(recorded.metadata.unwrap_or_default())
                        .from_generator(&self.name)
                        .with_sequence(self.sequence),
                )
            }
        }
    }

    fn update(&mut self, _value: &Self::Value, _process: ProcessId, _success: bool, _result: Option<&str>) {
        // Replay doesn't update based on results
    }

    fn is_exhausted(&self) -> bool {
        self.operations.is_empty()
    }

    fn reset(&mut self, _seed: u64) {
        self.operations = self.original.clone().into();
        self.position = 0;
        self.sequence = 0;
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn remaining(&self) -> Option<usize> {
        Some(self.operations.len())
    }
}

impl<V: Debug> std::fmt::Debug for ReplayGenerator<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReplayGenerator")
            .field("name", &self.name)
            .field("mode", &self.mode)
            .field("position", &self.position)
            .field("remaining", &self.operations.len())
            .field("total", &self.original.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestOp(String);

    fn create_history() -> Vec<RecordedOp<TestOp>> {
        vec![
            RecordedOp::new(ProcessId::new(1), TestOp("op1".to_string()), 0),
            RecordedOp::new(ProcessId::new(2), TestOp("op2".to_string()), 1),
            RecordedOp::new(ProcessId::new(1), TestOp("op3".to_string()), 2),
            RecordedOp::new(ProcessId::new(2), TestOp("op4".to_string()), 3),
        ]
    }

    #[tokio::test]
    async fn test_strict_replay() {
        let mut generator = ReplayGenerator::new(create_history()).with_mode(ReplayMode::Strict);
        let ctx = GeneratorContext::new();

        // Process 1 should get op1
        let op = generator.next(ProcessId::new(1), &ctx).await.unwrap();
        assert_eq!(op.value, TestOp("op1".to_string()));

        // Process 2 should get op2
        let op = generator.next(ProcessId::new(2), &ctx).await.unwrap();
        assert_eq!(op.value, TestOp("op2".to_string()));

        // Process 1 should get op3
        let op = generator.next(ProcessId::new(1), &ctx).await.unwrap();
        assert_eq!(op.value, TestOp("op3".to_string()));

        // Process 3 has no operations
        assert!(generator.next(ProcessId::new(3), &ctx).await.is_none());
    }

    #[tokio::test]
    async fn test_sequential_replay() {
        let mut generator = ReplayGenerator::new(create_history()).with_mode(ReplayMode::Sequential);
        let ctx = GeneratorContext::new();

        // Should get operations in order regardless of process
        let op = generator.next(ProcessId::new(99), &ctx).await.unwrap();
        assert_eq!(op.value, TestOp("op1".to_string()));

        let op = generator.next(ProcessId::new(99), &ctx).await.unwrap();
        assert_eq!(op.value, TestOp("op2".to_string()));
    }

    #[tokio::test]
    async fn test_replay_reset() {
        let mut generator = ReplayGenerator::new(create_history()).with_mode(ReplayMode::Sequential);
        let ctx = GeneratorContext::new();

        generator.next(ProcessId::new(1), &ctx).await;
        generator.next(ProcessId::new(1), &ctx).await;
        assert_eq!(generator.remaining_count(), 2);

        generator.reset(0);
        assert_eq!(generator.remaining_count(), 4);
        assert_eq!(generator.position(), 0);
    }

    #[tokio::test]
    async fn test_json_serialization() {
        let generator = ReplayGenerator::new(create_history());
        let json = generator.to_json().unwrap();

        let restored: ReplayGenerator<TestOp> = ReplayGenerator::from_json(&json).unwrap();
        assert_eq!(restored.total_operations(), generator.total_operations());
    }
}
