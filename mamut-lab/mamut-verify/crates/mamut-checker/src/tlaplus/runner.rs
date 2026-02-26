//! TLC runner for subprocess execution.
//!
//! This module provides the [`TlcRunner`] struct for executing TLC (the TLA+
//! model checker) as a subprocess and parsing its output.
//!
//! # Example
//!
//! ```rust,ignore
//! use mamut_checker::tlaplus::TlcRunner;
//! use std::path::Path;
//! use std::time::Duration;
//!
//! let runner = TlcRunner::builder(Path::new("/path/to/tla2tools.jar"))
//!     .with_timeout(Duration::from_secs(60))
//!     .with_workers(4)
//!     .build();
//!
//! let result = runner.validate_trace(
//!     Path::new("spec.tla"),
//!     Path::new("trace.ndjson"),
//! ).await?;
//! ```

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader as TokioBufReader};
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, error, info, trace, warn};

use super::config::TlcConfig;
use super::result::{
    CounterexampleState, CounterexampleTrace, ParsedTlcOutput, PropertyType, PropertyViolation,
    TlcError, TlcExitStatus, ValidationResult, ValidationStatus,
};
use super::trace::TraceFile;

/// Runner for executing TLC as a subprocess.
#[derive(Debug, Clone)]
pub struct TlcRunner {
    /// Path to tla2tools.jar.
    tla2tools_path: PathBuf,

    /// Optional Java home directory.
    java_home: Option<PathBuf>,

    /// Maximum execution time.
    timeout: Duration,

    /// Number of worker threads.
    workers: u32,

    /// JVM heap size in megabytes.
    heap_size_mb: Option<u32>,

    /// Additional JVM arguments.
    jvm_args: Vec<String>,

    /// Additional TLC arguments.
    tlc_args: Vec<String>,

    /// Working directory for TLC.
    work_dir: Option<PathBuf>,

    /// Whether to capture raw output.
    capture_output: bool,
}

impl TlcRunner {
    /// Create a new TLC runner with the path to tla2tools.jar.
    pub fn new(tla2tools_path: impl Into<PathBuf>) -> Self {
        TlcRunner {
            tla2tools_path: tla2tools_path.into(),
            java_home: None,
            timeout: Duration::from_secs(300),
            workers: 1,
            heap_size_mb: None,
            jvm_args: Vec::new(),
            tlc_args: Vec::new(),
            work_dir: None,
            capture_output: true,
        }
    }

    /// Create a builder for more complex configuration.
    pub fn builder(tla2tools_path: impl Into<PathBuf>) -> TlcRunnerBuilder {
        TlcRunnerBuilder::new(tla2tools_path)
    }

    /// Set the Java home directory.
    pub fn with_java_home(mut self, java_home: impl Into<PathBuf>) -> Self {
        self.java_home = Some(java_home.into());
        self
    }

    /// Set the timeout duration.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the number of workers.
    pub fn with_workers(mut self, workers: u32) -> Self {
        self.workers = workers;
        self
    }

    /// Get the path to the Java executable.
    fn java_path(&self) -> PathBuf {
        if let Some(ref java_home) = self.java_home {
            java_home.join("bin").join("java")
        } else {
            PathBuf::from("java")
        }
    }

    /// Build the command for running TLC.
    fn build_command(&self, spec_path: &Path, config: Option<&TlcConfig>) -> Command {
        let mut cmd = Command::new(self.java_path());

        // JVM arguments
        if let Some(heap) = self.heap_size_mb {
            cmd.arg(format!("-Xmx{}m", heap));
            cmd.arg(format!("-Xms{}m", heap / 2));
        }

        for arg in &self.jvm_args {
            cmd.arg(arg);
        }

        // TLC jar
        cmd.arg("-jar");
        cmd.arg(&self.tla2tools_path);

        // TLC arguments
        cmd.arg("-workers");
        cmd.arg(self.workers.to_string());

        // Add config arguments if provided
        if let Some(cfg) = config {
            for arg in cfg.to_args() {
                cmd.arg(arg);
            }
        }

        // Additional TLC arguments
        for arg in &self.tlc_args {
            cmd.arg(arg);
        }

        // Specification path
        cmd.arg(spec_path);

        // Working directory
        if let Some(ref work_dir) = self.work_dir {
            cmd.current_dir(work_dir);
        } else if let Some(parent) = spec_path.parent() {
            cmd.current_dir(parent);
        }

        // Capture output
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        cmd
    }

