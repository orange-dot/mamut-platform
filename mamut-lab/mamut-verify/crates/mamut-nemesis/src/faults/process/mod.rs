//! Process fault implementations.
//!
//! This module provides faults that affect processes:
//!
//! - `ProcessKill`: Terminates processes using signals

mod kill;

pub use kill::ProcessKill;

use std::fmt;

/// Unix signal types for process manipulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// SIGTERM (15) - Graceful termination request.
    Term,
    /// SIGKILL (9) - Forceful termination (cannot be caught).
    Kill,
    /// SIGSTOP (19) - Stop/pause process.
    Stop,
    /// SIGCONT (18) - Continue stopped process.
    Cont,
    /// SIGHUP (1) - Hangup, often used to reload config.
    Hup,
    /// SIGINT (2) - Interrupt from keyboard (Ctrl+C).
    Int,
    /// SIGQUIT (3) - Quit with core dump.
    Quit,
    /// SIGABRT (6) - Abort signal.
    Abrt,
    /// SIGUSR1 (10) - User-defined signal 1.
    Usr1,
    /// SIGUSR2 (12) - User-defined signal 2.
    Usr2,
    /// SIGPIPE (13) - Broken pipe.
    Pipe,
    /// SIGALRM (14) - Alarm clock.
    Alrm,
    /// SIGSEGV (11) - Segmentation fault.
    Segv,
}

impl Signal {
    /// Returns the signal number.
    pub fn number(&self) -> i32 {
        match self {
            Signal::Term => 15,
            Signal::Kill => 9,
            Signal::Stop => 19,
            Signal::Cont => 18,
            Signal::Hup => 1,
            Signal::Int => 2,
            Signal::Quit => 3,
            Signal::Abrt => 6,
            Signal::Usr1 => 10,
            Signal::Usr2 => 12,
            Signal::Pipe => 13,
            Signal::Alrm => 14,
            Signal::Segv => 11,
        }
    }

    /// Returns the signal name (without SIG prefix).
    pub fn name(&self) -> &'static str {
        match self {
            Signal::Term => "TERM",
            Signal::Kill => "KILL",
            Signal::Stop => "STOP",
            Signal::Cont => "CONT",
            Signal::Hup => "HUP",
            Signal::Int => "INT",
            Signal::Quit => "QUIT",
            Signal::Abrt => "ABRT",
            Signal::Usr1 => "USR1",
            Signal::Usr2 => "USR2",
            Signal::Pipe => "PIPE",
            Signal::Alrm => "ALRM",
            Signal::Segv => "SEGV",
        }
    }

    /// Returns whether this signal can be caught/handled by the process.
    pub fn is_catchable(&self) -> bool {
        !matches!(self, Signal::Kill | Signal::Stop)
    }

    /// Returns whether this signal typically causes process termination.
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            Signal::Term
                | Signal::Kill
                | Signal::Int
                | Signal::Quit
                | Signal::Abrt
                | Signal::Segv
        )
    }

    /// Creates a signal from a number.
    pub fn from_number(num: i32) -> Option<Self> {
        match num {
            15 => Some(Signal::Term),
            9 => Some(Signal::Kill),
            19 => Some(Signal::Stop),
            18 => Some(Signal::Cont),
            1 => Some(Signal::Hup),
            2 => Some(Signal::Int),
            3 => Some(Signal::Quit),
            6 => Some(Signal::Abrt),
            10 => Some(Signal::Usr1),
            12 => Some(Signal::Usr2),
            13 => Some(Signal::Pipe),
            14 => Some(Signal::Alrm),
            11 => Some(Signal::Segv),
            _ => None,
        }
    }
}

impl fmt::Display for Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SIG{}", self.name())
    }
}

/// Helper for generating process-related commands.
pub(crate) struct ProcessCommand;

impl ProcessCommand {
    /// Generates a kill command.
    pub fn kill(pid: u32, signal: Signal) -> String {
        format!("kill -{} {}", signal.number(), pid)
    }

    /// Generates a kill command with signal name.
    pub fn kill_named(pid: u32, signal: Signal) -> String {
        format!("kill -s {} {}", signal.name(), pid)
    }

    /// Generates a pkill command by name pattern.
    pub fn pkill(pattern: &str, signal: Signal) -> String {
        format!("pkill -{} '{}'", signal.number(), pattern)
    }

    /// Generates a pkill command with full matching.
    pub fn pkill_full(pattern: &str, signal: Signal) -> String {
        format!("pkill -{} -f '{}'", signal.number(), pattern)
    }

    /// Generates a pgrep command to find PIDs.
    pub fn pgrep(pattern: &str) -> String {
        format!("pgrep '{}'", pattern)
    }

    /// Generates a pgrep command with full matching.
    pub fn pgrep_full(pattern: &str) -> String {
        format!("pgrep -f '{}'", pattern)
    }

    /// Generates a command to check if a process is running.
    pub fn check_running(pid: u32) -> String {
        format!("kill -0 {} 2>/dev/null", pid)
    }

    /// Generates a command to get process info.
    pub fn ps_info(pid: u32) -> String {
        format!("ps -p {} -o pid,ppid,user,comm,state", pid)
    }

    /// Generates a command to get all child PIDs.
    pub fn get_children(pid: u32) -> String {
        format!("pgrep -P {}", pid)
    }

    /// Generates a command to kill a process tree.
    pub fn kill_tree(pid: u32, signal: Signal) -> String {
        format!(
            "pkill -{} -P {} && kill -{} {}",
            signal.number(),
            pid,
            signal.number(),
            pid
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_numbers() {
        assert_eq!(Signal::Kill.number(), 9);
        assert_eq!(Signal::Term.number(), 15);
        assert_eq!(Signal::Stop.number(), 19);
    }

    #[test]
    fn test_signal_names() {
        assert_eq!(Signal::Kill.name(), "KILL");
        assert_eq!(Signal::Term.name(), "TERM");
        assert_eq!(Signal::Usr1.name(), "USR1");
    }

    #[test]
    fn test_signal_catchable() {
        assert!(!Signal::Kill.is_catchable());
        assert!(!Signal::Stop.is_catchable());
        assert!(Signal::Term.is_catchable());
        assert!(Signal::Int.is_catchable());
    }

    #[test]
    fn test_signal_fatal() {
        assert!(Signal::Kill.is_fatal());
        assert!(Signal::Term.is_fatal());
        assert!(!Signal::Stop.is_fatal());
        assert!(!Signal::Cont.is_fatal());
    }

    #[test]
    fn test_signal_from_number() {
        assert_eq!(Signal::from_number(9), Some(Signal::Kill));
        assert_eq!(Signal::from_number(15), Some(Signal::Term));
        assert_eq!(Signal::from_number(999), None);
    }

    #[test]
    fn test_process_commands() {
        assert_eq!(ProcessCommand::kill(1234, Signal::Term), "kill -15 1234");
        assert_eq!(
            ProcessCommand::kill_named(1234, Signal::Kill),
            "kill -s KILL 1234"
        );
        assert_eq!(
            ProcessCommand::pkill("nginx", Signal::Hup),
            "pkill -1 'nginx'"
        );
    }

    #[test]
    fn test_signal_display() {
        assert_eq!(format!("{}", Signal::Kill), "SIGKILL");
        assert_eq!(format!("{}", Signal::Term), "SIGTERM");
    }
}
