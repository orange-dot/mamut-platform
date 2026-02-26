//! Core traits for fault injection.
//!
//! This module defines the primary trait that all fault implementations must
//! satisfy, along with supporting traits for fault composition and lifecycle
//! management.

use async_trait::async_trait;

use crate::capability::LinuxCapability;
use crate::context::{FaultContext, FaultHandle};
use crate::error::Result;

/// Core trait for fault injection implementations.
///
/// A `Fault` represents a specific type of failure or degradation that can be
/// injected into a system. Implementations must be thread-safe (`Send + Sync`)
/// to support concurrent fault injection scenarios.
///
/// # Lifecycle
///
/// 1. Check `required_capabilities()` to verify permissions
/// 2. Call `inject()` to activate the fault
/// 3. Optionally call `is_active()` to verify the fault is running
/// 4. Call `recover()` to clean up and restore normal operation
///
/// # Example
///
/// ```ignore
/// use mamut_nemesis::{Fault, FaultContext, FaultHandle, LinuxCapability, Result};
///
/// struct MyFault;
///
/// #[async_trait]
/// impl Fault for MyFault {
///     fn name(&self) -> &str {
///         "my-fault"
///     }
///
///     fn required_capabilities(&self) -> Vec<LinuxCapability> {
///         vec![LinuxCapability::NetAdmin]
///     }
///
///     async fn inject(&self, ctx: &FaultContext) -> Result<FaultHandle> {
///         // Inject the fault
///         Ok(FaultHandle::new("my-fault"))
///     }
///
///     async fn recover(&self, handle: FaultHandle) -> Result<()> {
///         // Clean up
///         Ok(())
///     }
///
///     async fn is_active(&self, handle: &FaultHandle) -> Result<bool> {
///         Ok(true)
///     }
/// }
/// ```
#[async_trait]
pub trait Fault: Send + Sync {
    /// Returns the unique name of this fault type.
    ///
    /// This name is used for logging, metrics, and identifying fault types
    /// in configuration and recovery data.
    fn name(&self) -> &str;

    /// Returns the Linux capabilities required to inject this fault.
    ///
    /// The fault scheduler uses this to verify that the execution environment
    /// has the necessary permissions before attempting injection.
    fn required_capabilities(&self) -> Vec<LinuxCapability>;

    /// Injects the fault into the target system.
    ///
    /// This method should:
    /// - Validate the context and target
    /// - Execute the necessary commands or modifications
    /// - Return a handle containing recovery information
    ///
    /// If `ctx.dry_run` is true, the implementation should generate and log
    /// commands without actually executing them.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The fault context containing target and configuration
    ///
    /// # Returns
    ///
    /// A `FaultHandle` that can be used to recover from the fault, or an
    /// error if injection failed.
    async fn inject(&self, ctx: &FaultContext) -> Result<FaultHandle>;

    /// Recovers from an injected fault.
    ///
    /// This method should completely undo all effects of the fault injection,
    /// restoring the system to its pre-fault state.
    ///
    /// # Arguments
    ///
    /// * `handle` - The handle returned from `inject()`
    ///
    /// # Returns
    ///
    /// `Ok(())` if recovery was successful, or an error if recovery failed.
    /// Note that partial recovery may leave the system in an inconsistent state.
    async fn recover(&self, handle: FaultHandle) -> Result<()>;

    /// Checks if a fault is currently active.
    ///
    /// This can be used to verify that a fault was successfully injected
    /// and is still in effect.
    ///
    /// # Arguments
    ///
    /// * `handle` - The handle to check
    ///
    /// # Returns
    ///
    /// `true` if the fault is still active, `false` if it has been recovered
    /// or expired, or an error if the check failed.
    async fn is_active(&self, handle: &FaultHandle) -> Result<bool>;

    /// Returns a human-readable description of this fault.
    ///
    /// Default implementation returns the fault name.
    fn description(&self) -> &str {
        self.name()
    }

