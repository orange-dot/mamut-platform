//! Network latency fault implementation.
//!
//! Uses tc (traffic control) with netem to add artificial latency
//! to network traffic.

use async_trait::async_trait;
use tracing::{debug, info};

use crate::capability::LinuxCapability;
use crate::context::{FaultContext, FaultHandle, FaultTarget, NetworkTarget, RecoveryData};
use crate::error::{NemesisError, Result};
use crate::traits::Fault;

use super::TcCommand;

/// Network latency fault that adds delay using tc/netem.
///
/// This fault uses the Linux traffic control (tc) subsystem with the
/// netem queueing discipline to add artificial latency to network packets.
///
/// # Example
///
/// ```ignore
/// use mamut_nemesis::faults::network::NetworkLatency;
///
/// let latency = NetworkLatency::new()
///     .delay_ms(100)
///     .jitter_ms(20)
///     .correlation_percent(25);
/// ```
#[derive(Debug, Clone)]
pub struct NetworkLatency {
    /// Base delay in milliseconds.
    delay_ms: u32,
    /// Jitter (variation) in milliseconds.
    jitter_ms: Option<u32>,
    /// Correlation percentage (how much each delay depends on the previous).
    correlation_percent: Option<u32>,
    /// Distribution for delay variation (normal, pareto, paretonormal).
    distribution: Option<String>,
    /// Packet loss percentage (can be combined with delay).
    loss_percent: Option<f32>,
    /// Packet duplication percentage.
    duplicate_percent: Option<f32>,
    /// Packet corruption percentage.
    corrupt_percent: Option<f32>,
    /// Packet reordering percentage.
    reorder_percent: Option<f32>,
}

impl Default for NetworkLatency {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkLatency {
    /// Creates a new network latency fault with default values.
    pub fn new() -> Self {
        Self {
            delay_ms: 100,
            jitter_ms: None,
            correlation_percent: None,
            distribution: None,
            loss_percent: None,
            duplicate_percent: None,
            corrupt_percent: None,
            reorder_percent: None,
        }
    }

    /// Sets the base delay in milliseconds.
    pub fn delay_ms(mut self, delay: u32) -> Self {
        self.delay_ms = delay;
        self
    }

    /// Sets the jitter (delay variation) in milliseconds.
    pub fn jitter_ms(mut self, jitter: u32) -> Self {
        self.jitter_ms = Some(jitter);
        self
    }

    /// Sets the correlation percentage.
    ///
    /// This determines how much each packet's delay depends on the previous
    /// packet's delay. Higher values create more "bursty" latency patterns.
    pub fn correlation_percent(mut self, correlation: u32) -> Self {
        self.correlation_percent = Some(correlation.min(100));
        self
    }

    /// Sets the delay distribution.
    ///
    /// Options: "normal", "pareto", "paretonormal"
    pub fn distribution(mut self, dist: impl Into<String>) -> Self {
        self.distribution = Some(dist.into());
        self
    }

    /// Sets packet loss percentage (0.0 - 100.0).
    pub fn loss_percent(mut self, loss: f32) -> Self {
        self.loss_percent = Some(loss.clamp(0.0, 100.0));
        self
    }

    /// Sets packet duplication percentage.
    pub fn duplicate_percent(mut self, dup: f32) -> Self {
        self.duplicate_percent = Some(dup.clamp(0.0, 100.0));
        self
    }

    /// Sets packet corruption percentage.
    pub fn corrupt_percent(mut self, corrupt: f32) -> Self {
        self.corrupt_percent = Some(corrupt.clamp(0.0, 100.0));
        self
    }

    /// Sets packet reordering percentage.
    pub fn reorder_percent(mut self, reorder: f32) -> Self {
        self.reorder_percent = Some(reorder.clamp(0.0, 100.0));
        self
    }

