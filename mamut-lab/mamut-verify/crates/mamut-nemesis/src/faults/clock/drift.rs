//! Clock drift fault implementation.
//!
//! Gradually skews the system clock to simulate clock drift in
//! distributed systems.

use std::time::Duration;

use async_trait::async_trait;
use tracing::{debug, info, warn};

use crate::capability::LinuxCapability;
use crate::context::{FaultContext, FaultHandle, FaultTarget, RecoveryData};
use crate::error::{NemesisError, Result};
use crate::traits::Fault;

use super::{ClockDirection, TimeCommand};

/// Clock drift fault that gradually skews the system time.
///
/// This fault adjusts the system clock tick rate to make time run
/// faster or slower than real time. This simulates clock drift that
/// can occur in distributed systems due to hardware variations.
///
/// # Example
///
/// ```ignore
/// use mamut_nemesis::faults::clock::ClockDrift;
///
/// // Create a drift that makes time run 10% faster
/// let drift = ClockDrift::new()
///     .drift_rate(1.1)
///     .disable_ntp(true);
/// ```
///
/// # Notes
///
/// - Requires CAP_SYS_TIME capability
/// - NTP synchronization should be disabled during drift injection
/// - The drift accumulates over time
#[derive(Debug, Clone)]
pub struct ClockDrift {
    /// Drift rate multiplier. 1.0 = normal, 1.1 = 10% faster, 0.9 = 10% slower.
    drift_rate: f64,
    /// Whether to automatically disable NTP during injection.
    disable_ntp: bool,
    /// Maximum accumulated drift before auto-recovery.
    max_drift: Option<Duration>,
    /// Direction of drift (derived from rate).
    direction: ClockDirection,
}

impl Default for ClockDrift {
    fn default() -> Self {
        Self::new()
    }
}

impl ClockDrift {
    /// Creates a new clock drift fault with default values.
    pub fn new() -> Self {
        Self {
            drift_rate: 1.01, // 1% faster by default
            disable_ntp: true,
            max_drift: None,
            direction: ClockDirection::Forward,
        }
    }

    /// Sets the drift rate.
    ///
    /// - Values > 1.0 make time run faster
    /// - Values < 1.0 make time run slower
    /// - Value of 1.0 means no drift
    ///
    /// # Panics
    ///
    /// Panics if drift_rate is <= 0.
    pub fn drift_rate(mut self, rate: f64) -> Self {
        assert!(rate > 0.0, "Drift rate must be positive");
        self.drift_rate = rate;
        self.direction = if rate >= 1.0 {
            ClockDirection::Forward
        } else {
            ClockDirection::Backward
        };
        self
    }

    /// Sets whether to disable NTP synchronization.
    pub fn disable_ntp(mut self, disable: bool) -> Self {
        self.disable_ntp = disable;
        self
    }

    /// Sets the maximum accumulated drift before auto-recovery.
    pub fn max_drift(mut self, max: Duration) -> Self {
        self.max_drift = Some(max);
        self
    }

    /// Creates a drift that makes time run faster by the given percentage.
    pub fn faster_by_percent(percent: f64) -> Self {
        Self::new().drift_rate(1.0 + percent / 100.0)
    }

    /// Creates a drift that makes time run slower by the given percentage.
    pub fn slower_by_percent(percent: f64) -> Self {
        Self::new().drift_rate(1.0 - percent / 100.0)
    }

    /// Generates commands for injecting the drift.
    fn generate_inject_commands(&self) -> Vec<String> {
        let mut commands = Vec::new();

        // Optionally disable NTP
        if self.disable_ntp {
            commands.push(TimeCommand::disable_ntp());
            commands.push(TimeCommand::stop_time_sync());
        }

        // Set the tick rate for drift
        let tick = TimeCommand::tick_for_drift_rate(self.drift_rate);
        commands.push(TimeCommand::set_tick(tick));

        commands
    }

    /// Generates commands for recovering from the drift.
    fn generate_recovery_commands(&self) -> Vec<String> {
        let mut commands = Vec::new();

        // Reset tick to normal
        commands.push(TimeCommand::reset_tick());

        // Re-enable NTP if we disabled it
        if self.disable_ntp {
            commands.push(TimeCommand::enable_ntp());
            commands.push(TimeCommand::start_time_sync());
        }

        commands
    }

    /// Executes a command (or logs it in dry-run mode).
    async fn execute_command(&self, cmd: &str, dry_run: bool) -> Result<()> {
        if dry_run {
            info!(command = cmd, "Dry-run: would execute");
            return Ok(());
        }

        debug!(command = cmd, "Executing command");

        // In a real implementation:
        // tokio::process::Command::new("sh")
        //     .arg("-c")
        //     .arg(cmd)
        //     .status()
        //     .await?;

        Ok(())
    }

    /// Calculates the expected accumulated drift after a given duration.
    pub fn expected_drift(&self, duration: Duration) -> Duration {
        let drift_factor = (self.drift_rate - 1.0).abs();
        Duration::from_secs_f64(duration.as_secs_f64() * drift_factor)
    }
}

#[async_trait]
impl Fault for ClockDrift {
    fn name(&self) -> &str {
        "clock-drift"
    }

