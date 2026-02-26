//! Clock jump fault implementation.
//!
//! Instantly jumps the system clock by a specified amount, simulating
//! sudden time changes that can occur during NTP corrections or
//! VM migrations.

use std::time::Duration;

use async_trait::async_trait;
use tracing::{debug, info, warn};

use crate::capability::LinuxCapability;
use crate::context::{FaultContext, FaultHandle, FaultTarget, RecoveryData};
use crate::error::{NemesisError, Result};
use crate::traits::Fault;

use super::{ClockDirection, TimeCommand, TimeOffset};

/// Clock jump fault that instantly changes the system time.
///
/// This fault immediately adjusts the system clock by a specified
/// amount, either forward or backward. This simulates scenarios like:
///
/// - NTP step corrections
/// - VM migrations with time skew
/// - Manual time adjustments
/// - Leap second insertions
///
/// # Example
///
/// ```ignore
/// use mamut_nemesis::faults::clock::ClockJump;
/// use std::time::Duration;
///
/// // Jump 1 hour into the future
/// let jump = ClockJump::forward(Duration::from_secs(3600));
///
/// // Jump 30 seconds into the past
/// let jump = ClockJump::backward(Duration::from_secs(30));
/// ```
///
/// # Safety
///
/// Jumping time backward can cause serious issues:
/// - File timestamps may become inconsistent
/// - Certificates may appear invalid
/// - Scheduled tasks may be skipped or repeated
/// - Databases may have ordering issues
#[derive(Debug, Clone)]
pub struct ClockJump {
    /// The time offset to apply.
    offset: TimeOffset,
    /// Whether to disable NTP during the jump.
    disable_ntp: bool,
    /// Whether to restore the original time on recovery.
    restore_on_recovery: bool,
}

impl Default for ClockJump {
    fn default() -> Self {
        Self::forward(Duration::from_secs(60))
    }
}

impl ClockJump {
    /// Creates a clock jump that moves time forward.
    pub fn forward(amount: Duration) -> Self {
        Self {
            offset: TimeOffset::forward(amount),
            disable_ntp: true,
            restore_on_recovery: true,
        }
    }

    /// Creates a clock jump that moves time backward.
    pub fn backward(amount: Duration) -> Self {
        Self {
            offset: TimeOffset::backward(amount),
            disable_ntp: true,
            restore_on_recovery: true,
        }
    }

    /// Creates a clock jump from a time offset.
    pub fn from_offset(offset: TimeOffset) -> Self {
        Self {
            offset,
            disable_ntp: true,
            restore_on_recovery: true,
        }
    }

    /// Sets whether to disable NTP synchronization.
    pub fn disable_ntp(mut self, disable: bool) -> Self {
        self.disable_ntp = disable;
        self
    }

    /// Sets whether to restore the original time on recovery.
    pub fn restore_on_recovery(mut self, restore: bool) -> Self {
        self.restore_on_recovery = restore;
        self
    }

    /// Returns the time offset.
    pub fn offset(&self) -> &TimeOffset {
        &self.offset
    }

    /// Generates the command to jump time.
    fn generate_jump_command(&self, current_time: f64) -> String {
        let new_time = current_time + self.offset.as_secs() as f64;
        TimeCommand::set_time_date(&format!("{:.6}", new_time))
    }

    /// Generates the command to restore time.
    fn generate_restore_command(&self, original_time: f64, elapsed_secs: f64) -> String {
        // Calculate what the time should be (original + elapsed)
        let correct_time = original_time + elapsed_secs;
        TimeCommand::set_time_date(&format!("{:.6}", correct_time))
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

    /// Gets the current system time as a Unix timestamp.
    async fn get_current_time(&self) -> Result<f64> {
        // In a real implementation:
        // let output = Command::new("date").arg("+%s.%N").output()?;
        // Parse the output

        // For now, use std::time
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| NemesisError::ClockFault(e.to_string()))?;
        Ok(now.as_secs_f64())
    }
}

