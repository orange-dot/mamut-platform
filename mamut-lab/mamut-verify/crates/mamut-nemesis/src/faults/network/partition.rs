//! Network partition fault implementation.
//!
//! Uses iptables to block network traffic between nodes, simulating
//! network partitions in distributed systems.

use std::net::IpAddr;

use async_trait::async_trait;
use tracing::{debug, info};

use crate::capability::LinuxCapability;
use crate::context::{FaultContext, FaultHandle, FaultTarget, NetworkTarget, RecoveryData};
use crate::error::{NemesisError, Result};
use crate::traits::Fault;

use super::IptablesCommand;

/// Network partition fault that blocks traffic using iptables.
///
/// This fault creates iptables rules to DROP packets, effectively
/// partitioning the network between specified nodes.
///
/// # Example
///
/// ```ignore
/// use mamut_nemesis::faults::network::NetworkPartition;
///
/// let partition = NetworkPartition::new()
///     .bidirectional(true)
///     .reject_with_icmp(true);
/// ```
#[derive(Debug, Clone)]
pub struct NetworkPartition {
    /// Whether to block traffic in both directions.
    bidirectional: bool,
    /// Whether to use REJECT instead of DROP (sends ICMP unreachable).
    reject_with_icmp: bool,
    /// Custom iptables chain name for isolation.
    chain_name: String,
}

impl Default for NetworkPartition {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkPartition {
    /// Creates a new network partition fault.
    pub fn new() -> Self {
        Self {
            bidirectional: true,
            reject_with_icmp: false,
            chain_name: "NEMESIS_PARTITION".to_string(),
        }
    }

    /// Sets whether the partition is bidirectional.
    pub fn bidirectional(mut self, bidirectional: bool) -> Self {
        self.bidirectional = bidirectional;
        self
    }

    /// Sets whether to use REJECT with ICMP unreachable instead of DROP.
    ///
    /// Using REJECT causes faster failure detection but is more visible.
    /// DROP causes timeouts which better simulates actual network partitions.
    pub fn reject_with_icmp(mut self, reject: bool) -> Self {
        self.reject_with_icmp = reject;
        self
    }

    /// Sets a custom chain name.
    pub fn chain_name(mut self, name: impl Into<String>) -> Self {
        self.chain_name = name.into();
        self
    }

    /// Extracts the network target from the fault context.
    fn get_network_target<'a>(&self, ctx: &'a FaultContext) -> Result<&'a NetworkTarget> {
        match &ctx.target {
            FaultTarget::Network(target) => Ok(target),
            _ => Err(NemesisError::InvalidConfiguration(
                "NetworkPartition requires a Network target".to_string(),
            )),
        }
    }

    /// Generates iptables commands for creating the partition.
    fn generate_inject_commands(
        &self,
        target: &NetworkTarget,
        interface: &str,
    ) -> Result<(Vec<String>, Vec<String>)> {
        let mut inject_commands = Vec::new();
        let mut recovery_commands = Vec::new();

        // Determine if we're dealing with IPv6
        let is_ipv6 = target
            .destination
            .map(|ip| ip.is_ipv6())
            .or_else(|| target.source.map(|ip| ip.is_ipv6()))
            .unwrap_or(false);

        let binary = if is_ipv6 { "ip6tables" } else { "iptables" };

        // Create custom chain for easier cleanup
        inject_commands.push(format!("{} -N {}", binary, self.chain_name));
        recovery_commands.push(format!("{} -F {}", binary, self.chain_name));
        recovery_commands.push(format!("{} -X {}", binary, self.chain_name));

        // Add rules based on target specification
        if let Some(dest) = target.destination {
            let iptables = IptablesCommand::new(is_ipv6).chain(&self.chain_name);

            if self.reject_with_icmp {
                let reject_type = if is_ipv6 {
                    "icmp6-port-unreachable"
                } else {
                    "icmp-port-unreachable"
                };
                inject_commands.push(iptables.reject_to(dest, reject_type));
            } else {
                inject_commands.push(iptables.drop_to(
                    dest,
                    target.destination_port,
                    target.protocol.as_deref(),
                ));
            }

            // Add bidirectional rule if requested
            if self.bidirectional {
                let iptables = IptablesCommand::new(is_ipv6).chain(&self.chain_name);
                inject_commands.push(iptables.drop_from(
                    dest,
                    target.source_port,
                    target.protocol.as_deref(),
                ));
            }
        }

        if let Some(source) = target.source {
            let iptables = IptablesCommand::new(is_ipv6).chain(&self.chain_name);
            inject_commands.push(iptables.drop_from(
                source,
                target.source_port,
                target.protocol.as_deref(),
            ));

            if self.bidirectional {
                let iptables = IptablesCommand::new(is_ipv6).chain(&self.chain_name);
                inject_commands.push(iptables.drop_to(
                    source,
                    target.destination_port,
                    target.protocol.as_deref(),
                ));
            }
        }

        // Jump to custom chain from INPUT and OUTPUT
        inject_commands.push(format!("{} -I INPUT -j {}", binary, self.chain_name));
        inject_commands.push(format!("{} -I OUTPUT -j {}", binary, self.chain_name));

        // Remove jumps during recovery (before flush)
        recovery_commands.insert(0, format!("{} -D INPUT -j {}", binary, self.chain_name));
        recovery_commands.insert(1, format!("{} -D OUTPUT -j {}", binary, self.chain_name));

        // Validate we have at least one rule
        if inject_commands.len() <= 3 {
            // Only chain creation and jumps
            return Err(NemesisError::InvalidConfiguration(
                "No target specified for network partition (need source or destination)"
                    .to_string(),
            ));
        }

        debug!(
            inject_count = inject_commands.len(),
            recovery_count = recovery_commands.len(),
            interface = interface,
            "Generated partition commands"
        );

        Ok((inject_commands, recovery_commands))
    }

    /// Executes a command (or logs it in dry-run mode).
    async fn execute_command(&self, cmd: &str, dry_run: bool) -> Result<()> {
        if dry_run {
            info!(command = cmd, "Dry-run: would execute");
            return Ok(());
        }

        debug!(command = cmd, "Executing command");

        // In a real implementation, this would execute the command
        // For now, we just log it
        // tokio::process::Command::new("sh")
        //     .arg("-c")
        //     .arg(cmd)
        //     .status()
        //     .await?;

        Ok(())
    }
}

