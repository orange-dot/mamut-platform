//! # Mamut Nemesis - Fault Injection Framework
//!
//! Nemesis is a fault injection framework for testing distributed systems.
//! It provides a composable, capability-aware approach to chaos engineering.
//!
//! ## Overview
//!
//! The framework is organized around several core concepts:
//!
//! - **Faults**: Individual failure scenarios that can be injected
//! - **Capabilities**: Linux capabilities required for fault injection
//! - **Context**: Configuration and targeting for fault injection
//! - **Scheduler**: Coordinates when and which faults to inject
//!
//! ## Quick Start
//!
//! ```ignore
//! use mamut_nemesis::{
//!     NemesisScheduler, SchedulingStrategy,
//!     FaultContext, FaultTarget, NetworkTarget,
//!     faults::network::NetworkLatency,
//!     LinuxCapability,
//! };
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a scheduler with random fault selection
//!     let mut scheduler = NemesisScheduler::random();
//!
//!     // Register faults
//!     scheduler.register_fault(Arc::new(
//!         NetworkLatency::new()
//!             .delay_ms(100)
//!             .jitter_ms(20)
//!     ));
//!
//!     // Create injection context
//!     let ctx = FaultContext::builder()
//!         .target(FaultTarget::Network(NetworkTarget::new()))
//!         .capability(LinuxCapability::NetAdmin)
//!         .duration(Duration::from_secs(30))
//!         .build();
//!
//!     // Inject a fault
//!     let handle = scheduler.inject(&ctx).await?;
//!
//!     // ... test your system ...
//!
//!     // Recover
//!     scheduler.recover(handle).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Available Faults
//!
//! ### Network Faults
//!
//! - [`NetworkPartition`](faults::network::NetworkPartition): Blocks traffic using iptables
//! - [`NetworkLatency`](faults::network::NetworkLatency): Adds delay using tc/netem
//!
//! ### Clock Faults
//!
//! - [`ClockDrift`](faults::clock::ClockDrift): Gradually skews system time
//! - [`ClockJump`](faults::clock::ClockJump): Instantly jumps system time
//!
//! ### Process Faults
//!
//! - [`ProcessKill`](faults::process::ProcessKill): Terminates processes with signals
//!
//! ## Scheduling Strategies
//!
//! - `Random`: Select faults randomly
//! - `RoundRobin`: Cycle through faults in order
//! - `Weighted`: Select based on configured weights
//! - `Composed`: Combine multiple strategies
//!
//! ## Linux Capabilities
//!
//! Nemesis uses Linux capabilities for fine-grained permission control:
//!
//! - `CAP_NET_ADMIN`: Network faults (iptables, tc)
//! - `CAP_SYS_TIME`: Clock faults
//! - `CAP_KILL`: Process faults
//!
//! ## Safety
//!
//! Fault injection can cause data loss and system instability. Always:
//!
//! - Use in isolated test environments
//! - Implement proper recovery procedures
//! - Monitor injected faults
//! - Set appropriate timeouts

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]

pub mod capability;
pub mod context;
pub mod error;
pub mod faults;
pub mod scheduler;
pub mod traits;

// Re-export main types at crate root for convenience
pub use capability::{CapabilitySet, LinuxCapability};
pub use context::{
    ClockTarget, ContainerRuntime, ContainerTarget, FaultContext, FaultContextBuilder,
    FaultHandle, FaultTarget, HostTarget, NetworkTarget, ProcessTarget, RecoveryData,
};
pub use error::{NemesisError, Result};
pub use scheduler::{
    CompositionMode, NemesisScheduler, SchedulerBuilder, SchedulerConfig, SchedulingStrategy,
};
pub use traits::{BoxedFault, Fault, FaultMetrics, FaultRegistry, FaultStatus};

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::capability::LinuxCapability;
    pub use crate::context::{FaultContext, FaultHandle, FaultTarget, NetworkTarget};
    pub use crate::error::Result;
    pub use crate::faults::{
        ClockDrift, ClockJump, NetworkLatency, NetworkPartition, ProcessKill,
    };
    pub use crate::scheduler::{NemesisScheduler, SchedulingStrategy};
    pub use crate::traits::Fault;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn test_basic_workflow() {
        use crate::faults::network::NetworkLatency;

        // Create scheduler
        let mut scheduler = NemesisScheduler::random_with_seed(42);

        // Register fault
        scheduler.register_fault(Arc::new(NetworkLatency::new().delay_ms(50)));

        // Create context
        let ctx = FaultContext::builder()
            .target(FaultTarget::Network(NetworkTarget::new()))
            .capability(LinuxCapability::NetAdmin)
            .duration(Duration::from_secs(10))
            .dry_run(true) // Don't actually execute commands in tests
            .build();

        // Inject
        let handle = scheduler.inject(&ctx).await.unwrap();
        assert_eq!(handle.fault_name, "network-latency");
        assert_eq!(scheduler.active_count(), 1);

        // Recover
        scheduler.recover(handle).await.unwrap();
        assert_eq!(scheduler.active_count(), 0);
    }

    #[tokio::test]
    async fn test_multiple_faults() {
        use crate::faults::network::{NetworkLatency, NetworkPartition};

        let mut scheduler = NemesisScheduler::round_robin();

        scheduler.register_fault(Arc::new(NetworkLatency::new()));
        scheduler.register_fault(Arc::new(NetworkPartition::new()));

        assert_eq!(scheduler.fault_count(), 2);

        // Round robin should select in order
        let first = scheduler.select_next().unwrap();
        assert_eq!(first.name(), "network-latency");

        let second = scheduler.select_next().unwrap();
        assert_eq!(second.name(), "network-partition");
    }

    #[test]
    fn test_capability_requirements() {
        use crate::faults::clock::ClockDrift;
        use crate::faults::network::NetworkLatency;
        use crate::faults::process::ProcessKill;

        let latency = NetworkLatency::new();
        assert!(latency
            .required_capabilities()
            .contains(&LinuxCapability::NetAdmin));

        let drift = ClockDrift::new();
        assert!(drift
            .required_capabilities()
            .contains(&LinuxCapability::SysTime));

        let kill = ProcessKill::default();
        assert!(kill.required_capabilities().contains(&LinuxCapability::Kill));
    }
}