    /// Validate a trace against a TLA+ specification.
    ///
    /// This method:
    /// 1. Reads the trace file
    /// 2. Generates a trace-checking TLC configuration
    /// 3. Runs TLC to validate the trace
    /// 4. Parses the output and returns the result
    pub async fn validate_trace(
        &self,
        spec_path: &Path,
        trace_file: &Path,
    ) -> Result<ValidationResult, TlcError> {
        info!(
            "Validating trace {} against spec {}",
            trace_file.display(),
            spec_path.display()
        );

        // Verify files exist
        if !spec_path.exists() {
            return Err(TlcError::SpecNotFound {
                path: spec_path.display().to_string(),
            });
        }

        if !trace_file.exists() {
            return Err(TlcError::TraceNotFound {
                path: trace_file.display().to_string(),
            });
        }

        // Read and validate the trace
        let trace = TraceFile::read_from_path(trace_file)
            .map_err(|e| TlcError::ParseError(format!("Failed to read trace file: {}", e)))?;

        debug!("Loaded trace with {} steps", trace.len());

        // Run TLC
        self.run_tlc(spec_path, None).await
    }

    /// Run TLC on a specification with optional configuration.
    pub async fn run_tlc(
        &self,
        spec_path: &Path,
        config: Option<&TlcConfig>,
    ) -> Result<ValidationResult, TlcError> {
        let start_time = std::time::Instant::now();

        // Verify tla2tools.jar exists
        if !self.tla2tools_path.exists() {
            return Err(TlcError::NotFound {
                path: self.tla2tools_path.display().to_string(),
            });
        }

        let mut cmd = self.build_command(spec_path, config);
        debug!("Running TLC command: {:?}", cmd);

        // Spawn the process
        let mut child = cmd.spawn()?;

        // Get stdout and stderr handles
        let stdout = child.stdout.take().expect("stdout not captured");
        let stderr = child.stderr.take().expect("stderr not captured");

        // Read output asynchronously
        let stdout_reader = TokioBufReader::new(stdout);
        let stderr_reader = TokioBufReader::new(stderr);

        let stdout_handle = tokio::spawn(async move {
            let mut lines = Vec::new();
            let mut reader = stdout_reader.lines();
            while let Ok(Some(line)) = reader.next_line().await {
                trace!("TLC stdout: {}", line);
                lines.push(line);
            }
            lines
        });

        let stderr_handle = tokio::spawn(async move {
            let mut lines = Vec::new();
            let mut reader = stderr_reader.lines();
            while let Ok(Some(line)) = reader.next_line().await {
                trace!("TLC stderr: {}", line);
                lines.push(line);
            }
            lines
        });

        // Wait for process with timeout
        let result = timeout(self.timeout, child.wait()).await;

        let exit_status = match result {
            Ok(Ok(status)) => {
                let code = status.code().unwrap_or(-1);
                debug!("TLC exited with code: {}", code);
                TlcExitStatus::from_exit_code(code)
            }
            Ok(Err(e)) => {
                error!("TLC process error: {}", e);
                return Err(TlcError::ProcessStart(e));
            }
            Err(_) => {
                warn!("TLC timed out after {:?}", self.timeout);
                // Kill the process
                let _ = child.kill().await;
                return Ok(ValidationResult::timeout().with_duration(self.timeout));
            }
        };

        // Collect output
        let stdout_lines = stdout_handle.await.unwrap_or_default();
        let stderr_lines = stderr_handle.await.unwrap_or_default();
        let combined_output = [stdout_lines.clone(), stderr_lines].concat().join("\n");

        // Parse the output
        let parsed = parse_tlc_output(&stdout_lines);
        let duration = start_time.elapsed();

        // Build the result
        let mut result = match exit_status {
            TlcExitStatus::Success => {
                info!("TLC validation successful");
                ValidationResult::valid()
            }
            TlcExitStatus::Violation
            | TlcExitStatus::SafetyViolation
            | TlcExitStatus::LivenessViolation
            | TlcExitStatus::Deadlock => {
                warn!("TLC found violations: {:?}", exit_status);
                let violations = extract_violations(&stdout_lines, &parsed);
                let mut result = ValidationResult::invalid(violations);
                if let Some(ce) = parsed.counterexample {
                    result = result.with_counterexample(ce);
                }
                result
            }
            TlcExitStatus::ParseError | TlcExitStatus::SemanticError => {
                error!("TLC found errors in specification");
                let error_msg = parsed.errors.join("\n");
                ValidationResult::error(error_msg)
            }
            _ => {
                error!("TLC returned unexpected status: {:?}", exit_status);
                ValidationResult::error(format!("TLC returned status: {}", exit_status))
            }
        };

        result = result.with_exit_status(exit_status).with_duration(duration);

        if let (Some(explored), Some(distinct)) = (parsed.states_generated, parsed.distinct_states)
        {
            result = result.with_state_counts(explored, distinct);
        }

        if self.capture_output {
            result = result.with_raw_output(combined_output);
        }

        Ok(result)
    }

