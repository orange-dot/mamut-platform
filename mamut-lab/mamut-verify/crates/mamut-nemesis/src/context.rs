//! Fault injection context and handle types.
//!
//! This module provides the context for fault injection operations and
//! handles for tracking active faults.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use uuid::Uuid;

use crate::capability::CapabilitySet;

/// Context provided to fault injection operations.
///
/// Contains all information needed for a fault to execute, including
/// target identification, timing parameters, and metadata.
#[derive(Debug, Clone)]
pub struct FaultContext {
    /// Unique identifier for this fault injection session.
    pub session_id: Uuid,

    /// Target specification for the fault.
    pub target: FaultTarget,

    /// Duration for which the fault should remain active.
    /// If None, the fault remains active until explicitly recovered.
    pub duration: Option<Duration>,

    /// Available capabilities for this context.
    pub available_capabilities: CapabilitySet,

    /// Additional parameters for fault configuration.
    pub parameters: HashMap<String, String>,

    /// Whether to run in dry-run mode (generate commands without executing).
    pub dry_run: bool,

    /// Namespace or container context for the fault.
    pub namespace: Option<String>,

    /// Network interface to target (for network faults).
    pub interface: Option<String>,
}

impl FaultContext {
    /// Creates a new fault context with the given session ID and target.
    pub fn new(session_id: Uuid, target: FaultTarget) -> Self {
        Self {
            session_id,
            target,
            duration: None,
            available_capabilities: CapabilitySet::new(),
            parameters: HashMap::new(),
            dry_run: false,
            namespace: None,
            interface: None,
        }
    }

    /// Creates a builder for constructing a fault context.
    pub fn builder() -> FaultContextBuilder {
        FaultContextBuilder::new()
    }

    /// Sets the duration for the fault.
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Sets dry-run mode.
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Adds a parameter to the context.
    pub fn with_parameter(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.parameters.insert(key.into(), value.into());
        self
    }

    /// Gets a parameter value.
    pub fn get_parameter(&self, key: &str) -> Option<&str> {
        self.parameters.get(key).map(|s| s.as_str())
    }

    /// Gets the network interface, defaulting to "eth0".
    pub fn get_interface(&self) -> &str {
        self.interface.as_deref().unwrap_or("eth0")
    }
}

/// Builder for constructing FaultContext instances.
#[derive(Debug, Default)]
pub struct FaultContextBuilder {
    session_id: Option<Uuid>,
    target: Option<FaultTarget>,
    duration: Option<Duration>,
    available_capabilities: CapabilitySet,
    parameters: HashMap<String, String>,
    dry_run: bool,
    namespace: Option<String>,
    interface: Option<String>,
}

impl FaultContextBuilder {
    /// Creates a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the session ID.
    pub fn session_id(mut self, id: Uuid) -> Self {
        self.session_id = Some(id);
        self
    }

    /// Sets the fault target.
    pub fn target(mut self, target: FaultTarget) -> Self {
        self.target = Some(target);
        self
    }

    /// Sets the duration.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Adds a capability.
    pub fn capability(mut self, cap: crate::capability::LinuxCapability) -> Self {
        self.available_capabilities.add(cap);
        self
    }

    /// Adds multiple capabilities.
    pub fn capabilities(mut self, caps: impl IntoIterator<Item = crate::capability::LinuxCapability>) -> Self {
        for cap in caps {
            self.available_capabilities.add(cap);
        }
        self
    }

    /// Adds a parameter.
    pub fn parameter(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.parameters.insert(key.into(), value.into());
        self
    }

    /// Sets dry-run mode.
    pub fn dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Sets the namespace.
    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Sets the network interface.
    pub fn interface(mut self, interface: impl Into<String>) -> Self {
        self.interface = Some(interface.into());
        self
    }

    /// Builds the FaultContext.
    ///
    /// # Panics
    /// Panics if target is not set.
    pub fn build(self) -> FaultContext {
        FaultContext {
            session_id: self.session_id.unwrap_or_else(Uuid::new_v4),
            target: self.target.expect("FaultTarget is required"),
            duration: self.duration,
            available_capabilities: self.available_capabilities,
            parameters: self.parameters,
            dry_run: self.dry_run,
            namespace: self.namespace,
            interface: self.interface,
        }
    }
}

/// Specifies the target of a fault injection.
#[derive(Debug, Clone)]
pub enum FaultTarget {
    /// Target a specific process by PID.
    Process(ProcessTarget),

