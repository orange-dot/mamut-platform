//! TLC configuration generation.
//!
//! This module provides utilities for generating TLC configuration files
//! that control how TLC runs, including specification of invariants,
//! properties, constants, and state constraints.
//!
//! # Example
//!
//! ```rust,ignore
//! use mamut_checker::tlaplus::config::{TlcConfig, TlcConfigBuilder};
//!
//! let config = TlcConfigBuilder::new("KVStore")
//!     .with_init("Init")
//!     .with_next("Next")
//!     .with_invariant("TypeInvariant")
//!     .with_invariant("SafetyInvariant")
//!     .with_property("EventualConsistency")
//!     .with_constant("MaxKeys", "100")
//!     .with_workers(4)
//!     .build();
//!
//! let cfg_content = config.to_cfg_string();
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::io::{self, Write};
use std::path::Path;

/// TLC configuration for running model checking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlcConfig {
    /// The specification (module) name.
    pub spec_name: String,

    /// The initial state predicate.
    pub init: Option<String>,

    /// The next-state relation.
    pub next: Option<String>,

    /// Invariants to check.
    pub invariants: Vec<String>,

    /// Temporal properties to check.
    pub properties: Vec<String>,

    /// State constraints to limit state space.
    pub state_constraints: Vec<String>,

    /// Action constraints.
    pub action_constraints: Vec<String>,

    /// Constant definitions.
    pub constants: HashMap<String, ConstantValue>,

    /// Symmetry sets for state reduction.
    pub symmetry: Vec<String>,

    /// View definition for abstraction.
    pub view: Option<String>,

    /// Worker configuration.
    pub workers: TlcWorkerConfig,

    /// Whether to check for deadlocks.
    pub check_deadlock: bool,

    /// Depth limit for simulation mode.
    pub depth_limit: Option<u32>,

    /// Seed for random simulation.
    pub seed: Option<u64>,

    /// Additional TLC options.
    pub extra_options: Vec<String>,
}

impl TlcConfig {
    /// Create a new configuration with default settings.
    pub fn new(spec_name: impl Into<String>) -> Self {
        TlcConfig {
            spec_name: spec_name.into(),
            init: None,
            next: None,
            invariants: Vec::new(),
            properties: Vec::new(),
            state_constraints: Vec::new(),
            action_constraints: Vec::new(),
            constants: HashMap::new(),
            symmetry: Vec::new(),
            view: None,
            workers: TlcWorkerConfig::default(),
            check_deadlock: true,
            depth_limit: None,
            seed: None,
            extra_options: Vec::new(),
        }
    }

    /// Create a builder for this configuration.
    pub fn builder(spec_name: impl Into<String>) -> TlcConfigBuilder {
        TlcConfigBuilder::new(spec_name)
    }

    /// Generate the TLC configuration file content (.cfg format).
    pub fn to_cfg_string(&self) -> String {
        let mut output = String::new();

        // Header comment
        output.push_str("\\* TLC Configuration File\n");
        output.push_str(&format!("\\* Generated for: {}\n\n", self.spec_name));

        // Specification
        if let Some(ref init) = self.init {
            output.push_str(&format!("INIT {}\n", init));
        }

        if let Some(ref next) = self.next {
            output.push_str(&format!("NEXT {}\n", next));
        }

        output.push('\n');

        // Invariants
        for inv in &self.invariants {
            output.push_str(&format!("INVARIANT {}\n", inv));
        }

        if !self.invariants.is_empty() {
            output.push('\n');
        }

        // Temporal properties
        for prop in &self.properties {
            output.push_str(&format!("PROPERTY {}\n", prop));
        }

        if !self.properties.is_empty() {
            output.push('\n');
        }

        // State constraints
        for constraint in &self.state_constraints {
            output.push_str(&format!("CONSTRAINT {}\n", constraint));
        }

        if !self.state_constraints.is_empty() {
            output.push('\n');
        }

        // Action constraints
        for constraint in &self.action_constraints {
            output.push_str(&format!("ACTION_CONSTRAINT {}\n", constraint));
        }

        if !self.action_constraints.is_empty() {
            output.push('\n');
        }

        // Constants
        if !self.constants.is_empty() {
            output.push_str("CONSTANTS\n");
            for (name, value) in &self.constants {
                output.push_str(&format!("  {} = {}\n", name, value.to_tla_string()));
            }
            output.push('\n');
        }

        // Symmetry
        for sym in &self.symmetry {
            output.push_str(&format!("SYMMETRY {}\n", sym));
        }

        if !self.symmetry.is_empty() {
            output.push('\n');
        }

        // View
        if let Some(ref view) = self.view {
            output.push_str(&format!("VIEW {}\n\n", view));
        }

        // Check deadlock
        if !self.check_deadlock {
            output.push_str("CHECK_DEADLOCK FALSE\n\n");
        }

        output
    }