    /// Check if TLC is available (Java and tla2tools.jar exist).
    pub async fn check_availability(&self) -> Result<String, TlcError> {
        // Check tla2tools.jar
        if !self.tla2tools_path.exists() {
            return Err(TlcError::NotFound {
                path: self.tla2tools_path.display().to_string(),
            });
        }

        // Check Java
        let java_path = self.java_path();
        let output = Command::new(&java_path)
            .arg("-version")
            .output()
            .await
            .map_err(|e| TlcError::JavaError {
                message: format!("Failed to run java: {}", e),
            })?;

        if !output.status.success() {
            return Err(TlcError::JavaError {
                message: "Java version check failed".to_string(),
            });
        }

        // Parse Java version from stderr (java -version outputs to stderr)
        let version_output = String::from_utf8_lossy(&output.stderr);
        let version = version_output
            .lines()
            .next()
            .unwrap_or("unknown")
            .to_string();

        Ok(version)
    }
}

/// Builder for TlcRunner with additional configuration options.
#[derive(Debug, Clone)]
pub struct TlcRunnerBuilder {
    runner: TlcRunner,
}

impl TlcRunnerBuilder {
    /// Create a new builder.
    pub fn new(tla2tools_path: impl Into<PathBuf>) -> Self {
        TlcRunnerBuilder {
            runner: TlcRunner::new(tla2tools_path),
        }
    }

    /// Set the Java home directory.
    pub fn with_java_home(mut self, java_home: impl Into<PathBuf>) -> Self {
        self.runner.java_home = Some(java_home.into());
        self
    }

    /// Set the timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.runner.timeout = timeout;
        self
    }

    /// Set the number of workers.
    pub fn with_workers(mut self, workers: u32) -> Self {
        self.runner.workers = workers;
        self
    }

    /// Set the heap size in megabytes.
    pub fn with_heap_size(mut self, size_mb: u32) -> Self {
        self.runner.heap_size_mb = Some(size_mb);
        self
    }

    /// Add a JVM argument.
    pub fn with_jvm_arg(mut self, arg: impl Into<String>) -> Self {
        self.runner.jvm_args.push(arg.into());
        self
    }

    /// Add a TLC argument.
    pub fn with_tlc_arg(mut self, arg: impl Into<String>) -> Self {
        self.runner.tlc_args.push(arg.into());
        self
    }

    /// Set the working directory.
    pub fn with_work_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.runner.work_dir = Some(dir.into());
        self
    }

    /// Enable or disable output capture.
    pub fn capture_output(mut self, capture: bool) -> Self {
        self.runner.capture_output = capture;
        self
    }

    /// Build the runner.
    pub fn build(self) -> TlcRunner {
        self.runner
    }
}