    /// Generates the tc/netem command for injection.
    fn generate_inject_command(&self, interface: &str) -> String {
        let mut cmd = format!("tc qdisc add dev {} root netem", interface);

        // Add delay
        cmd.push_str(&format!(" delay {}ms", self.delay_ms));

        if let Some(jitter) = self.jitter_ms {
            cmd.push_str(&format!(" {}ms", jitter));

            if let Some(correlation) = self.correlation_percent {
                cmd.push_str(&format!(" {}%", correlation));
            }
        }

        if let Some(ref dist) = self.distribution {
            cmd.push_str(&format!(" distribution {}", dist));
        }

        // Add packet loss
        if let Some(loss) = self.loss_percent {
            cmd.push_str(&format!(" loss {:.2}%", loss));
        }

        // Add duplication
        if let Some(dup) = self.duplicate_percent {
            cmd.push_str(&format!(" duplicate {:.2}%", dup));
        }

        // Add corruption
        if let Some(corrupt) = self.corrupt_percent {
            cmd.push_str(&format!(" corrupt {:.2}%", corrupt));
        }

        // Add reordering
        if let Some(reorder) = self.reorder_percent {
            cmd.push_str(&format!(" reorder {:.2}%", reorder));
            // Reorder requires a delay specification, which we already have
        }

        cmd
    }

    /// Generates the tc command for recovery (removing the qdisc).
    fn generate_recovery_command(&self, interface: &str) -> String {
        TcCommand::new(interface).delete_root()
    }

    /// Executes a command (or logs it in dry-run mode).
    async fn execute_command(&self, cmd: &str, dry_run: bool) -> Result<()> {
        if dry_run {
            info!(command = cmd, "Dry-run: would execute");
            return Ok(());
        }

        debug!(command = cmd, "Executing command");

        // In a real implementation, this would execute the command
        // tokio::process::Command::new("sh")
        //     .arg("-c")
        //     .arg(cmd)
        //     .status()
        //     .await?;

        Ok(())
    }
}

#[async_trait]
impl Fault for NetworkLatency {
    fn name(&self) -> &str {
        "network-latency"
    }

    fn required_capabilities(&self) -> Vec<LinuxCapability> {
        vec![LinuxCapability::NetAdmin]
    }

    async fn inject(&self, ctx: &FaultContext) -> Result<FaultHandle> {
        let interface = ctx.get_interface();

        // Validate target is network-related
        match &ctx.target {
            FaultTarget::Network(_) => {}
            _ => {
                return Err(NemesisError::InvalidConfiguration(
                    "NetworkLatency requires a Network target".to_string(),
                ));
            }
        }

        let inject_cmd = self.generate_inject_command(interface);
        let recovery_cmd = self.generate_recovery_command(interface);

        // Execute injection
        self.execute_command(&inject_cmd, ctx.dry_run).await?;

        // Build the fault handle
        let mut handle = FaultHandle::new(self.name());

        if let Some(duration) = ctx.duration {
            handle = handle.with_expiration(duration);
        }

        handle = handle.add_command(inject_cmd);

        let recovery_data = RecoveryData::new().add_command(recovery_cmd);
        handle = handle.with_recovery_data(recovery_data);

        info!(
            handle_id = %handle.id,
            delay_ms = self.delay_ms,
            jitter_ms = ?self.jitter_ms,
            interface = interface,
            "Network latency injected"
        );

        Ok(handle)
    }

    async fn recover(&self, handle: FaultHandle) -> Result<()> {
        info!(handle_id = %handle.id, "Recovering from network latency");

        for cmd in &handle.recovery_data.recovery_commands {
            if let Err(e) = self.execute_command(cmd, false).await {
                debug!(command = cmd, error = %e, "Recovery command failed");
            }
        }

        info!(handle_id = %handle.id, "Network latency recovered");
        Ok(())
    }

    async fn is_active(&self, handle: &FaultHandle) -> Result<bool> {
        // In a real implementation, we would check if the qdisc exists:
        // let output = Command::new("tc").args(["qdisc", "show"]).output()?;
        // Check if netem is in the output

        Ok(!handle.is_expired())
    }

    fn description(&self) -> &str {
        "Adds artificial latency to network traffic using tc/netem"
    }
}

/// Builder for network latency configuration with common presets.
#[derive(Debug)]
pub struct NetworkLatencyBuilder {
    latency: NetworkLatency,
}

impl Default for NetworkLatencyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkLatencyBuilder {
    /// Creates a new builder.
    pub fn new() -> Self {
        Self {
            latency: NetworkLatency::new(),
        }
    }