    /// Generate TLC command-line arguments.
    pub fn to_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // Worker configuration
        args.push("-workers".to_string());
        args.push(self.workers.num_workers.to_string());

        // Check deadlock
        if !self.check_deadlock {
            args.push("-deadlock".to_string());
        }

        // Depth limit (simulation mode)
        if let Some(depth) = self.depth_limit {
            args.push("-depth".to_string());
            args.push(depth.to_string());
        }

        // Seed
        if let Some(seed) = self.seed {
            args.push("-seed".to_string());
            args.push(seed.to_string());
        }

        // Memory settings
        if let Some(heap) = &self.workers.heap_size_mb {
            args.push(format!("-Xmx{}m", heap));
        }

        // Extra options
        args.extend(self.extra_options.clone());

        args
    }

    /// Write the configuration to a file.
    pub fn write_to_file(&self, path: &Path) -> io::Result<()> {
        let content = self.to_cfg_string();
        let mut file = std::fs::File::create(path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }
}

/// Builder for TLC configurations.
#[derive(Debug, Clone)]
pub struct TlcConfigBuilder {
    config: TlcConfig,
}

impl TlcConfigBuilder {
    /// Create a new configuration builder.
    pub fn new(spec_name: impl Into<String>) -> Self {
        TlcConfigBuilder {
            config: TlcConfig::new(spec_name),
        }
    }

    /// Set the initial state predicate.
    pub fn with_init(mut self, init: impl Into<String>) -> Self {
        self.config.init = Some(init.into());
        self
    }

    /// Set the next-state relation.
    pub fn with_next(mut self, next: impl Into<String>) -> Self {
        self.config.next = Some(next.into());
        self
    }

    /// Add an invariant to check.
    pub fn with_invariant(mut self, invariant: impl Into<String>) -> Self {
        self.config.invariants.push(invariant.into());
        self
    }

    /// Add multiple invariants.
    pub fn with_invariants(
        mut self,
        invariants: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        for inv in invariants {
            self.config.invariants.push(inv.into());
        }
        self
    }

    /// Add a temporal property to check.
    pub fn with_property(mut self, property: impl Into<String>) -> Self {
        self.config.properties.push(property.into());
        self
    }

    /// Add multiple properties.
    pub fn with_properties(
        mut self,
        properties: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        for prop in properties {
            self.config.properties.push(prop.into());
        }
        self
    }

    /// Add a state constraint.
    pub fn with_state_constraint(mut self, constraint: impl Into<String>) -> Self {
        self.config.state_constraints.push(constraint.into());
        self
    }

    /// Add an action constraint.
    pub fn with_action_constraint(mut self, constraint: impl Into<String>) -> Self {
        self.config.action_constraints.push(constraint.into());
        self
    }

    /// Add a constant with a string value.
    pub fn with_constant(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.config
            .constants
            .insert(name.into(), ConstantValue::String(value.into()));
        self
    }

    /// Add a constant with an integer value.
    pub fn with_constant_int(mut self, name: impl Into<String>, value: i64) -> Self {
        self.config
            .constants
            .insert(name.into(), ConstantValue::Integer(value));
        self
    }

    /// Add a constant with a boolean value.
    pub fn with_constant_bool(mut self, name: impl Into<String>, value: bool) -> Self {
        self.config
            .constants
            .insert(name.into(), ConstantValue::Boolean(value));
        self
    }

    /// Add a constant as a model value.
    pub fn with_model_value(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        self.config
            .constants
            .insert(name.clone(), ConstantValue::ModelValue(name));
        self
    }

    /// Add a constant as a set of model values.
    pub fn with_model_value_set(
        mut self,
        name: impl Into<String>,
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let values: Vec<String> = values.into_iter().map(|v| v.into()).collect();
        self.config
            .constants
            .insert(name.into(), ConstantValue::ModelValueSet(values));
        self
    }

    /// Add a symmetry set.
    pub fn with_symmetry(mut self, symmetry: impl Into<String>) -> Self {
        self.config.symmetry.push(symmetry.into());
        self
    }

