//! Clock fault implementations.
//!
//! This module provides faults that affect system time:
//!
//! - `ClockDrift`: Gradually skews time forward or backward
//! - `ClockJump`: Instantly jumps time by a specified amount

mod drift;
mod jump;

pub use drift::ClockDrift;
pub use jump::ClockJump;

use std::time::Duration;

/// Direction of clock manipulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockDirection {
    /// Move time forward (into the future).
    Forward,
    /// Move time backward (into the past).
    Backward,
}

impl ClockDirection {
    /// Returns the multiplier for this direction.
    pub fn multiplier(&self) -> i64 {
        match self {
            ClockDirection::Forward => 1,
            ClockDirection::Backward => -1,
        }
    }
}

/// Helper for generating time manipulation commands.
pub(crate) struct TimeCommand;

impl TimeCommand {
    /// Generates a command to read the current system time.
    pub fn read_time() -> String {
        "date +%s.%N".to_string()
    }

    /// Generates a command to set the system time using date.
    pub fn set_time_date(timestamp: &str) -> String {
        format!("date -s '@{}'", timestamp)
    }

    /// Generates a command to set the time using timedatectl.
    pub fn set_time_timedatectl(datetime: &str) -> String {
        format!("timedatectl set-time '{}'", datetime)
    }

    /// Generates a command to adjust time using adjtime (for gradual drift).
    /// The offset is in microseconds.
    pub fn adjust_time(offset_usec: i64) -> String {
        // Using adjtimex to adjust the clock
        // This is a simplified representation; actual implementation would use
        // the adjtimex system call
        format!("adjtimex --offset {}", offset_usec)
    }

    /// Generates a command to set the clock tick adjustment.
    /// Normal tick is 10000 usec. Adjusting creates drift.
    pub fn set_tick(tick_usec: u32) -> String {
        format!("adjtimex --tick {}", tick_usec)
    }

    /// Generates a command to reset tick to normal.
    pub fn reset_tick() -> String {
        "adjtimex --tick 10000".to_string()
    }

    /// Generates a command to disable NTP synchronization.
    pub fn disable_ntp() -> String {
        "timedatectl set-ntp false".to_string()
    }

    /// Generates a command to enable NTP synchronization.
    pub fn enable_ntp() -> String {
        "timedatectl set-ntp true".to_string()
    }

    /// Generates a command to stop chronyd/ntpd services.
    pub fn stop_time_sync() -> String {
        "systemctl stop chronyd ntpd 2>/dev/null || true".to_string()
    }

    /// Generates a command to start chronyd/ntpd services.
    pub fn start_time_sync() -> String {
        "systemctl start chronyd 2>/dev/null || systemctl start ntpd 2>/dev/null || true"
            .to_string()
    }

    /// Calculates the tick value for a given drift rate.
    ///
    /// A drift rate of 1.0 means normal time (no drift).
    /// A drift rate of 1.1 means time runs 10% faster.
    /// A drift rate of 0.9 means time runs 10% slower.
    pub fn tick_for_drift_rate(drift_rate: f64) -> u32 {
        // Normal tick is 10000 usec
        // Adjusting tick changes how fast the clock advances
        let base_tick = 10000.0;
        (base_tick * drift_rate) as u32
    }
}

/// Represents a time offset for clock manipulation.
#[derive(Debug, Clone, Copy)]
pub struct TimeOffset {
    /// The offset amount.
    pub amount: Duration,
    /// The direction of the offset.
    pub direction: ClockDirection,
}

impl TimeOffset {
    /// Creates a forward time offset.
    pub fn forward(amount: Duration) -> Self {
        Self {
            amount,
            direction: ClockDirection::Forward,
        }
    }

    /// Creates a backward time offset.
    pub fn backward(amount: Duration) -> Self {
        Self {
            amount,
            direction: ClockDirection::Backward,
        }
    }

    /// Returns the offset in seconds (positive or negative).
    pub fn as_secs(&self) -> i64 {
        self.direction.multiplier() * self.amount.as_secs() as i64
    }

    /// Returns the offset in milliseconds (positive or negative).
    pub fn as_millis(&self) -> i64 {
        self.direction.multiplier() * self.amount.as_millis() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_direction() {
        assert_eq!(ClockDirection::Forward.multiplier(), 1);
        assert_eq!(ClockDirection::Backward.multiplier(), -1);
    }

    #[test]
    fn test_time_offset() {
        let forward = TimeOffset::forward(Duration::from_secs(60));
        assert_eq!(forward.as_secs(), 60);

        let backward = TimeOffset::backward(Duration::from_secs(60));
        assert_eq!(backward.as_secs(), -60);
    }

    #[test]
    fn test_time_commands() {
        assert!(TimeCommand::read_time().contains("date"));
        assert!(TimeCommand::disable_ntp().contains("timedatectl"));
        assert!(TimeCommand::reset_tick().contains("10000"));
    }

    #[test]
    fn test_tick_calculation() {
        // Normal rate
        assert_eq!(TimeCommand::tick_for_drift_rate(1.0), 10000);
        // 10% faster
        assert_eq!(TimeCommand::tick_for_drift_rate(1.1), 11000);
        // 10% slower
        assert_eq!(TimeCommand::tick_for_drift_rate(0.9), 9000);
    }
}