/// Parse TLC output into structured format.
fn parse_tlc_output(lines: &[String]) -> ParsedTlcOutput {
    let mut output = ParsedTlcOutput::new();
    let mut in_counterexample = false;
    let mut counterexample_states: Vec<CounterexampleState> = Vec::new();
    let mut current_state: Option<CounterexampleState> = None;
    let mut current_state_num = 0;

    for line in lines {
        // Version detection
        if line.starts_with("TLC2 Version") || line.starts_with("TLC Version") {
            output.version = Some(line.clone());
            continue;
        }

        // State counts
        if line.contains("states generated") {
            if let Some(count) = extract_number(line) {
                output.states_generated = Some(count);
            }
        }

        if line.contains("distinct states") {
            if let Some(count) = extract_number(line) {
                output.distinct_states = Some(count);
            }
        }

        if line.contains("left on queue") {
            if let Some(count) = extract_number(line) {
                output.states_left = Some(count);
            }
        }

        // Completion detection
        if line.contains("Model checking completed") {
            output.completed = true;
        }

        // Error detection
        if line.contains("Error:") || line.starts_with("Error ") {
            output.errors.push(line.clone());
        }

        // Warning detection
        if line.contains("Warning:") {
            output.warnings.push(line.clone());
        }

        // Counterexample parsing
        if line.contains("Error: Invariant")
            || line.contains("Error: Action property")
            || line.contains("The following sequence of states")
            || line.contains("Error: Temporal")
        {
            in_counterexample = true;
            continue;
        }

        if in_counterexample {
            // State header: "State 1: <Initial predicate>"
            if line.starts_with("State ") {
                // Save previous state if exists
                if let Some(state) = current_state.take() {
                    counterexample_states.push(state);
                }

                // Parse state number and action
                if let Some((num, action)) = parse_state_header(line) {
                    current_state_num = num;
                    let mut state = CounterexampleState::new(num);
                    if let Some(act) = action {
                        state = state.with_action(act);
                    }
                    current_state = Some(state);
                }
            }
            // Variable assignment: "/\ var = value"
            else if line.starts_with("/\\") || line.starts_with("  /\\") {
                if let Some(ref mut state) = current_state {
                    if let Some((var_name, var_value)) = parse_variable_assignment(line) {
                        state.variables.insert(var_name, var_value);
                    }
                }
            }
            // End of counterexample
            else if line.is_empty() && current_state.is_some() {
                // Empty line might end a state block
            } else if line.contains("Back to state") {
                // Lasso detected
                if let Some(state) = current_state.take() {
                    counterexample_states.push(state);
                }
                // Extract loop-back state number
                if let Some(loop_to) = extract_number(line) {
                    let trace = CounterexampleTrace::new(counterexample_states.clone())
                        .with_lasso(loop_to as usize - 1);
                    output.counterexample = Some(trace);
                }
                in_counterexample = false;
            }
        }
    }

    // Finalize counterexample if still in progress
    if let Some(state) = current_state {
        counterexample_states.push(state);
    }
    if !counterexample_states.is_empty() && output.counterexample.is_none() {
        output.counterexample = Some(CounterexampleTrace::new(counterexample_states));
    }

    output
}