    /// Set the view definition.
    pub fn with_view(mut self, view: impl Into<String>) -> Self {
        self.config.view = Some(view.into());
        self
    }

    /// Set the number of workers.
    pub fn with_workers(mut self, num_workers: u32) -> Self {
        self.config.workers.num_workers = num_workers;
        self
    }

    /// Set the heap size in megabytes.
    pub fn with_heap_size(mut self, size_mb: u32) -> Self {
        self.config.workers.heap_size_mb = Some(size_mb);
        self
    }

    /// Configure worker settings.
    pub fn with_worker_config(mut self, config: TlcWorkerConfig) -> Self {
        self.config.workers = config;
        self
    }

    /// Enable or disable deadlock checking.
    pub fn check_deadlock(mut self, check: bool) -> Self {
        self.config.check_deadlock = check;
        self
    }

    /// Set depth limit for simulation mode.
    pub fn with_depth_limit(mut self, depth: u32) -> Self {
        self.config.depth_limit = Some(depth);
        self
    }

    /// Set the random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.config.seed = Some(seed);
        self
    }

    /// Add extra TLC command-line options.
    pub fn with_extra_option(mut self, option: impl Into<String>) -> Self {
        self.config.extra_options.push(option.into());
        self
    }

    /// Build the configuration.
    pub fn build(self) -> TlcConfig {
        self.config
    }
}

/// Configuration for TLC workers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlcWorkerConfig {
    /// Number of worker threads.
    pub num_workers: u32,

    /// JVM heap size in megabytes.
    pub heap_size_mb: Option<u32>,

    /// Whether to use off-heap memory for states.
    pub use_off_heap: bool,

    /// Fingerprint function (1 = default, 0-127 available).
    pub fp_set: Option<u8>,

    /// Number of fingerprint workers.
    pub fp_workers: Option<u32>,
}

impl Default for TlcWorkerConfig {
    fn default() -> Self {
        TlcWorkerConfig {
            num_workers: 1,
            heap_size_mb: None,
            use_off_heap: false,
            fp_set: None,
            fp_workers: None,
        }
    }
}

impl TlcWorkerConfig {
    /// Create a new worker configuration.
    pub fn new(num_workers: u32) -> Self {
        TlcWorkerConfig {
            num_workers,
            ..Default::default()
        }
    }

    /// Set heap size.
    pub fn with_heap_size(mut self, size_mb: u32) -> Self {
        self.heap_size_mb = Some(size_mb);
        self
    }

    /// Enable off-heap storage.
    pub fn with_off_heap(mut self, enable: bool) -> Self {
        self.use_off_heap = enable;
        self
    }

    /// Set fingerprint configuration.
    pub fn with_fp_config(mut self, fp_set: u8, fp_workers: u32) -> Self {
        self.fp_set = Some(fp_set);
        self.fp_workers = Some(fp_workers);
        self
    }

    /// Generate JVM arguments for this configuration.
    pub fn to_jvm_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        if let Some(heap) = self.heap_size_mb {
            args.push(format!("-Xmx{}m", heap));
            args.push(format!("-Xms{}m", heap));
        }

        if self.use_off_heap {
            args.push("-XX:+UseG1GC".to_string());
            args.push("-XX:MaxGCPauseMillis=50".to_string());
        }

        args
    }
}

/// A constant value for TLC configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstantValue {
    /// An integer constant.
    Integer(i64),

    /// A boolean constant.
    Boolean(bool),

    /// A string constant.
    String(String),

    /// A model value (symbolic constant).
    ModelValue(String),

    /// A set of model values.
    ModelValueSet(Vec<String>),

    /// A TLA+ expression.
    Expression(String),

    /// A sequence.
    Sequence(Vec<ConstantValue>),

    /// A set.
    Set(Vec<ConstantValue>),
}

impl ConstantValue {
    /// Convert to TLA+ string representation.
    pub fn to_tla_string(&self) -> String {
        match self {
            ConstantValue::Integer(n) => n.to_string(),
            ConstantValue::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
            ConstantValue::String(s) => format!("\"{}\"", s.replace('\"', "\\\"")),
            ConstantValue::ModelValue(name) => name.clone(),
            ConstantValue::ModelValueSet(values) => {
                format!("{{ {} }}", values.join(", "))
            }
            ConstantValue::Expression(expr) => expr.clone(),
            ConstantValue::Sequence(items) => {
                let items_str: Vec<String> = items.iter().map(|v| v.to_tla_string()).collect();
                format!("<< {} >>", items_str.join(", "))
            }
            ConstantValue::Set(items) => {
                let items_str: Vec<String> = items.iter().map(|v| v.to_tla_string()).collect();
                format!("{{ {} }}", items_str.join(", "))
            }
        }
    }
}