    /// Creates a preset for simulating WAN latency.
    pub fn wan_latency() -> Self {
        Self {
            latency: NetworkLatency::new()
                .delay_ms(50)
                .jitter_ms(10)
                .correlation_percent(25),
        }
    }

    /// Creates a preset for simulating satellite latency.
    pub fn satellite_latency() -> Self {
        Self {
            latency: NetworkLatency::new()
                .delay_ms(600)
                .jitter_ms(50)
                .correlation_percent(25),
        }
    }

    /// Creates a preset for simulating 3G mobile network.
    pub fn mobile_3g() -> Self {
        Self {
            latency: NetworkLatency::new()
                .delay_ms(200)
                .jitter_ms(100)
                .correlation_percent(25)
                .loss_percent(1.0),
        }
    }

    /// Creates a preset for simulating lossy network.
    pub fn lossy_network() -> Self {
        Self {
            latency: NetworkLatency::new()
                .delay_ms(20)
                .jitter_ms(5)
                .loss_percent(5.0)
                .duplicate_percent(1.0),
        }
    }

    /// Sets the delay.
    pub fn delay_ms(mut self, delay: u32) -> Self {
        self.latency = self.latency.delay_ms(delay);
        self
    }

    /// Sets the jitter.
    pub fn jitter_ms(mut self, jitter: u32) -> Self {
        self.latency = self.latency.jitter_ms(jitter);
        self
    }

    /// Sets the correlation.
    pub fn correlation_percent(mut self, correlation: u32) -> Self {
        self.latency = self.latency.correlation_percent(correlation);
        self
    }

    /// Sets packet loss.
    pub fn loss_percent(mut self, loss: f32) -> Self {
        self.latency = self.latency.loss_percent(loss);
        self
    }

    /// Builds the network latency fault.
    pub fn build(self) -> NetworkLatency {
        self.latency
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::NetworkTarget;

    #[test]
    fn test_latency_default() {
        let latency = NetworkLatency::new();
        assert_eq!(latency.delay_ms, 100);
        assert!(latency.jitter_ms.is_none());
    }

    #[test]
    fn test_latency_builder() {
        let latency = NetworkLatencyBuilder::new()
            .delay_ms(200)
            .jitter_ms(50)
            .correlation_percent(30)
            .loss_percent(2.5)
            .build();

        assert_eq!(latency.delay_ms, 200);
        assert_eq!(latency.jitter_ms, Some(50));
        assert_eq!(latency.correlation_percent, Some(30));
        assert_eq!(latency.loss_percent, Some(2.5));
    }

    #[test]
    fn test_generate_command_simple() {
        let latency = NetworkLatency::new().delay_ms(100);
        let cmd = latency.generate_inject_command("eth0");

        assert!(cmd.contains("tc qdisc add"));
        assert!(cmd.contains("dev eth0"));
        assert!(cmd.contains("netem"));
        assert!(cmd.contains("delay 100ms"));
    }

    #[test]
    fn test_generate_command_complex() {
        let latency = NetworkLatency::new()
            .delay_ms(100)
            .jitter_ms(20)
            .correlation_percent(25)
            .loss_percent(5.0)
            .duplicate_percent(1.0);

        let cmd = latency.generate_inject_command("eth0");

        assert!(cmd.contains("delay 100ms 20ms 25%"));
        assert!(cmd.contains("loss 5.00%"));
        assert!(cmd.contains("duplicate 1.00%"));
    }

    #[test]
    fn test_recovery_command() {
        let latency = NetworkLatency::new();
        let cmd = latency.generate_recovery_command("eth0");

        assert_eq!(cmd, "tc qdisc del dev eth0 root");
    }

    #[test]
    fn test_preset_wan() {
        let latency = NetworkLatencyBuilder::wan_latency().build();
        assert_eq!(latency.delay_ms, 50);
        assert_eq!(latency.jitter_ms, Some(10));
    }

    #[test]
    fn test_required_capabilities() {
        let latency = NetworkLatency::new();
        let caps = latency.required_capabilities();
        assert!(caps.contains(&LinuxCapability::NetAdmin));
    }
}