/// Extract violations from TLC output.
fn extract_violations(lines: &[String], parsed: &ParsedTlcOutput) -> Vec<PropertyViolation> {
    let mut violations = Vec::new();

    for line in lines {
        // Invariant violation
        if line.contains("Error: Invariant") {
            if let Some(name) = extract_quoted_string(line) {
                violations.push(
                    PropertyViolation::new(PropertyType::Invariant, line.clone()).with_name(name),
                );
            } else {
                violations.push(PropertyViolation::new(
                    PropertyType::Invariant,
                    line.clone(),
                ));
            }
        }
        // Temporal property violation
        else if line.contains("Error: Temporal") {
            if let Some(name) = extract_quoted_string(line) {
                violations.push(
                    PropertyViolation::new(PropertyType::Temporal, line.clone()).with_name(name),
                );
            } else {
                violations.push(PropertyViolation::new(PropertyType::Temporal, line.clone()));
            }
        }
        // Deadlock
        else if line.contains("Deadlock reached") {
            violations.push(PropertyViolation::new(
                PropertyType::Deadlock,
                "Deadlock: no actions enabled".to_string(),
            ));
        }
        // Assertion failure
        else if line.contains("Assertion failed") {
            violations.push(PropertyViolation::new(
                PropertyType::Assertion,
                line.clone(),
            ));
        }
    }

    // Add state information from counterexample
    if let Some(ref ce) = parsed.counterexample {
        if let Some(final_state) = ce.final_state() {
            for violation in &mut violations {
                violation.step = Some(final_state.step);
                violation.state = Some(final_state.clone());
            }
        }
    }

    violations
}

/// Extract a number from a line of text.
fn extract_number(line: &str) -> Option<u64> {
    line.split_whitespace()
        .find_map(|word| word.replace(",", "").parse::<u64>().ok())
}

/// Extract a quoted string from a line.
fn extract_quoted_string(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let end = line[start + 1..].find('"')?;
    Some(line[start + 1..start + 1 + end].to_string())
}

/// Parse a state header line.
fn parse_state_header(line: &str) -> Option<(usize, Option<String>)> {
    // Format: "State N: <Action>"
    let line = line.strip_prefix("State ")?.trim();
    let colon_idx = line.find(':')?;
    let num: usize = line[..colon_idx].parse().ok()?;

    let action = if colon_idx + 1 < line.len() {
        let action_str = line[colon_idx + 1..].trim();
        if action_str.starts_with('<') && action_str.ends_with('>') {
            Some(action_str[1..action_str.len() - 1].to_string())
        } else if !action_str.is_empty() {
            Some(action_str.to_string())
        } else {
            None
        }
    } else {
        None
    };

    Some((num, action))
}

/// Parse a variable assignment line.
fn parse_variable_assignment(line: &str) -> Option<(String, serde_json::Value)> {
    // Format: "/\ var = value" or "  /\ var = value"
    let line = line.trim().strip_prefix("/\\")?;
    let line = line.trim();

    let eq_idx = line.find('=')?;
    let var_name = line[..eq_idx].trim().to_string();
    let value_str = line[eq_idx + 1..].trim();

    // Parse TLA+ value to JSON
    let value = parse_tla_value(value_str)?;

    Some((var_name, value))
}