impl fmt::Display for ConstantValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_tla_string())
    }
}

/// Generate a trace-checking TLA+ module that imports the trace and validates it.
pub fn generate_trace_checker_module(
    module_name: &str,
    spec_module: &str,
    trace_variable: &str,
) -> String {
    format!(
        r#"---- MODULE {module_name} ----
EXTENDS {spec_module}, TLC, Sequences, Naturals

\* Trace to validate
VARIABLE _trace_index

\* The trace data (imported)
_Trace == {trace_variable}

\* Initial state: start at beginning of trace
_TraceInit ==
    /\ _trace_index = 1
    /\ Init

\* Next state: advance through trace
_TraceNext ==
    /\ _trace_index < Len(_Trace)
    /\ _trace_index' = _trace_index + 1
    /\ Next
    /\ \* Match the trace state
       LET expected == _Trace[_trace_index'] IN
       \A var \in DOMAIN expected :
           TLCGet(var) = expected[var]

\* Trace validation spec
_TraceSpec == _TraceInit /\ [][_TraceNext]__<<_trace_index, vars>>

====
"#,
        module_name = module_name,
        spec_module = spec_module,
        trace_variable = trace_variable
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = TlcConfigBuilder::new("KVStore")
            .with_init("Init")
            .with_next("Next")
            .with_invariant("TypeInvariant")
            .with_invariant("SafetyInvariant")
            .with_property("Liveness")
            .with_constant_int("MaxKeys", 100)
            .with_model_value_set("Nodes", ["n1", "n2", "n3"])
            .with_workers(4)
            .check_deadlock(true)
            .build();

        assert_eq!(config.spec_name, "KVStore");
        assert_eq!(config.init, Some("Init".to_string()));
        assert_eq!(config.next, Some("Next".to_string()));
        assert_eq!(config.invariants.len(), 2);
        assert_eq!(config.properties.len(), 1);
        assert_eq!(config.workers.num_workers, 4);
    }

    #[test]
    fn test_config_to_cfg_string() {
        let config = TlcConfigBuilder::new("Test")
            .with_init("Init")
            .with_next("Next")
            .with_invariant("Inv")
            .with_constant_int("N", 10)
            .build();

        let cfg = config.to_cfg_string();
        assert!(cfg.contains("INIT Init"));
        assert!(cfg.contains("NEXT Next"));
        assert!(cfg.contains("INVARIANT Inv"));
        assert!(cfg.contains("N = 10"));
    }

    #[test]
    fn test_constant_values() {
        assert_eq!(ConstantValue::Integer(42).to_tla_string(), "42");
        assert_eq!(ConstantValue::Boolean(true).to_tla_string(), "TRUE");
        assert_eq!(ConstantValue::Boolean(false).to_tla_string(), "FALSE");
        assert_eq!(
            ConstantValue::String("hello".into()).to_tla_string(),
            "\"hello\""
        );
        assert_eq!(
            ConstantValue::ModelValueSet(vec!["a".into(), "b".into()]).to_tla_string(),
            "{ a, b }"
        );
        assert_eq!(
            ConstantValue::Sequence(vec![ConstantValue::Integer(1), ConstantValue::Integer(2)])
                .to_tla_string(),
            "<< 1, 2 >>"
        );
    }

    #[test]
    fn test_worker_config() {
        let config = TlcWorkerConfig::new(8)
            .with_heap_size(4096)
            .with_off_heap(true);

        let args = config.to_jvm_args();
        assert!(args.contains(&"-Xmx4096m".to_string()));
        assert!(args.contains(&"-Xms4096m".to_string()));
        assert!(args.contains(&"-XX:+UseG1GC".to_string()));
    }

    #[test]
    fn test_generate_trace_checker() {
        let module = generate_trace_checker_module("KVStoreTrace", "KVStore", "trace_data");
        assert!(module.contains("MODULE KVStoreTrace"));
        assert!(module.contains("EXTENDS KVStore"));
        assert!(module.contains("_trace_index"));
    }
}