    /// Validates that the fault can be injected with the given context.
    ///
    /// Default implementation checks capability requirements.
    fn validate(&self, ctx: &FaultContext) -> Result<()> {
        let required = self.required_capabilities();
        for cap in required {
            if !ctx.available_capabilities.contains(cap) {
                return Err(crate::error::NemesisError::MissingCapability(cap));
            }
        }
        Ok(())
    }
}

/// Trait for faults that can be composed with other faults.
pub trait ComposableFault: Fault {
    /// Returns whether this fault can be combined with another fault type.
    fn is_compatible_with(&self, other: &dyn Fault) -> bool;

    /// Returns the priority of this fault when combined with others.
    /// Higher priority faults are injected first.
    fn priority(&self) -> i32 {
        0
    }
}

/// Trait for faults that can provide status information.
#[async_trait]
pub trait ObservableFault: Fault {
    /// Returns detailed status information about an active fault.
    async fn status(&self, handle: &FaultHandle) -> Result<FaultStatus>;

    /// Returns metrics about the fault's impact.
    async fn metrics(&self, handle: &FaultHandle) -> Result<FaultMetrics>;
}

/// Status information for an active fault.
#[derive(Debug, Clone)]
pub struct FaultStatus {
    /// Whether the fault is currently active.
    pub is_active: bool,

    /// Human-readable status message.
    pub message: String,

    /// Additional status details.
    pub details: std::collections::HashMap<String, String>,
}

/// Metrics about a fault's impact.
#[derive(Debug, Clone, Default)]
pub struct FaultMetrics {
    /// Number of times the fault has been triggered.
    pub trigger_count: u64,

    /// Number of affected operations.
    pub affected_operations: u64,

    /// Total duration the fault has been active.
    pub active_duration_ms: u64,

    /// Custom metrics.
    pub custom: std::collections::HashMap<String, f64>,
}

/// A boxed fault for dynamic dispatch.
pub type BoxedFault = Box<dyn Fault>;

/// A collection of fault implementations.
#[derive(Default)]
pub struct FaultRegistry {
    faults: std::collections::HashMap<String, BoxedFault>,
}

impl FaultRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a fault implementation.
    pub fn register<F: Fault + 'static>(&mut self, fault: F) {
        self.faults.insert(fault.name().to_string(), Box::new(fault));
    }

    /// Gets a fault by name.
    pub fn get(&self, name: &str) -> Option<&dyn Fault> {
        self.faults.get(name).map(|f| f.as_ref())
    }

    /// Returns all registered fault names.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.faults.keys().map(|s| s.as_str())
    }

    /// Returns the number of registered faults.
    pub fn len(&self) -> usize {
        self.faults.len()
    }

    /// Checks if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.faults.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::FaultTarget;
    use crate::context::NetworkTarget;

    struct TestFault;

    #[async_trait]
    impl Fault for TestFault {
        fn name(&self) -> &str {
            "test-fault"
        }

        fn required_capabilities(&self) -> Vec<LinuxCapability> {
            vec![LinuxCapability::NetAdmin]
        }

        async fn inject(&self, _ctx: &FaultContext) -> Result<FaultHandle> {
            Ok(FaultHandle::new("test-fault"))
        }

        async fn recover(&self, _handle: FaultHandle) -> Result<()> {
            Ok(())
        }

        async fn is_active(&self, _handle: &FaultHandle) -> Result<bool> {
            Ok(true)
        }
    }

    #[test]
    fn test_fault_registry() {
        let mut registry = FaultRegistry::new();
        registry.register(TestFault);

        assert_eq!(registry.len(), 1);
        assert!(registry.get("test-fault").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[tokio::test]
    async fn test_fault_validation() {
        let fault = TestFault;

        // Without required capability
        let ctx = FaultContext::builder()
            .target(FaultTarget::Network(NetworkTarget::new()))
            .build();

        assert!(fault.validate(&ctx).is_err());

        // With required capability
        let ctx = FaultContext::builder()
            .target(FaultTarget::Network(NetworkTarget::new()))
            .capability(LinuxCapability::NetAdmin)
            .build();

        assert!(fault.validate(&ctx).is_ok());
    }
}
