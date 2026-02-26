//! Trace file format for TLA+ validation.
//!
//! This module defines the NDJSON (Newline-Delimited JSON) format used for
//! execution traces that can be validated against TLA+ specifications.
//!
//! # Format
//!
//! Each line in a trace file is a JSON object representing a single step:
//!
//! ```json
//! {"clock": {"logical": 1}, "action": "Write", "vars": {"x": 42}, "params": {"key": "foo"}}
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use thiserror::Error;

/// Errors that can occur when working with trace files.
#[derive(Debug, Error)]
pub enum TraceError {
    /// I/O error reading or writing trace file.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parsing error.
    #[error("JSON error at line {line}: {source}")]
    Json {
        line: usize,
        #[source]
        source: serde_json::Error,
    },

    /// Invalid trace format.
    #[error("Invalid trace format: {0}")]
    InvalidFormat(String),

    /// Empty trace file.
    #[error("Trace file is empty")]
    EmptyTrace,

    /// Clock ordering violation.
    #[error("Clock ordering violation at step {step}: {message}")]
    ClockOrdering { step: usize, message: String },
}

/// A clock value for ordering trace steps.
///
/// Supports both logical (Lamport) clocks and vector clocks for
/// partial ordering in distributed systems.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TraceClock {
    /// A simple logical (Lamport) clock.
    Logical(u64),

    /// A vector clock for distributed systems.
    Vector(VectorClock),

    /// A hybrid logical clock (HLC).
    Hybrid {
        /// Physical time component (typically wall clock).
        physical: u64,
        /// Logical counter for events at the same physical time.
        logical: u32,
    },
}

impl TraceClock {
    /// Create a new logical clock with the given value.
    pub fn logical(value: u64) -> Self {
        TraceClock::Logical(value)
    }

    /// Create a new vector clock.
    pub fn vector(clock: VectorClock) -> Self {
        TraceClock::Vector(clock)
    }

    /// Create a new hybrid logical clock.
    pub fn hybrid(physical: u64, logical: u32) -> Self {
        TraceClock::Hybrid { physical, logical }
    }

    /// Check if this clock happens before another clock.
    ///
    /// For logical clocks, this is a simple less-than comparison.
    /// For vector clocks, this uses the happens-before relation.
    pub fn happens_before(&self, other: &TraceClock) -> bool {
        match (self, other) {
            (TraceClock::Logical(a), TraceClock::Logical(b)) => a < b,
            (TraceClock::Vector(a), TraceClock::Vector(b)) => a.happens_before(b),
            (
                TraceClock::Hybrid {
                    physical: p1,
                    logical: l1,
                },
                TraceClock::Hybrid {
                    physical: p2,
                    logical: l2,
                },
            ) => p1 < p2 || (p1 == p2 && l1 < l2),
            // Different clock types are not comparable
            _ => false,
        }
    }

    /// Check if this clock is concurrent with another clock.
    ///
    /// Two events are concurrent if neither happens before the other.
    pub fn concurrent_with(&self, other: &TraceClock) -> bool {
        !self.happens_before(other) && !other.happens_before(self)
    }

    /// Get the logical value if this is a logical clock.
    pub fn as_logical(&self) -> Option<u64> {
        match self {
            TraceClock::Logical(v) => Some(*v),
            TraceClock::Hybrid { logical, .. } => Some(*logical as u64),
            _ => None,
        }
    }
}

impl Default for TraceClock {
    fn default() -> Self {
        TraceClock::Logical(0)
    }
}

impl PartialOrd for TraceClock {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (TraceClock::Logical(a), TraceClock::Logical(b)) => Some(a.cmp(b)),
            (TraceClock::Vector(a), TraceClock::Vector(b)) => a.partial_cmp(b),
            (
                TraceClock::Hybrid {
                    physical: p1,
                    logical: l1,
                },
                TraceClock::Hybrid {
                    physical: p2,
                    logical: l2,
                },
            ) => Some((p1, l1).cmp(&(p2, l2))),
            _ => None,
        }
    }
}

/// A vector clock for tracking causality in distributed systems.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorClock {
    /// The clock values for each process.
    #[serde(flatten)]
    pub values: HashMap<String, u64>,
}

impl VectorClock {
    /// Create a new empty vector clock.
    pub fn new() -> Self {
        VectorClock {
            values: HashMap::new(),
        }
    }

    /// Create a vector clock from a map of process IDs to clock values.
    pub fn from_map(values: HashMap<String, u64>) -> Self {
        VectorClock { values }
    }

    /// Increment the clock for a specific process.
    pub fn increment(&mut self, process: &str) {
        *self.values.entry(process.to_string()).or_insert(0) += 1;
    }

    /// Get the clock value for a specific process.
    pub fn get(&self, process: &str) -> u64 {
        self.values.get(process).copied().unwrap_or(0)
    }