    /// Target network communication.
    Network(NetworkTarget),

    /// Target the system clock.
    Clock(ClockTarget),

    /// Target a specific host.
    Host(HostTarget),

    /// Target a container.
    Container(ContainerTarget),
}

/// Target specification for process-level faults.
#[derive(Debug, Clone)]
pub struct ProcessTarget {
    /// Process ID to target.
    pub pid: Option<u32>,

    /// Process name pattern to match.
    pub name_pattern: Option<String>,

    /// Command line pattern to match.
    pub cmdline_pattern: Option<String>,
}

impl ProcessTarget {
    /// Creates a target for a specific PID.
    pub fn pid(pid: u32) -> Self {
        Self {
            pid: Some(pid),
            name_pattern: None,
            cmdline_pattern: None,
        }
    }

    /// Creates a target matching a process name pattern.
    pub fn name(pattern: impl Into<String>) -> Self {
        Self {
            pid: None,
            name_pattern: Some(pattern.into()),
            cmdline_pattern: None,
        }
    }
}

/// Target specification for network faults.
#[derive(Debug, Clone)]
pub struct NetworkTarget {
    /// Source IP address or CIDR.
    pub source: Option<IpAddr>,

    /// Destination IP address or CIDR.
    pub destination: Option<IpAddr>,

    /// Source port.
    pub source_port: Option<u16>,

    /// Destination port.
    pub destination_port: Option<u16>,

    /// Protocol (tcp, udp, icmp, all).
    pub protocol: Option<String>,
}

impl NetworkTarget {
    /// Creates a new empty network target.
    pub fn new() -> Self {
        Self {
            source: None,
            destination: None,
            source_port: None,
            destination_port: None,
            protocol: None,
        }
    }

    /// Creates a target for traffic to a specific destination.
    pub fn to_destination(addr: IpAddr) -> Self {
        Self {
            destination: Some(addr),
            ..Self::new()
        }
    }

    /// Creates a target for traffic from a specific source.
    pub fn from_source(addr: IpAddr) -> Self {
        Self {
            source: Some(addr),
            ..Self::new()
        }
    }

    /// Sets the protocol.
    pub fn with_protocol(mut self, protocol: impl Into<String>) -> Self {
        self.protocol = Some(protocol.into());
        self
    }

    /// Sets the destination port.
    pub fn with_destination_port(mut self, port: u16) -> Self {
        self.destination_port = Some(port);
        self
    }
}

impl Default for NetworkTarget {
    fn default() -> Self {
        Self::new()
    }
}

/// Target specification for clock faults.
#[derive(Debug, Clone)]
pub struct ClockTarget {
    /// Whether to affect the system-wide clock.
    pub system_wide: bool,

    /// Specific process to target for clock virtualization.
    pub process_id: Option<u32>,
}

impl ClockTarget {
    /// Creates a system-wide clock target.
    pub fn system() -> Self {
        Self {
            system_wide: true,
            process_id: None,
        }
    }

    /// Creates a process-specific clock target.
    pub fn process(pid: u32) -> Self {
        Self {
            system_wide: false,
            process_id: Some(pid),
        }
    }
}

/// Target specification for host-level faults.
#[derive(Debug, Clone)]
pub struct HostTarget {
    /// Hostname or IP address.
    pub address: String,

    /// SSH port for remote execution.
    pub ssh_port: Option<u16>,

    /// SSH user for authentication.
    pub ssh_user: Option<String>,
}

impl HostTarget {
    /// Creates a new host target.
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            ssh_port: None,
            ssh_user: None,
        }
    }
}

/// Target specification for container faults.
#[derive(Debug, Clone)]
pub struct ContainerTarget {
    /// Container ID.
    pub container_id: Option<String>,

    /// Container name.
    pub container_name: Option<String>,

    /// Container runtime (docker, containerd, cri-o).
    pub runtime: ContainerRuntime,
}

impl ContainerTarget {
    /// Creates a target for a container by ID.
    pub fn id(id: impl Into<String>) -> Self {
        Self {
            container_id: Some(id.into()),
            container_name: None,
            runtime: ContainerRuntime::Docker,
        }
    }

    /// Creates a target for a container by name.
    pub fn name(name: impl Into<String>) -> Self {
        Self {
            container_id: None,
            container_name: Some(name.into()),
            runtime: ContainerRuntime::Docker,
        }
    }

