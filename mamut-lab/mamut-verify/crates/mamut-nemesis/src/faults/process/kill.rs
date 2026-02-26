//! Process kill fault implementation.
//!
//! Terminates processes using Unix signals to simulate process failures,
//! crashes, and controlled shutdowns.

use async_trait::async_trait;
use tracing::{debug, info, warn};

use crate::capability::LinuxCapability;
use crate::context::{FaultContext, FaultHandle, FaultTarget, ProcessTarget, RecoveryData};
use crate::error::{NemesisError, Result};
use crate::traits::Fault;

use super::{ProcessCommand, Signal};

/// Process kill fault that terminates processes using signals.
///
/// This fault sends Unix signals to target processes, allowing simulation
/// of various failure scenarios:
///
/// - Graceful shutdown (SIGTERM)
/// - Forced termination (SIGKILL)
/// - Process pause/resume (SIGSTOP/SIGCONT)
/// - Crash simulation (SIGSEGV, SIGABRT)
///
/// # Example
///
/// ```ignore
/// use mamut_nemesis::faults::process::ProcessKill;
/// use mamut_nemesis::faults::process::Signal;
///
/// // Kill a process gracefully
/// let kill = ProcessKill::new(Signal::Term);
///
/// // Force kill with no chance to handle
/// let kill = ProcessKill::new(Signal::Kill);
///
/// // Kill an entire process tree
/// let kill = ProcessKill::new(Signal::Term).kill_children(true);
/// ```
#[derive(Debug, Clone)]
pub struct ProcessKill {
    /// Signal to send.
    signal: Signal,
    /// Whether to also kill child processes.
    kill_children: bool,
    /// Grace period before escalating to SIGKILL (if using SIGTERM).
    grace_period_ms: Option<u64>,
    /// Whether to verify the process is dead after signaling.
    verify_kill: bool,
    /// Number of times to retry if process doesn't die.
    retry_count: u32,
}

impl Default for ProcessKill {
    fn default() -> Self {
        Self::new(Signal::Term)
    }
}

impl ProcessKill {
    /// Creates a new process kill fault with the specified signal.
    pub fn new(signal: Signal) -> Self {
        Self {
            signal,
            kill_children: false,
            grace_period_ms: Some(5000),
            verify_kill: true,
            retry_count: 3,
        }
    }

    /// Creates a graceful kill using SIGTERM.
    pub fn graceful() -> Self {
        Self::new(Signal::Term)
    }

    /// Creates a forceful kill using SIGKILL.
    pub fn forceful() -> Self {
        Self::new(Signal::Kill)
            .grace_period_ms(None)
            .verify_kill(true)
    }

    /// Creates a process pause using SIGSTOP.
    pub fn pause() -> Self {
        Self::new(Signal::Stop)
            .grace_period_ms(None)
            .verify_kill(false)
    }

    /// Sets whether to kill child processes.
    pub fn kill_children(mut self, kill: bool) -> Self {
        self.kill_children = kill;
        self
    }

    /// Sets the grace period before escalating (in milliseconds).
    pub fn grace_period_ms(mut self, period: Option<u64>) -> Self {
        self.grace_period_ms = period;
        self
    }

    /// Sets whether to verify the process is killed.
    pub fn verify_kill(mut self, verify: bool) -> Self {
        self.verify_kill = verify;
        self
    }

    /// Sets the retry count for verification.
    pub fn retry_count(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }

