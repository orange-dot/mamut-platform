//! TLA+ integration module for trace validation.
//!
//! This module provides integration with the TLC model checker for validating
//! execution traces against TLA+ specifications. It supports:
//!
//! - Running TLC as a subprocess to validate traces
//! - Parsing TLC output and extracting counterexamples
//! - Binding runtime events to TLA+ state variables
//! - Generating TLC configuration files
//!
//! # Architecture
//!
//! The module is organized into several components:
//!
//! - [`TlcRunner`] - Manages TLC subprocess execution
//! - [`TraceStep`] - Represents a single step in an execution trace
//! - [`SpecBinding`] - Maps runtime events to TLA+ state
//! - [`ValidationResult`] - Contains the result of trace validation
//! - [`TlcConfig`] - Generates TLC configuration files
//!
//! # Example
//!
//! ```rust,ignore
//! use mamut_checker::tlaplus::{TlcRunner, TraceFile, SpecBinding};
//! use std::path::Path;
//! use std::time::Duration;
//!
//! let runner = TlcRunner::new(Path::new("/path/to/tla2tools.jar"))
//!     .with_timeout(Duration::from_secs(60));
//!
//! let result = runner.validate_trace(
//!     Path::new("spec.tla"),
//!     Path::new("trace.ndjson"),
//! ).await?;
//!
//! if result.is_valid() {
//!     println!("Trace is valid!");
//! } else {
//!     for violation in result.violations() {
//!         println!("Violation: {}", violation);
//!     }
//! }
//! ```

mod binding;
mod config;
mod result;
mod runner;
mod trace;

pub use binding::{
    OperationMapping, SpecBinding, SpecId, VariableMapping, VariableSource, VariableTransform,
};
pub use config::{TlcConfig, TlcConfigBuilder, TlcWorkerConfig};
pub use result::{
    CounterexampleState, CounterexampleTrace, ParsedTlcOutput, PropertyViolation, TlcError,
    TlcExitStatus, ValidationResult, ValidationStatus,
};
pub use runner::{TlcRunner, TlcRunnerBuilder};
pub use trace::{TraceClock, TraceFile, TraceStep, VectorClock};