    /// Sets the container runtime.
    pub fn with_runtime(mut self, runtime: ContainerRuntime) -> Self {
        self.runtime = runtime;
        self
    }
}

/// Container runtime type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerRuntime {
    Docker,
    Containerd,
    CriO,
    Podman,
}

/// Handle to an active fault injection.
///
/// This handle is returned when a fault is successfully injected and
/// is required to recover (undo) the fault.
#[derive(Debug, Clone)]
pub struct FaultHandle {
    /// Unique identifier for this fault instance.
    pub id: Uuid,

    /// Name of the fault type.
    pub fault_name: String,

    /// When the fault was injected.
    pub injected_at: Instant,

    /// When the fault should automatically recover (if set).
    pub expires_at: Option<Instant>,

    /// Metadata needed for recovery.
    pub recovery_data: Arc<RecoveryData>,

    /// Commands executed during injection (for logging/debugging).
    pub injection_commands: Vec<String>,
}

impl FaultHandle {
    /// Creates a new fault handle.
    pub fn new(fault_name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            fault_name: fault_name.into(),
            injected_at: Instant::now(),
            expires_at: None,
            recovery_data: Arc::new(RecoveryData::default()),
            injection_commands: Vec::new(),
        }
    }

    /// Sets the expiration time.
    pub fn with_expiration(mut self, duration: Duration) -> Self {
        self.expires_at = Some(Instant::now() + duration);
        self
    }

    /// Sets the recovery data.
    pub fn with_recovery_data(mut self, data: RecoveryData) -> Self {
        self.recovery_data = Arc::new(data);
        self
    }

    /// Adds an injection command to the log.
    pub fn add_command(mut self, cmd: impl Into<String>) -> Self {
        self.injection_commands.push(cmd.into());
        self
    }

    /// Checks if the fault has expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at.map_or(false, |exp| Instant::now() >= exp)
    }

    /// Returns the elapsed time since injection.
    pub fn elapsed(&self) -> Duration {
        self.injected_at.elapsed()
    }
}

/// Data needed to recover from a fault.
#[derive(Debug, Clone, Default)]
pub struct RecoveryData {
    /// Commands to run for recovery.
    pub recovery_commands: Vec<String>,

    /// Files to restore.
    pub files_to_restore: Vec<FileBackup>,

    /// Process IDs that were affected.
    pub affected_pids: Vec<u32>,

    /// Network rules that were added.
    pub network_rules: Vec<String>,

    /// Original values that were modified.
    pub original_values: HashMap<String, String>,
}

impl RecoveryData {
    /// Creates new recovery data.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a recovery command.
    pub fn add_command(mut self, cmd: impl Into<String>) -> Self {
        self.recovery_commands.push(cmd.into());
        self
    }

    /// Adds an affected PID.
    pub fn add_pid(mut self, pid: u32) -> Self {
        self.affected_pids.push(pid);
        self
    }

    /// Stores an original value.
    pub fn store_original(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.original_values.insert(key.into(), value.into());
        self
    }
}

/// Backup information for a file.
#[derive(Debug, Clone)]
pub struct FileBackup {
    /// Original file path.
    pub path: String,

    /// Backup file path.
    pub backup_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_fault_context_builder() {
        let ctx = FaultContext::builder()
            .target(FaultTarget::Network(NetworkTarget::new()))
            .duration(Duration::from_secs(30))
            .dry_run(true)
            .parameter("key", "value")
            .build();

        assert_eq!(ctx.duration, Some(Duration::from_secs(30)));
        assert!(ctx.dry_run);
        assert_eq!(ctx.get_parameter("key"), Some("value"));
    }

    #[test]
    fn test_network_target() {
        let target = NetworkTarget::to_destination(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)))
            .with_protocol("tcp")
            .with_destination_port(8080);

        assert_eq!(target.destination, Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
        assert_eq!(target.protocol, Some("tcp".to_string()));
        assert_eq!(target.destination_port, Some(8080));
    }

    #[test]
    fn test_fault_handle_expiration() {
        let handle = FaultHandle::new("test")
            .with_expiration(Duration::from_millis(10));

        assert!(!handle.is_expired());
        std::thread::sleep(Duration::from_millis(20));
        assert!(handle.is_expired());
    }

    #[test]
    fn test_process_target() {
        let target = ProcessTarget::pid(1234);
        assert_eq!(target.pid, Some(1234));

        let target = ProcessTarget::name("nginx");
        assert_eq!(target.name_pattern, Some("nginx".to_string()));
    }
}