    /// Extracts the process target from the fault context.
    fn get_process_target<'a>(&self, ctx: &'a FaultContext) -> Result<&'a ProcessTarget> {
        match &ctx.target {
            FaultTarget::Process(target) => Ok(target),
            _ => Err(NemesisError::InvalidConfiguration(
                "ProcessKill requires a Process target".to_string(),
            )),
        }
    }

    /// Generates commands for killing by PID.
    fn generate_kill_by_pid(&self, pid: u32) -> Vec<String> {
        let mut commands = Vec::new();

        if self.kill_children {
            commands.push(ProcessCommand::kill_tree(pid, self.signal));
        } else {
            commands.push(ProcessCommand::kill(pid, self.signal));
        }

        // If using a catchable signal, optionally add escalation
        if self.signal.is_catchable() && self.signal.is_fatal() {
            if let Some(grace_ms) = self.grace_period_ms {
                // Add a command to check and escalate after grace period
                commands.push(format!(
                    "sleep {} && {} && kill -9 {} 2>/dev/null || true",
                    grace_ms as f64 / 1000.0,
                    ProcessCommand::check_running(pid),
                    pid
                ));
            }
        }

        commands
    }

    /// Generates commands for killing by name pattern.
    fn generate_kill_by_pattern(&self, pattern: &str, full_match: bool) -> Vec<String> {
        let mut commands = Vec::new();

        if full_match {
            commands.push(ProcessCommand::pkill_full(pattern, self.signal));
        } else {
            commands.push(ProcessCommand::pkill(pattern, self.signal));
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
        // let status = tokio::process::Command::new("sh")
        //     .arg("-c")
        //     .arg(cmd)
        //     .status()
        //     .await?;

        Ok(())
    }

    /// Checks if a process is still running.
    async fn is_process_running(&self, pid: u32) -> Result<bool> {
        // In a real implementation:
        // let status = Command::new("kill")
        //     .args(["-0", &pid.to_string()])
        //     .status()
        //     .await?;
        // Ok(status.success())

        // For now, assume it's not running after we signal it
        Ok(false)
    }
}

#[async_trait]
impl Fault for ProcessKill {
    fn name(&self) -> &str {
        "process-kill"
    }

    fn required_capabilities(&self) -> Vec<LinuxCapability> {
        vec![LinuxCapability::Kill]
    }