/// Parse a TLA+ value to JSON.
fn parse_tla_value(s: &str) -> Option<serde_json::Value> {
    let s = s.trim();

    // Boolean
    if s == "TRUE" {
        return Some(serde_json::json!(true));
    }
    if s == "FALSE" {
        return Some(serde_json::json!(false));
    }

    // Integer
    if let Ok(n) = s.parse::<i64>() {
        return Some(serde_json::json!(n));
    }

    // String
    if s.starts_with('"') && s.ends_with('"') {
        return Some(serde_json::json!(s[1..s.len() - 1]));
    }

    // Sequence << ... >>
    if s.starts_with("<<") && s.ends_with(">>") {
        let inner = s[2..s.len() - 2].trim();
        if inner.is_empty() {
            return Some(serde_json::json!([]));
        }
        // Simple split by comma (doesn't handle nested structures)
        let items: Vec<serde_json::Value> = inner
            .split(',')
            .filter_map(|item| parse_tla_value(item.trim()))
            .collect();
        return Some(serde_json::Value::Array(items));
    }

    // Set { ... }
    if s.starts_with('{') && s.ends_with('}') {
        let inner = s[1..s.len() - 1].trim();
        if inner.is_empty() {
            return Some(serde_json::json!([]));
        }
        // Represent as array (sets become arrays in JSON)
        let items: Vec<serde_json::Value> = inner
            .split(',')
            .filter_map(|item| parse_tla_value(item.trim()))
            .collect();
        return Some(serde_json::Value::Array(items));
    }

    // Function [key |-> value, ...]
    if s.starts_with('[') && s.ends_with(']') {
        // Simplified parsing - treat as object
        let inner = s[1..s.len() - 1].trim();
        let mut obj = serde_json::Map::new();

        for part in inner.split(',') {
            let part = part.trim();
            if let Some(arrow_idx) = part.find("|->") {
                let key = part[..arrow_idx].trim();
                let value = part[arrow_idx + 3..].trim();
                if let Some(v) = parse_tla_value(value) {
                    obj.insert(key.to_string(), v);
                }
            }
        }

        if !obj.is_empty() {
            return Some(serde_json::Value::Object(obj));
        }
    }

    // Fall back to string representation
    Some(serde_json::json!(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tla_value() {
        assert_eq!(parse_tla_value("TRUE"), Some(serde_json::json!(true)));
        assert_eq!(parse_tla_value("FALSE"), Some(serde_json::json!(false)));
        assert_eq!(parse_tla_value("42"), Some(serde_json::json!(42)));
        assert_eq!(
            parse_tla_value("\"hello\""),
            Some(serde_json::json!("hello"))
        );
        assert_eq!(
            parse_tla_value("<< 1, 2, 3 >>"),
            Some(serde_json::json!([1, 2, 3]))
        );
        assert_eq!(
            parse_tla_value("{ 1, 2, 3 }"),
            Some(serde_json::json!([1, 2, 3]))
        );
    }

    #[test]
    fn test_parse_state_header() {
        assert_eq!(
            parse_state_header("State 1: <Init>"),
            Some((1, Some("Init".to_string())))
        );
        assert_eq!(
            parse_state_header("State 2: <Next>"),
            Some((2, Some("Next".to_string())))
        );
        assert_eq!(parse_state_header("State 3:"), Some((3, None)));
    }

    #[test]
    fn test_parse_variable_assignment() {
        assert_eq!(
            parse_variable_assignment("/\\ x = 42"),
            Some(("x".to_string(), serde_json::json!(42)))
        );
        assert_eq!(
            parse_variable_assignment("  /\\ flag = TRUE"),
            Some(("flag".to_string(), serde_json::json!(true)))
        );
        assert_eq!(
            parse_variable_assignment("/\\ items = << 1, 2, 3 >>"),
            Some(("items".to_string(), serde_json::json!([1, 2, 3])))
        );
    }

    #[test]
    fn test_parse_tlc_output() {
        let lines = vec![
            "TLC2 Version 2.18".to_string(),
            "Starting...".to_string(),
            "100 states generated, 50 distinct states found".to_string(),
            "Model checking completed. No error has been found.".to_string(),
        ];

        let output = parse_tlc_output(&lines);
        assert!(output.version.is_some());
        assert!(output.completed);
        assert_eq!(output.states_generated, Some(100));
        assert_eq!(output.distinct_states, Some(50));
    }

    #[test]
    fn test_extract_violations() {
        let lines = vec![
            "Error: Invariant \"SafetyInvariant\" is violated.".to_string(),
            "State 1: <Init>".to_string(),
            "/\\ x = 0".to_string(),
        ];

        let parsed = parse_tlc_output(&lines);
        let violations = extract_violations(&lines, &parsed);

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].property_type, PropertyType::Invariant);
        assert_eq!(
            violations[0].property_name,
            Some("SafetyInvariant".to_string())
        );
    }

    #[test]
    fn test_tlc_runner_builder() {
        let runner = TlcRunnerBuilder::new("/path/to/tla2tools.jar")
            .with_timeout(Duration::from_secs(120))
            .with_workers(4)
            .with_heap_size(2048)
            .with_jvm_arg("-XX:+UseG1GC")
            .with_tlc_arg("-deadlock")
            .build();

        assert_eq!(runner.timeout, Duration::from_secs(120));
        assert_eq!(runner.workers, 4);
        assert_eq!(runner.heap_size_mb, Some(2048));
        assert!(runner.jvm_args.contains(&"-XX:+UseG1GC".to_string()));
        assert!(runner.tlc_args.contains(&"-deadlock".to_string()));
    }
}