    /// Merge this clock with another, taking the maximum for each process.
    pub fn merge(&mut self, other: &VectorClock) {
        for (process, &value) in &other.values {
            let entry = self.values.entry(process.clone()).or_insert(0);
            *entry = (*entry).max(value);
        }
    }

    /// Check if this clock happens before another.
    ///
    /// A vector clock `a` happens before `b` if all components of `a`
    /// are less than or equal to the corresponding components in `b`,
    /// and at least one component is strictly less.
    pub fn happens_before(&self, other: &VectorClock) -> bool {
        let mut at_least_one_less = false;

        // Check all processes in self
        for (process, &value) in &self.values {
            let other_value = other.get(process);
            if value > other_value {
                return false;
            }
            if value < other_value {
                at_least_one_less = true;
            }
        }

        // Check processes only in other
        for (process, &value) in &other.values {
            if !self.values.contains_key(process) && value > 0 {
                at_least_one_less = true;
            }
        }

        at_least_one_less
    }

    /// Check if this clock is concurrent with another.
    pub fn concurrent_with(&self, other: &VectorClock) -> bool {
        !self.happens_before(other) && !other.happens_before(self)
    }
}

impl Default for VectorClock {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialOrd for VectorClock {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self == other {
            Some(std::cmp::Ordering::Equal)
        } else if self.happens_before(other) {
            Some(std::cmp::Ordering::Less)
        } else if other.happens_before(self) {
            Some(std::cmp::Ordering::Greater)
        } else {
            None // Concurrent
        }
    }
}

/// A single step in an execution trace.
///
/// Each step represents a state transition in the system, capturing:
/// - The clock value at this step
/// - The action/operation that caused this step
/// - The variable values after this step
/// - Optional parameters for the action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStep {
    /// The clock value at this step.
    pub clock: TraceClock,

    /// The name of the action/operation that led to this state.
    ///
    /// This should match an action name in the TLA+ specification.
    pub action: String,

    /// The variable values after this step.
    ///
    /// Keys are variable names (matching TLA+ variables),
    /// values are the current state of those variables.
    pub vars: HashMap<String, serde_json::Value>,

    /// Optional parameters for the action.
    ///
    /// These can be used to pass additional context about the operation,
    /// such as the key/value for a write operation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, serde_json::Value>>,

    /// Optional process/node identifier for distributed traces.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process: Option<String>,

    /// Optional metadata for debugging and analysis.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

impl TraceStep {
    /// Create a new trace step with the given clock, action, and variables.
    pub fn new(
        clock: TraceClock,
        action: impl Into<String>,
        vars: HashMap<String, serde_json::Value>,
    ) -> Self {
        TraceStep {
            clock,
            action: action.into(),
            vars,
            params: None,
            process: None,
            metadata: None,
        }
    }

    /// Add parameters to this step.
    pub fn with_params(mut self, params: HashMap<String, serde_json::Value>) -> Self {
        self.params = Some(params);
        self
    }

    /// Set the process identifier for this step.
    pub fn with_process(mut self, process: impl Into<String>) -> Self {
        self.process = Some(process.into());
        self
    }

    /// Add metadata to this step.
    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Get a variable value by name.
    pub fn get_var(&self, name: &str) -> Option<&serde_json::Value> {
        self.vars.get(name)
    }

    /// Get a parameter value by name.
    pub fn get_param(&self, name: &str) -> Option<&serde_json::Value> {
        self.params.as_ref().and_then(|p| p.get(name))
    }
}

/// A complete trace file containing multiple steps.
///
/// Provides methods for reading, writing, and validating trace files
/// in NDJSON format.
#[derive(Debug, Clone)]
pub struct TraceFile {
    /// The steps in this trace.
    pub steps: Vec<TraceStep>,

    /// Optional metadata about the trace.
    pub metadata: Option<TraceMetadata>,
}

/// Metadata about a trace file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceMetadata {
    /// The TLA+ specification this trace was generated from/for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_name: Option<String>,

    /// Version of the trace format.
    #[serde(default = "default_version")]
    pub version: String,

    /// Timestamp when the trace was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Additional properties.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

fn default_version() -> String {
    "1.0".to_string()
}

impl Default for TraceMetadata {
    fn default() -> Self {
        TraceMetadata {
            spec_name: None,
            version: default_version(),
            created_at: None,
            extra: HashMap::new(),
        }
    }
}

impl TraceFile {
    /// Create a new empty trace file.
    pub fn new() -> Self {
        TraceFile {
            steps: Vec::new(),
            metadata: None,
        }
    }

    /// Create a trace file from a vector of steps.
    pub fn from_steps(steps: Vec<TraceStep>) -> Self {
        TraceFile {
            steps,
            metadata: None,
        }
    }

