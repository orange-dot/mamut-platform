//! Fault implementations for various failure scenarios.
//!
//! This module contains concrete fault implementations organized by category:
//!
//! - **network**: Network-level faults (partition, latency, packet loss)
//! - **clock**: Time-related faults (drift, jump)
//! - **process**: Process-level faults (kill, pause, resource exhaustion)

pub mod clock;
pub mod network;
pub mod process;

// Re-export commonly used faults
pub use clock::{ClockDrift, ClockJump};
pub use network::{NetworkLatency, NetworkPartition};
pub use process::ProcessKill;
