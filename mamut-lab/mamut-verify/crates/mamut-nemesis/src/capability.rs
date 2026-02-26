//! Linux capability definitions for fault injection.
//!
//! This module defines the Linux capabilities required by various fault
//! injection operations. Capabilities provide fine-grained control over
//! privileged operations without requiring full root access.

use std::fmt;

/// Linux capabilities required for fault injection operations.
///
/// These map to the capabilities defined in `linux/capability.h`.
/// Fault injectors declare which capabilities they need, allowing
/// the scheduler to verify permissions before injection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LinuxCapability {
    /// Allows various network-related operations:
    /// - Interface configuration
    /// - Firewall administration (iptables)
    /// - Traffic control (tc)
    /// - Setting socket options
    NetAdmin,

    /// Allows manipulation of system time:
    /// - Setting system clock
    /// - Adjusting time with adjtime
    SysTime,

    /// Allows tracing and debugging of processes:
    /// - ptrace operations
    /// - Process inspection
    SysPtrace,

    /// Allows sending signals to any process:
    /// - kill() to any process
    /// - Signal injection
    Kill,

    /// Allows raw network access:
    /// - Creating raw sockets
    /// - Packet injection
    NetRaw,

    /// Allows binding to privileged ports (< 1024):
    /// - TCP/UDP port binding
    NetBindService,

    /// Allows various system administration operations:
    /// - Mounting filesystems
    /// - Configuring kernel parameters
    SysAdmin,

    /// Allows I/O port operations:
    /// - Direct I/O port access
    /// - Useful for hardware fault simulation
    SysRawio,

    /// Allows setting resource limits:
    /// - rlimit adjustments
    /// - CPU/memory quota modifications
    SysResource,

    /// Allows module loading/unloading:
    /// - Kernel module operations
    /// - Driver fault injection
    SysModule,

    /// Allows file system operations:
    /// - chroot
    /// - File permission bypasses
    SysChroot,

    /// Allows setting file capabilities:
    /// - Capability manipulation on files
    Setfcap,

    /// Allows DAC (Discretionary Access Control) bypass for reading:
    /// - Read any file regardless of permissions
    DacReadSearch,

    /// Allows DAC bypass for writing:
    /// - Write any file regardless of permissions
    DacOverride,
}

impl LinuxCapability {
    /// Returns the capability name as it appears in the Linux kernel.
    pub fn kernel_name(&self) -> &'static str {
        match self {
            LinuxCapability::NetAdmin => "CAP_NET_ADMIN",
            LinuxCapability::SysTime => "CAP_SYS_TIME",
            LinuxCapability::SysPtrace => "CAP_SYS_PTRACE",
            LinuxCapability::Kill => "CAP_KILL",
            LinuxCapability::NetRaw => "CAP_NET_RAW",
            LinuxCapability::NetBindService => "CAP_NET_BIND_SERVICE",
            LinuxCapability::SysAdmin => "CAP_SYS_ADMIN",
            LinuxCapability::SysRawio => "CAP_SYS_RAWIO",
            LinuxCapability::SysResource => "CAP_SYS_RESOURCE",
            LinuxCapability::SysModule => "CAP_SYS_MODULE",
            LinuxCapability::SysChroot => "CAP_SYS_CHROOT",
            LinuxCapability::Setfcap => "CAP_SETFCAP",
            LinuxCapability::DacReadSearch => "CAP_DAC_READ_SEARCH",
            LinuxCapability::DacOverride => "CAP_DAC_OVERRIDE",
        }
    }

    /// Returns the capability number as defined in the kernel.
    pub fn cap_number(&self) -> u32 {
        match self {
            LinuxCapability::NetAdmin => 12,
            LinuxCapability::SysTime => 25,
            LinuxCapability::SysPtrace => 19,
            LinuxCapability::Kill => 5,
            LinuxCapability::NetRaw => 13,
            LinuxCapability::NetBindService => 10,
            LinuxCapability::SysAdmin => 21,
            LinuxCapability::SysRawio => 17,
            LinuxCapability::SysResource => 24,
            LinuxCapability::SysModule => 16,
            LinuxCapability::SysChroot => 18,
            LinuxCapability::Setfcap => 31,
            LinuxCapability::DacReadSearch => 2,
            LinuxCapability::DacOverride => 1,
        }
    }

    /// Returns a human-readable description of the capability.
    pub fn description(&self) -> &'static str {
        match self {
            LinuxCapability::NetAdmin => "Network administration (iptables, tc, routing)",
            LinuxCapability::SysTime => "System time manipulation",
            LinuxCapability::SysPtrace => "Process tracing and debugging",
            LinuxCapability::Kill => "Send signals to any process",
            LinuxCapability::NetRaw => "Raw network socket access",
            LinuxCapability::NetBindService => "Bind to privileged ports",
            LinuxCapability::SysAdmin => "System administration operations",
            LinuxCapability::SysRawio => "Raw I/O port access",
            LinuxCapability::SysResource => "Resource limit configuration",
            LinuxCapability::SysModule => "Kernel module operations",
            LinuxCapability::SysChroot => "Filesystem root operations",
            LinuxCapability::Setfcap => "File capability manipulation",
            LinuxCapability::DacReadSearch => "Bypass file read permissions",
            LinuxCapability::DacOverride => "Bypass file write permissions",
        }
    }
}