    /// Add metadata to this trace file.
    pub fn with_metadata(mut self, metadata: TraceMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Add a step to the trace.
    pub fn push(&mut self, step: TraceStep) {
        self.steps.push(step);
    }

    /// Get the number of steps in this trace.
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Check if the trace is empty.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Get an iterator over the steps.
    pub fn iter(&self) -> impl Iterator<Item = &TraceStep> {
        self.steps.iter()
    }

    /// Read a trace file from a path.
    ///
    /// The file should be in NDJSON format, with each line being a JSON
    /// object representing a [`TraceStep`].
    ///
    /// Optionally, the first line may be a metadata object with a
    /// `"_type": "metadata"` field.
    pub fn read_from_path(path: &Path) -> Result<Self, TraceError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::read_from_reader(reader)
    }

    /// Read a trace file from a reader.
    pub fn read_from_reader<R: BufRead>(reader: R) -> Result<Self, TraceError> {
        let mut steps = Vec::new();
        let mut metadata = None;

        for (line_num, line_result) in reader.lines().enumerate() {
            let line = line_result?;
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                continue;
            }

            // Try to parse as a JSON object
            let value: serde_json::Value =
                serde_json::from_str(trimmed).map_err(|e| TraceError::Json {
                    line: line_num + 1,
                    source: e,
                })?;

            // Check if this is metadata
            if let Some(obj) = value.as_object() {
                if obj.get("_type").and_then(|v| v.as_str()) == Some("metadata") {
                    metadata = Some(serde_json::from_value(value.clone()).map_err(|e| {
                        TraceError::Json {
                            line: line_num + 1,
                            source: e,
                        }
                    })?);
                    continue;
                }
            }

            // Parse as a trace step
            let step: TraceStep = serde_json::from_value(value).map_err(|e| TraceError::Json {
                line: line_num + 1,
                source: e,
            })?;
            steps.push(step);
        }

        if steps.is_empty() {
            return Err(TraceError::EmptyTrace);
        }

        Ok(TraceFile { steps, metadata })
    }

    /// Write this trace file to a path.
    pub fn write_to_path(&self, path: &Path) -> Result<(), TraceError> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        self.write_to_writer(writer)
    }

    /// Write this trace file to a writer.
    pub fn write_to_writer<W: Write>(&self, mut writer: W) -> Result<(), TraceError> {
        // Write metadata if present
        if let Some(ref metadata) = self.metadata {
            let mut meta_value = serde_json::to_value(metadata)
                .map_err(|e| TraceError::Json { line: 0, source: e })?;
            if let Some(obj) = meta_value.as_object_mut() {
                obj.insert(
                    "_type".to_string(),
                    serde_json::Value::String("metadata".to_string()),
                );
            }
            writeln!(
                writer,
                "{}",
                serde_json::to_string(&meta_value)
                    .map_err(|e| TraceError::Json { line: 0, source: e })?
            )?;
        }

        // Write each step
        for step in &self.steps {
            let json =
                serde_json::to_string(step).map_err(|e| TraceError::Json { line: 0, source: e })?;
            writeln!(writer, "{}", json)?;
        }

        Ok(())
    }

    /// Validate that the trace has monotonically increasing clocks.
    ///
    /// For logical clocks, each step should have a clock value greater
    /// than the previous step. For vector clocks, each step should
    /// happen-after or be concurrent with the previous step (no going back).
    pub fn validate_clock_ordering(&self) -> Result<(), TraceError> {
        for (i, window) in self.steps.windows(2).enumerate() {
            let prev = &window[0];
            let curr = &window[1];

            // Check that we're not going backwards in time
            if curr.clock.happens_before(&prev.clock) {
                return Err(TraceError::ClockOrdering {
                    step: i + 1,
                    message: format!(
                        "Step {} has clock {:?} which happens before step {}'s clock {:?}",
                        i + 1,
                        curr.clock,
                        i,
                        prev.clock
                    ),
                });
            }
        }
        Ok(())
    }

    /// Get all unique action names in this trace.
    pub fn action_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.steps.iter().map(|s| s.action.as_str()).collect();
        names.sort();
        names.dedup();
        names
    }

    /// Get all unique variable names in this trace.
    pub fn variable_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .steps
            .iter()
            .flat_map(|s| s.vars.keys().map(|k| k.as_str()))
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// Filter steps by action name.
    pub fn filter_by_action(&self, action: &str) -> Vec<&TraceStep> {
        self.steps.iter().filter(|s| s.action == action).collect()
    }

    /// Filter steps by process.
    pub fn filter_by_process(&self, process: &str) -> Vec<&TraceStep> {
        self.steps
            .iter()
            .filter(|s| s.process.as_deref() == Some(process))
            .collect()
    }
}

impl Default for TraceFile {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for TraceFile {
    type Item = TraceStep;
    type IntoIter = std::vec::IntoIter<TraceStep>;