#[async_trait]
impl Fault for ClockJump {
    fn name(&self) -> &str {
        "clock-jump"
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
                    "ClockJump requires a Clock target".to_string(),
                ));
            }
        }

        let mut inject_commands = Vec::new();
        let mut recovery_commands = Vec::new();

        // Get current time for recovery
        let current_time = self.get_current_time().await?;

        // Optionally disable NTP
        if self.disable_ntp {
            inject_commands.push(TimeCommand::disable_ntp());
            inject_commands.push(TimeCommand::stop_time_sync());

            recovery_commands.push(TimeCommand::enable_ntp());
            recovery_commands.push(TimeCommand::start_time_sync());
        }

        // Generate the jump command
        let jump_cmd = self.generate_jump_command(current_time);
        inject_commands.push(jump_cmd);

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

        // Store original time for recovery
        recovery_data = recovery_data.store_original("original_time", &current_time.to_string());
        recovery_data =
            recovery_data.store_original("offset_secs", &self.offset.as_secs().to_string());
        recovery_data = recovery_data.store_original(
            "restore_on_recovery",
            &self.restore_on_recovery.to_string(),
        );

        handle = handle.with_recovery_data(recovery_data);

        info!(
            handle_id = %handle.id,
            offset_secs = self.offset.as_secs(),
            direction = ?self.offset.direction,
            disable_ntp = self.disable_ntp,
            "Clock jump injected"
        );

        Ok(handle)
    }

    async fn recover(&self, handle: FaultHandle) -> Result<()> {
        info!(handle_id = %handle.id, "Recovering from clock jump");

        // Check if we should restore the time
        let should_restore = handle
            .recovery_data
            .original_values
            .get("restore_on_recovery")
            .map(|v| v == "true")
            .unwrap_or(true);

        if should_restore {
            if let Some(original_time_str) = handle.recovery_data.original_values.get("original_time") {
                if let Ok(original_time) = original_time_str.parse::<f64>() {
                    let elapsed = handle.elapsed().as_secs_f64();
                    let restore_cmd = self.generate_restore_command(original_time, elapsed);

                    if let Err(e) = self.execute_command(&restore_cmd, false).await {
                        warn!(command = restore_cmd, error = %e, "Failed to restore time");
                    }
                }
            }
        }

        // Execute other recovery commands (NTP re-enable, etc.)
        for cmd in &handle.recovery_data.recovery_commands {
            if let Err(e) = self.execute_command(cmd, false).await {
                warn!(command = cmd, error = %e, "Recovery command failed");
            }
        }

        info!(handle_id = %handle.id, "Clock jump recovered");
        Ok(())
    }

    async fn is_active(&self, handle: &FaultHandle) -> Result<bool> {
        // A clock jump is "active" if the time hasn't been restored
        // In a real implementation, we would compare current time with expected

        Ok(!handle.is_expired())
    }

    fn description(&self) -> &str {
        "Instantly jumps the system clock by a specified amount"
    }
}

/// Preset clock jump configurations for common scenarios.
pub struct ClockJumpPresets;

impl ClockJumpPresets {
    /// Simulates a small NTP correction.
    pub fn ntp_correction() -> ClockJump {
        ClockJump::forward(Duration::from_millis(500))
    }

    /// Simulates a VM migration time skew.
    pub fn vm_migration() -> ClockJump {
        ClockJump::forward(Duration::from_secs(5))
    }

    /// Simulates a leap second insertion.
    pub fn leap_second() -> ClockJump {
        ClockJump::backward(Duration::from_secs(1))
    }

    /// Simulates a major time correction (e.g., dead battery).
    pub fn major_correction() -> ClockJump {
        ClockJump::forward(Duration::from_secs(3600)) // 1 hour
    }

    /// Simulates certificate expiry testing.
    pub fn expire_certificates() -> ClockJump {
        ClockJump::forward(Duration::from_secs(86400 * 365)) // 1 year
    }

    /// Simulates going back before certificate validity.
    pub fn invalidate_certificates() -> ClockJump {
        ClockJump::backward(Duration::from_secs(86400 * 365)) // 1 year ago
    }

    /// Simulates Y2K-style date boundary testing.
    pub fn year_boundary() -> ClockJump {
        // Jump to near midnight on Dec 31
        ClockJump::forward(Duration::from_secs(86400 * 30)) // ~1 month
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ClockTarget;

    #[test]
    fn test_jump_forward() {
        let jump = ClockJump::forward(Duration::from_secs(60));
        assert_eq!(jump.offset.direction, ClockDirection::Forward);
        assert_eq!(jump.offset.as_secs(), 60);
    }

    #[test]
    fn test_jump_backward() {
        let jump = ClockJump::backward(Duration::from_secs(60));
        assert_eq!(jump.offset.direction, ClockDirection::Backward);
        assert_eq!(jump.offset.as_secs(), -60);
    }

    #[test]
    fn test_jump_options() {
        let jump = ClockJump::forward(Duration::from_secs(10))
            .disable_ntp(false)
            .restore_on_recovery(false);

        assert!(!jump.disable_ntp);
        assert!(!jump.restore_on_recovery);
    }

    #[test]
    fn test_generate_jump_command() {
        let jump = ClockJump::forward(Duration::from_secs(100));
        let cmd = jump.generate_jump_command(1000.0);

        assert!(cmd.contains("date"));
        assert!(cmd.contains("1100")); // 1000 + 100
    }

    #[test]
    fn test_generate_restore_command() {
        let jump = ClockJump::forward(Duration::from_secs(100));
        let cmd = jump.generate_restore_command(1000.0, 50.0);

        assert!(cmd.contains("date"));
        assert!(cmd.contains("1050")); // 1000 + 50 elapsed
    }

    #[test]
    fn test_required_capabilities() {
        let jump = ClockJump::forward(Duration::from_secs(10));
        let caps = jump.required_capabilities();
        assert!(caps.contains(&LinuxCapability::SysTime));
    }

    #[test]
    fn test_presets() {
        let ntp = ClockJumpPresets::ntp_correction();
        assert!(ntp.offset.amount < Duration::from_secs(1));

        let leap = ClockJumpPresets::leap_second();
        assert_eq!(leap.offset.direction, ClockDirection::Backward);
        assert_eq!(leap.offset.amount, Duration::from_secs(1));
    }
}
