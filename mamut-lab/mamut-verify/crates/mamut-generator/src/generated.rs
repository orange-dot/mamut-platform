//! Generated operation types

use serde::{Deserialize, Serialize};
use std::time::Duration;

use mamut_core::ProcessId;

/// Metadata about a generated operation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OperationMetadata {
    /// Priority hint (higher = more important)
    pub priority: i32,
    /// Expected duration hint
    #[serde(with = "option_duration_millis")]
    pub expected_duration: Option<Duration>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Whether this operation may cause state changes
    pub may_mutate: bool,
    /// Whether this operation is idempotent
    pub idempotent: bool,
    /// Suggested retry count on failure
    pub retry_count: u32,
    /// Custom metadata
    pub custom: std::collections::HashMap<String, String>,
}

mod option_duration_millis {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(value: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(d) => Some(d.as_millis() as u64).serialize(serializer),
            None => None::<u64>.serialize(serializer),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis: Option<u64> = Option::deserialize(deserializer)?;
        Ok(millis.map(Duration::from_millis))
    }
}

impl OperationMetadata {
    /// Create new metadata with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set the expected duration
    pub fn with_expected_duration(mut self, duration: Duration) -> Self {
        self.expected_duration = Some(duration);
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(|t| t.into()));
        self
    }

    /// Mark as mutating
    pub fn mutating(mut self) -> Self {
        self.may_mutate = true;
        self
    }

    /// Mark as read-only (non-mutating)
    pub fn read_only(mut self) -> Self {
        self.may_mutate = false;
        self
    }

    /// Mark as idempotent
    pub fn idempotent(mut self) -> Self {
        self.idempotent = true;
        self
    }

    /// Set retry count
    pub fn with_retries(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }

    /// Add custom metadata
    pub fn with_custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom.insert(key.into(), value.into());
        self
    }

    /// Check if a tag is present
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}

/// A generated operation ready for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedOp<V> {
    /// The operation value
    pub value: V,
    /// Target process (may differ from requesting process)
    pub target_process: Option<ProcessId>,
    /// Operation metadata
    pub metadata: OperationMetadata,
    /// Delay before execution (optional) in milliseconds
    pub delay_ms: Option<u64>,
    /// Preconditions that must hold (serialized predicates)
    pub preconditions: Vec<String>,
    /// The generator that produced this operation
    pub source_generator: String,
    /// Generation timestamp (for ordering)
    pub generation_sequence: u64,
}

impl<V> GeneratedOp<V> {
    /// Create a new generated operation
    pub fn new(value: V) -> Self {
        Self {
            value,
            target_process: None,
            metadata: OperationMetadata::new(),
            delay_ms: None,
            preconditions: Vec::new(),
            source_generator: String::new(),
            generation_sequence: 0,
        }
    }

    /// Create an operation with a target process
    pub fn for_process(value: V, process: ProcessId) -> Self {
        Self {
            value,
            target_process: Some(process),
            metadata: OperationMetadata::new(),
            delay_ms: None,
            preconditions: Vec::new(),
            source_generator: String::new(),
            generation_sequence: 0,
        }
    }

    /// Set the target process
    pub fn with_target(mut self, process: ProcessId) -> Self {
        self.target_process = Some(process);
        self
    }

    /// Set the metadata
    pub fn with_metadata(mut self, metadata: OperationMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set a delay before execution
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay_ms = Some(delay.as_millis() as u64);
        self
    }

    /// Get the delay as a Duration
    pub fn delay(&self) -> Option<Duration> {
        self.delay_ms.map(Duration::from_millis)
    }

    /// Add a precondition
    pub fn with_precondition(mut self, condition: impl Into<String>) -> Self {
        self.preconditions.push(condition.into());
        self
    }

    /// Set the source generator name
    pub fn from_generator(mut self, name: impl Into<String>) -> Self {
        self.source_generator = name.into();
        self
    }

    /// Set the generation sequence number
    pub fn with_sequence(mut self, seq: u64) -> Self {
        self.generation_sequence = seq;
        self
    }

    /// Map the value to a new type
    pub fn map<U, F: FnOnce(V) -> U>(self, f: F) -> GeneratedOp<U> {
        GeneratedOp {
            value: f(self.value),
            target_process: self.target_process,
            metadata: self.metadata,
            delay_ms: self.delay_ms,
            preconditions: self.preconditions,
            source_generator: self.source_generator,
            generation_sequence: self.generation_sequence,
        }
    }

    /// Check if this operation has any preconditions
    pub fn has_preconditions(&self) -> bool {
        !self.preconditions.is_empty()
    }

    /// Get the effective target process
    pub fn effective_target(&self, default: ProcessId) -> ProcessId {
        self.target_process.unwrap_or(default)
    }
}

impl<V: Clone> GeneratedOp<V> {
    /// Clone with a new target process
    pub fn retarget(&self, process: ProcessId) -> Self {
        Self {
            value: self.value.clone(),
            target_process: Some(process),
            metadata: self.metadata.clone(),
            delay_ms: self.delay_ms,
            preconditions: self.preconditions.clone(),
            source_generator: self.source_generator.clone(),
            generation_sequence: self.generation_sequence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_metadata() {
        let meta = OperationMetadata::new()
            .with_priority(10)
            .with_tag("critical")
            .with_tag("network")
            .mutating()
            .with_retries(3)
            .with_custom("key", "value");

        assert_eq!(meta.priority, 10);
        assert!(meta.has_tag("critical"));
        assert!(meta.has_tag("network"));
        assert!(!meta.has_tag("other"));
        assert!(meta.may_mutate);
        assert_eq!(meta.retry_count, 3);
        assert_eq!(meta.custom.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_generated_op() {
        let op = GeneratedOp::new(42i32)
            .with_target(ProcessId::new(1))
            .with_delay(Duration::from_millis(100))
            .with_precondition("state.ready == true")
            .from_generator("random")
            .with_sequence(5);

        assert_eq!(op.value, 42);
        assert_eq!(op.target_process, Some(ProcessId::new(1)));
        assert_eq!(op.delay(), Some(Duration::from_millis(100)));
        assert!(op.has_preconditions());
        assert_eq!(op.source_generator, "random");
        assert_eq!(op.generation_sequence, 5);
    }

    #[test]
    fn test_generated_op_map() {
        let op = GeneratedOp::new(42i32)
            .with_target(ProcessId::new(1))
            .from_generator("test");

        let mapped = op.map(|v| v.to_string());
        assert_eq!(mapped.value, "42");
        assert_eq!(mapped.target_process, Some(ProcessId::new(1)));
        assert_eq!(mapped.source_generator, "test");
    }

    #[test]
    fn test_effective_target() {
        let default = ProcessId::new(99);

        let with_target = GeneratedOp::new(1).with_target(ProcessId::new(5));
        assert_eq!(with_target.effective_target(default), ProcessId::new(5));

        let without_target = GeneratedOp::new(1);
        assert_eq!(without_target.effective_target(default), default);
    }

    #[test]
    fn test_metadata_serialization() {
        let meta = OperationMetadata::new()
            .with_expected_duration(Duration::from_millis(500))
            .with_tag("test");

        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: OperationMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.expected_duration, Some(Duration::from_millis(500)));
        assert!(deserialized.has_tag("test"));
    }
}