    fn into_iter(self) -> Self::IntoIter {
        self.steps.into_iter()
    }
}

impl<'a> IntoIterator for &'a TraceFile {
    type Item = &'a TraceStep;
    type IntoIter = std::slice::Iter<'a, TraceStep>;

    fn into_iter(self) -> Self::IntoIter {
        self.steps.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_logical_clock_ordering() {
        let c1 = TraceClock::logical(1);
        let c2 = TraceClock::logical(2);
        let c3 = TraceClock::logical(2);

        assert!(c1.happens_before(&c2));
        assert!(!c2.happens_before(&c1));
        assert!(!c2.happens_before(&c3)); // Equal, not before
    }

    #[test]
    fn test_vector_clock_ordering() {
        let mut vc1 = VectorClock::new();
        vc1.values.insert("A".to_string(), 1);
        vc1.values.insert("B".to_string(), 0);

        let mut vc2 = VectorClock::new();
        vc2.values.insert("A".to_string(), 1);
        vc2.values.insert("B".to_string(), 1);

        assert!(vc1.happens_before(&vc2));
        assert!(!vc2.happens_before(&vc1));

        // Concurrent clocks
        let mut vc3 = VectorClock::new();
        vc3.values.insert("A".to_string(), 2);
        vc3.values.insert("B".to_string(), 0);

        assert!(!vc2.happens_before(&vc3));
        assert!(!vc3.happens_before(&vc2));
        assert!(vc2.concurrent_with(&vc3));
    }

    #[test]
    fn test_trace_step_serialization() {
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), serde_json::json!(42));

        let mut params = HashMap::new();
        params.insert("key".to_string(), serde_json::json!("foo"));

        let step = TraceStep::new(TraceClock::logical(1), "Write", vars).with_params(params);

        let json = serde_json::to_string(&step).unwrap();
        let parsed: TraceStep = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.action, "Write");
        assert_eq!(parsed.clock, TraceClock::logical(1));
        assert_eq!(parsed.get_var("x"), Some(&serde_json::json!(42)));
        assert_eq!(parsed.get_param("key"), Some(&serde_json::json!("foo")));
    }

    #[test]
    fn test_trace_file_read_write() {
        let trace_data = r#"{"clock":1,"action":"Init","vars":{"x":0}}
{"clock":2,"action":"Write","vars":{"x":42},"params":{"value":42}}
{"clock":3,"action":"Read","vars":{"x":42}}"#;

        let reader = Cursor::new(trace_data);
        let trace = TraceFile::read_from_reader(reader).unwrap();

        assert_eq!(trace.len(), 3);
        assert_eq!(trace.steps[0].action, "Init");
        assert_eq!(trace.steps[1].action, "Write");
        assert_eq!(trace.steps[2].action, "Read");

        // Write and re-read
        let mut output = Vec::new();
        trace.write_to_writer(&mut output).unwrap();

        let reader2 = Cursor::new(output);
        let trace2 = TraceFile::read_from_reader(reader2).unwrap();

        assert_eq!(trace2.len(), 3);
    }

    #[test]
    fn test_trace_validation() {
        // Valid trace
        let trace = TraceFile::from_steps(vec![
            TraceStep::new(TraceClock::logical(1), "A", HashMap::new()),
            TraceStep::new(TraceClock::logical(2), "B", HashMap::new()),
            TraceStep::new(TraceClock::logical(3), "C", HashMap::new()),
        ]);
        assert!(trace.validate_clock_ordering().is_ok());

        // Invalid trace (clock goes backwards)
        let invalid_trace = TraceFile::from_steps(vec![
            TraceStep::new(TraceClock::logical(2), "A", HashMap::new()),
            TraceStep::new(TraceClock::logical(1), "B", HashMap::new()),
        ]);
        assert!(invalid_trace.validate_clock_ordering().is_err());
    }

    #[test]
    fn test_trace_queries() {
        let trace = TraceFile::from_steps(vec![
            TraceStep::new(
                TraceClock::logical(1),
                "Write",
                [("x".to_string(), serde_json::json!(1))].into(),
            )
            .with_process("P1"),
            TraceStep::new(
                TraceClock::logical(2),
                "Read",
                [("x".to_string(), serde_json::json!(1))].into(),
            )
            .with_process("P2"),
            TraceStep::new(
                TraceClock::logical(3),
                "Write",
                [("x".to_string(), serde_json::json!(2))].into(),
            )
            .with_process("P1"),
        ]);

        assert_eq!(trace.action_names(), vec!["Read", "Write"]);
        assert_eq!(trace.variable_names(), vec!["x"]);
        assert_eq!(trace.filter_by_action("Write").len(), 2);
        assert_eq!(trace.filter_by_process("P1").len(), 2);
        assert_eq!(trace.filter_by_process("P2").len(), 1);
    }
}