#[async_trait]
impl Fault for NetworkPartition {
    fn name(&self) -> &str {
        "network-partition"
    }

    fn required_capabilities(&self) -> Vec<LinuxCapability> {
        vec![LinuxCapability::NetAdmin]
    }

    async fn inject(&self, ctx: &FaultContext) -> Result<FaultHandle> {
        let target = self.get_network_target(ctx)?;
        let interface = ctx.get_interface();

        let (inject_commands, recovery_commands) =
            self.generate_inject_commands(target, interface)?;

        // Execute injection commands
        for cmd in &inject_commands {
            self.execute_command(cmd, ctx.dry_run).await?;
        }

        // Build the fault handle
        let mut handle = FaultHandle::new(self.name());

        if let Some(duration) = ctx.duration {
            handle = handle.with_expiration(duration);
        }

        for cmd in &inject_commands {
            handle = handle.add_command(cmd.clone());
        }

        let recovery_data = RecoveryData::new();
        let recovery_data = recovery_commands
            .into_iter()
            .fold(recovery_data, |data, cmd| data.add_command(cmd));

        handle = handle.with_recovery_data(recovery_data);

        info!(
            handle_id = %handle.id,
            destination = ?target.destination,
            source = ?target.source,
            bidirectional = self.bidirectional,
            "Network partition injected"
        );

        Ok(handle)
    }

    async fn recover(&self, handle: FaultHandle) -> Result<()> {
        info!(handle_id = %handle.id, "Recovering from network partition");

        for cmd in &handle.recovery_data.recovery_commands {
            // In recovery, we try to execute all commands even if some fail
            if let Err(e) = self.execute_command(cmd, false).await {
                debug!(command = cmd, error = %e, "Recovery command failed");
            }
        }

        info!(handle_id = %handle.id, "Network partition recovered");
        Ok(())
    }

    async fn is_active(&self, handle: &FaultHandle) -> Result<bool> {
        // Check if the iptables chain still exists
        // In a real implementation:
        // let output = Command::new("iptables").arg("-L").arg(&self.chain_name).output()?;
        // Ok(output.status.success())

        Ok(!handle.is_expired())
    }

    fn description(&self) -> &str {
        "Blocks network traffic between nodes using iptables rules"
    }
}

/// Builder for creating network partition configurations.
#[derive(Debug, Default)]
pub struct NetworkPartitionBuilder {
    partition: NetworkPartition,
    destinations: Vec<IpAddr>,
    sources: Vec<IpAddr>,
    ports: Vec<u16>,
    protocol: Option<String>,
}

impl NetworkPartitionBuilder {
    /// Creates a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a destination to partition.
    pub fn destination(mut self, addr: IpAddr) -> Self {
        self.destinations.push(addr);
        self
    }

    /// Adds a source to partition.
    pub fn source(mut self, addr: IpAddr) -> Self {
        self.sources.push(addr);
        self
    }

    /// Adds a port to filter.
    pub fn port(mut self, port: u16) -> Self {
        self.ports.push(port);
        self
    }

    /// Sets the protocol.
    pub fn protocol(mut self, protocol: impl Into<String>) -> Self {
        self.protocol = Some(protocol.into());
        self
    }

    /// Sets bidirectional mode.
    pub fn bidirectional(mut self, bidirectional: bool) -> Self {
        self.partition = self.partition.bidirectional(bidirectional);
        self
    }

    /// Sets ICMP reject mode.
    pub fn reject_with_icmp(mut self, reject: bool) -> Self {
        self.partition = self.partition.reject_with_icmp(reject);
        self
    }

    /// Builds the network partition.
    pub fn build(self) -> NetworkPartition {
        self.partition
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_partition_default() {
        let partition = NetworkPartition::new();
        assert!(partition.bidirectional);
        assert!(!partition.reject_with_icmp);
    }

    #[test]
    fn test_partition_builder() {
        let partition = NetworkPartitionBuilder::new()
            .bidirectional(false)
            .reject_with_icmp(true)
            .build();

        assert!(!partition.bidirectional);
        assert!(partition.reject_with_icmp);
    }

    #[test]
    fn test_generate_commands() {
        let partition = NetworkPartition::new();
        let target = NetworkTarget::to_destination(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)))
            .with_protocol("tcp")
            .with_destination_port(8080);

        let (inject, recover) = partition
            .generate_inject_commands(&target, "eth0")
            .unwrap();

        assert!(!inject.is_empty());
        assert!(!recover.is_empty());

        // Check that recovery commands clean up properly
        assert!(recover.iter().any(|c| c.contains("-F")));
        assert!(recover.iter().any(|c| c.contains("-X")));
    }

    #[test]
    fn test_required_capabilities() {
        let partition = NetworkPartition::new();
        let caps = partition.required_capabilities();
        assert!(caps.contains(&LinuxCapability::NetAdmin));
    }
}