    fn required_capabilities(&self) -> Vec<LinuxCapability> {
        vec![LinuxCapability::SysTime]
    }

    async fn inject(&self, ctx: &FaultContext) -> Result<FaultHandle> {
        // Validate target
        match &ctx.target {
            FaultTarget::Clock(_) => {}
            _ => {
                return Err(NemesisError::InvalidConfiguration(
                    "ClockDrift requires a Clock target".to_string(),
                ));
            }
        }

        let inject_commands = self.generate_inject_commands();
        let recovery_commands = self.generate_recovery_commands();

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

        let mut recovery_data = RecoveryData::new();
        for cmd in recovery_commands {
            recovery_data = recovery_data.add_command(cmd);
        }

        // Store original tick value for verification
        recovery_data = recovery_data.store_original("tick", "10000");
        recovery_data = recovery_data.store_original("drift_rate", &self.drift_rate.to_string());

        handle = handle.with_recovery_data(recovery_data);

        info!(
            handle_id = %handle.id,
            drift_rate = self.drift_rate,
            direction = ?self.direction,
            disable_ntp = self.disable_ntp,
            "Clock drift injected"
        );

        Ok(handle)
    }

    async fn recover(&self, handle: FaultHandle) -> Result<()> {
        info!(handle_id = %handle.id, "Recovering from clock drift");

        for cmd in &handle.recovery_data.recovery_commands {
            if let Err(e) = self.execute_command(cmd, false).await {
                warn!(command = cmd, error = %e, "Recovery command failed");
            }
        }

        info!(handle_id = %handle.id, "Clock drift recovered");
        Ok(())
    }

    async fn is_active(&self, handle: &FaultHandle) -> Result<bool> {
        // In a real implementation, we would check the current tick value:
        // let output = Command::new("adjtimex").arg("--print").output()?;
        // Parse the tick value and compare with 10000

        Ok(!handle.is_expired())
    }

    fn description(&self) -> &str {
        "Gradually skews the system clock by adjusting the tick rate"
    }
}

/// Preset drift configurations for common scenarios.
pub struct ClockDriftPresets;

impl ClockDriftPresets {
    /// Simulates a fast clock (common in VMs under load).
    pub fn fast_vm() -> ClockDrift {
        ClockDrift::faster_by_percent(5.0)
    }

    /// Simulates a slow clock (common with power-saving).
    pub fn slow_powersave() -> ClockDrift {
        ClockDrift::slower_by_percent(2.0)
    }

    /// Simulates extreme drift for stress testing.
    pub fn extreme_fast() -> ClockDrift {
        ClockDrift::faster_by_percent(50.0)
    }

    /// Simulates extreme slow drift.
    pub fn extreme_slow() -> ClockDrift {
        ClockDrift::slower_by_percent(50.0)
    }

    /// Minimal drift for subtle timing issues.
    pub fn subtle() -> ClockDrift {
        ClockDrift::faster_by_percent(0.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ClockTarget;

    #[test]
    fn test_drift_default() {
        let drift = ClockDrift::new();
        assert!(drift.drift_rate > 1.0); // Default is slightly fast
        assert!(drift.disable_ntp);
    }

    #[test]
    fn test_drift_rate_setting() {
        let drift = ClockDrift::new().drift_rate(1.1);
        assert_eq!(drift.drift_rate, 1.1);
        assert_eq!(drift.direction, ClockDirection::Forward);

        let drift = ClockDrift::new().drift_rate(0.9);
        assert_eq!(drift.drift_rate, 0.9);
        assert_eq!(drift.direction, ClockDirection::Backward);
    }

    #[test]
    fn test_faster_slower_helpers() {
        let fast = ClockDrift::faster_by_percent(10.0);
        assert!((fast.drift_rate - 1.1).abs() < 0.001);

        let slow = ClockDrift::slower_by_percent(10.0);
        assert!((slow.drift_rate - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_expected_drift() {
        let drift = ClockDrift::new().drift_rate(1.1); // 10% faster
        let expected = drift.expected_drift(Duration::from_secs(100));
        // After 100 seconds at 10% drift, we expect ~10 seconds of drift
        assert!((expected.as_secs_f64() - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_generate_commands() {
        let drift = ClockDrift::new().drift_rate(1.1).disable_ntp(true);
        let commands = drift.generate_inject_commands();

        assert!(commands.iter().any(|c| c.contains("timedatectl")));
        assert!(commands.iter().any(|c| c.contains("adjtimex")));
    }

    #[test]
    fn test_required_capabilities() {
        let drift = ClockDrift::new();
        let caps = drift.required_capabilities();
        assert!(caps.contains(&LinuxCapability::SysTime));
    }

    #[test]
    fn test_presets() {
        let fast_vm = ClockDriftPresets::fast_vm();
        assert!(fast_vm.drift_rate > 1.0);

        let slow = ClockDriftPresets::slow_powersave();
        assert!(slow.drift_rate < 1.0);
    }

    #[test]
    #[should_panic(expected = "Drift rate must be positive")]
    fn test_invalid_drift_rate() {
        ClockDrift::new().drift_rate(0.0);
    }
}