    async fn inject(&self, ctx: &FaultContext) -> Result<FaultHandle> {
        let target = self.get_process_target(ctx)?;

        let mut inject_commands = Vec::new();
        let mut affected_pids = Vec::new();

        // Generate commands based on target type
        if let Some(pid) = target.pid {
            inject_commands.extend(self.generate_kill_by_pid(pid));
            affected_pids.push(pid);
        } else if let Some(ref pattern) = target.name_pattern {
            inject_commands.extend(self.generate_kill_by_pattern(pattern, false));
        } else if let Some(ref pattern) = target.cmdline_pattern {
            inject_commands.extend(self.generate_kill_by_pattern(pattern, true));
        } else {
            return Err(NemesisError::InvalidConfiguration(
                "ProcessKill requires a pid, name_pattern, or cmdline_pattern".to_string(),
            ));
        }

        // Execute injection commands
        for cmd in &inject_commands {
            self.execute_command(cmd, ctx.dry_run).await?;
        }

        // Verify kill if requested
        if self.verify_kill && !ctx.dry_run {
            for &pid in &affected_pids {
                for attempt in 0..self.retry_count {
                    if !self.is_process_running(pid).await? {
                        break;
                    }
                    if attempt < self.retry_count - 1 {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                }
            }
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
        for pid in affected_pids {
            recovery_data = recovery_data.add_pid(pid);
        }

        // For SIGSTOP, add SIGCONT as recovery
        if self.signal == Signal::Stop {
            if let Some(pid) = target.pid {
                recovery_data =
                    recovery_data.add_command(ProcessCommand::kill(pid, Signal::Cont));
            }
        }

        recovery_data = recovery_data.store_original("signal", &self.signal.number().to_string());

        handle = handle.with_recovery_data(recovery_data);

        info!(
            handle_id = %handle.id,
            signal = %self.signal,
            target_pid = ?target.pid,
            target_pattern = ?target.name_pattern,
            kill_children = self.kill_children,
            "Process kill injected"
        );

        Ok(handle)
    }

    async fn recover(&self, handle: FaultHandle) -> Result<()> {
        info!(handle_id = %handle.id, "Recovering from process kill");

        // For SIGSTOP, send SIGCONT to resume
        for cmd in &handle.recovery_data.recovery_commands {
            if let Err(e) = self.execute_command(cmd, false).await {
                warn!(command = cmd, error = %e, "Recovery command failed");
            }
        }

        // Note: For fatal signals (TERM, KILL), there's no real recovery -
        // the process is dead. The application would need to be restarted
        // externally.

        info!(handle_id = %handle.id, "Process kill recovered");
        Ok(())
    }

    async fn is_active(&self, handle: &FaultHandle) -> Result<bool> {
        // For stop signals, check if process is still stopped
        if self.signal == Signal::Stop {
            // In a real implementation, check if process state is 'T' (stopped)
            return Ok(!handle.is_expired());
        }

        // For fatal signals, the fault is "active" until recovery is called
        // (even though there's nothing to recover)
        Ok(!handle.is_expired())
    }

    fn description(&self) -> &str {
        "Terminates or signals processes to simulate failures"
    }
}

/// Preset kill configurations for common scenarios.
pub struct ProcessKillPresets;

impl ProcessKillPresets {
    /// Simulates OOM killer behavior.
    pub fn oom_kill() -> ProcessKill {
        ProcessKill::new(Signal::Kill).kill_children(true).verify_kill(true)
    }

    /// Simulates graceful shutdown request.
    pub fn graceful_shutdown() -> ProcessKill {
        ProcessKill::new(Signal::Term)
            .grace_period_ms(Some(30000))
            .verify_kill(true)
    }

    /// Simulates ctrl+c interrupt.
    pub fn keyboard_interrupt() -> ProcessKill {
        ProcessKill::new(Signal::Int)
    }

    /// Simulates config reload signal.
    pub fn reload_config() -> ProcessKill {
        ProcessKill::new(Signal::Hup).verify_kill(false)
    }

    /// Simulates process crash.
    pub fn crash() -> ProcessKill {
        ProcessKill::new(Signal::Segv)
            .grace_period_ms(None)
            .verify_kill(true)
    }

    /// Simulates process freeze/hang.
    pub fn freeze() -> ProcessKill {
        ProcessKill::pause()
    }

    /// Simulates abort with core dump.
    pub fn abort_with_dump() -> ProcessKill {
        ProcessKill::new(Signal::Abrt)
            .grace_period_ms(None)
            .verify_kill(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kill_default() {
        let kill = ProcessKill::default();
        assert_eq!(kill.signal, Signal::Term);
        assert!(!kill.kill_children);
        assert!(kill.verify_kill);
    }

    #[test]
    fn test_kill_graceful() {
        let kill = ProcessKill::graceful();
        assert_eq!(kill.signal, Signal::Term);
        assert!(kill.grace_period_ms.is_some());
    }

    #[test]
    fn test_kill_forceful() {
        let kill = ProcessKill::forceful();
        assert_eq!(kill.signal, Signal::Kill);
        assert!(kill.grace_period_ms.is_none());
    }

    #[test]
    fn test_kill_pause() {
        let kill = ProcessKill::pause();
        assert_eq!(kill.signal, Signal::Stop);
        assert!(!kill.verify_kill);
    }

    #[test]
    fn test_generate_kill_by_pid() {
        let kill = ProcessKill::new(Signal::Term);
        let commands = kill.generate_kill_by_pid(1234);

        assert!(!commands.is_empty());
        assert!(commands[0].contains("kill"));
        assert!(commands[0].contains("1234"));
    }

    #[test]
    fn test_generate_kill_with_children() {
        let kill = ProcessKill::new(Signal::Kill).kill_children(true);
        let commands = kill.generate_kill_by_pid(1234);

        assert!(commands[0].contains("pkill"));
        assert!(commands[0].contains("-P"));
    }

    #[test]
    fn test_generate_kill_by_pattern() {
        let kill = ProcessKill::new(Signal::Term);
        let commands = kill.generate_kill_by_pattern("nginx", false);

        assert!(!commands.is_empty());
        assert!(commands[0].contains("pkill"));
        assert!(commands[0].contains("nginx"));
    }

    #[test]
    fn test_required_capabilities() {
        let kill = ProcessKill::new(Signal::Term);
        let caps = kill.required_capabilities();
        assert!(caps.contains(&LinuxCapability::Kill));
    }

    #[test]
    fn test_presets() {
        let oom = ProcessKillPresets::oom_kill();
        assert_eq!(oom.signal, Signal::Kill);
        assert!(oom.kill_children);

        let freeze = ProcessKillPresets::freeze();
        assert_eq!(freeze.signal, Signal::Stop);
    }
}