impl fmt::Display for LinuxCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kernel_name())
    }
}

/// A set of Linux capabilities.
#[derive(Debug, Clone, Default)]
pub struct CapabilitySet {
    capabilities: Vec<LinuxCapability>,
}

impl CapabilitySet {
    /// Creates an empty capability set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a capability to the set.
    pub fn add(&mut self, cap: LinuxCapability) {
        if !self.capabilities.contains(&cap) {
            self.capabilities.push(cap);
        }
    }

    /// Checks if the set contains a capability.
    pub fn contains(&self, cap: LinuxCapability) -> bool {
        self.capabilities.contains(&cap)
    }

    /// Checks if this set contains all capabilities from another set.
    pub fn contains_all(&self, other: &CapabilitySet) -> bool {
        other.capabilities.iter().all(|cap| self.contains(*cap))
    }

    /// Returns the capabilities as a slice.
    pub fn as_slice(&self) -> &[LinuxCapability] {
        &self.capabilities
    }

    /// Returns the number of capabilities in the set.
    pub fn len(&self) -> usize {
        self.capabilities.len()
    }

    /// Checks if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }

    /// Returns an iterator over the capabilities.
    pub fn iter(&self) -> impl Iterator<Item = &LinuxCapability> {
        self.capabilities.iter()
    }
}

impl FromIterator<LinuxCapability> for CapabilitySet {
    fn from_iter<T: IntoIterator<Item = LinuxCapability>>(iter: T) -> Self {
        let mut set = CapabilitySet::new();
        for cap in iter {
            set.add(cap);
        }
        set
    }
}

impl<'a> IntoIterator for &'a CapabilitySet {
    type Item = &'a LinuxCapability;
    type IntoIter = std::slice::Iter<'a, LinuxCapability>;

    fn into_iter(self) -> Self::IntoIter {
        self.capabilities.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_names() {
        assert_eq!(LinuxCapability::NetAdmin.kernel_name(), "CAP_NET_ADMIN");
        assert_eq!(LinuxCapability::SysTime.kernel_name(), "CAP_SYS_TIME");
    }

    #[test]
    fn test_capability_set() {
        let mut set = CapabilitySet::new();
        set.add(LinuxCapability::NetAdmin);
        set.add(LinuxCapability::SysTime);
        set.add(LinuxCapability::NetAdmin); // Duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(LinuxCapability::NetAdmin));
        assert!(set.contains(LinuxCapability::SysTime));
        assert!(!set.contains(LinuxCapability::Kill));
    }

    #[test]
    fn test_capability_set_contains_all() {
        let mut available = CapabilitySet::new();
        available.add(LinuxCapability::NetAdmin);
        available.add(LinuxCapability::SysTime);
        available.add(LinuxCapability::Kill);

        let mut required = CapabilitySet::new();
        required.add(LinuxCapability::NetAdmin);
        required.add(LinuxCapability::SysTime);

        assert!(available.contains_all(&required));

        required.add(LinuxCapability::SysPtrace);
        assert!(!available.contains_all(&required));
    }
}
