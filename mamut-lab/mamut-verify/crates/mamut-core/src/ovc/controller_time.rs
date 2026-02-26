//! Controller Time - Monotonic ground truth time from the test controller.
//!
//! `ControllerTime` represents the authoritative time source in the OVC system.
//! It is a monotonically increasing nanosecond counter maintained by the test
//! controller, providing a single source of truth for ordering events across
//! all nodes in the distributed system under test.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Sub};
use std::time::Duration;

/// Monotonic nanosecond timestamp from the test controller.
///
/// This is the ground truth time source that all other time measurements
/// are compared against. It is guaranteed to be monotonically increasing
/// and consistent across all observations.
///
/// # Example
///
/// ```
/// use mamut_core::ovc::ControllerTime;
///
/// let t1 = ControllerTime::from_nanos(1_000_000_000);
/// let t2 = ControllerTime::from_nanos(2_000_000_000);
///
/// assert!(t1 < t2);
/// assert_eq!(t2.as_nanos() - t1.as_nanos(), 1_000_000_000);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ControllerTime(u128);

impl ControllerTime {
    /// The epoch (zero) time.
    pub const EPOCH: ControllerTime = ControllerTime(0);

    /// Maximum representable time.
    pub const MAX: ControllerTime = ControllerTime(u128::MAX);

    /// Creates a new `ControllerTime` from nanoseconds.
    #[inline]
    pub const fn from_nanos(nanos: u128) -> Self {
        ControllerTime(nanos)
    }

    /// Creates a new `ControllerTime` from microseconds.
    #[inline]
    pub const fn from_micros(micros: u128) -> Self {
        ControllerTime(micros * 1_000)
    }

    /// Creates a new `ControllerTime` from milliseconds.
    #[inline]
    pub const fn from_millis(millis: u128) -> Self {
        ControllerTime(millis * 1_000_000)
    }

    /// Creates a new `ControllerTime` from seconds.
    #[inline]
    pub const fn from_secs(secs: u64) -> Self {
        ControllerTime(secs as u128 * 1_000_000_000)
    }

    /// Returns the time as nanoseconds.
    #[inline]
    pub const fn as_nanos(&self) -> u128 {
        self.0
    }

    /// Returns the time as microseconds (truncated).
    #[inline]
    pub const fn as_micros(&self) -> u128 {
        self.0 / 1_000
    }

    /// Returns the time as milliseconds (truncated).
    #[inline]
    pub const fn as_millis(&self) -> u128 {
        self.0 / 1_000_000
    }

    /// Returns the time as seconds (truncated).
    #[inline]
    pub const fn as_secs(&self) -> u64 {
        (self.0 / 1_000_000_000) as u64
    }

    /// Returns the fractional nanoseconds part (0 to 999,999,999).
    #[inline]
    pub const fn subsec_nanos(&self) -> u32 {
        (self.0 % 1_000_000_000) as u32
    }

    /// Converts to a `Duration` if the time fits within `Duration`'s range.
    #[inline]
    pub fn to_duration(&self) -> Option<Duration> {
        if self.0 <= u64::MAX as u128 * 1_000_000_000 + 999_999_999 {
            Some(Duration::new(self.as_secs(), self.subsec_nanos()))
        } else {
            None
        }
    }

    /// Creates from a `Duration`.
    #[inline]
    pub fn from_duration(d: Duration) -> Self {
        ControllerTime(d.as_nanos())
    }

    /// Saturating addition.
    #[inline]
    pub fn saturating_add(self, nanos: u128) -> Self {
        ControllerTime(self.0.saturating_add(nanos))
    }

    /// Saturating subtraction.
    #[inline]
    pub fn saturating_sub(self, nanos: u128) -> Self {
        ControllerTime(self.0.saturating_sub(nanos))
    }

    /// Checked addition.
    #[inline]
    pub fn checked_add(self, nanos: u128) -> Option<Self> {
        self.0.checked_add(nanos).map(ControllerTime)
    }

    /// Checked subtraction.
    #[inline]
    pub fn checked_sub(self, nanos: u128) -> Option<Self> {
        self.0.checked_sub(nanos).map(ControllerTime)
    }

    /// Returns the absolute difference between two times.
    #[inline]
    pub fn abs_diff(self, other: Self) -> u128 {
        if self.0 >= other.0 {
            self.0 - other.0
        } else {
            other.0 - self.0
        }
    }
}

impl Default for ControllerTime {
    fn default() -> Self {
        Self::EPOCH
    }
}

impl fmt::Display for ControllerTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let secs = self.as_secs();
        let nanos = self.subsec_nanos();
        write!(f, "{}.{:09}s", secs, nanos)
    }
}

impl Add<u128> for ControllerTime {
    type Output = Self;

    fn add(self, nanos: u128) -> Self::Output {
        ControllerTime(self.0 + nanos)
    }
}

impl Sub<u128> for ControllerTime {
    type Output = Self;

    fn sub(self, nanos: u128) -> Self::Output {
        ControllerTime(self.0 - nanos)
    }
}

impl Sub for ControllerTime {
    type Output = i128;

    fn sub(self, other: Self) -> Self::Output {
        self.0 as i128 - other.0 as i128
    }
}

impl Add<Duration> for ControllerTime {
    type Output = Self;

    fn add(self, duration: Duration) -> Self::Output {
        ControllerTime(self.0 + duration.as_nanos())
    }
}

impl Sub<Duration> for ControllerTime {
    type Output = Self;

    fn sub(self, duration: Duration) -> Self::Output {
        ControllerTime(self.0 - duration.as_nanos())
    }
}

impl From<u128> for ControllerTime {
    fn from(nanos: u128) -> Self {
        ControllerTime(nanos)
    }
}

impl From<ControllerTime> for u128 {
    fn from(ct: ControllerTime) -> Self {
        ct.0
    }
}

impl From<Duration> for ControllerTime {
    fn from(d: Duration) -> Self {
        Self::from_duration(d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_units() {
        assert_eq!(ControllerTime::from_nanos(1_000).as_nanos(), 1_000);
        assert_eq!(ControllerTime::from_micros(1).as_nanos(), 1_000);
        assert_eq!(ControllerTime::from_millis(1).as_nanos(), 1_000_000);
        assert_eq!(ControllerTime::from_secs(1).as_nanos(), 1_000_000_000);
    }

    #[test]
    fn test_ordering() {
        let t1 = ControllerTime::from_nanos(100);
        let t2 = ControllerTime::from_nanos(200);

        assert!(t1 < t2);
        assert!(t2 > t1);
        assert_eq!(t1, ControllerTime::from_nanos(100));
    }

    #[test]
    fn test_arithmetic() {
        let t1 = ControllerTime::from_nanos(100);
        let t2 = t1 + 50;

        assert_eq!(t2.as_nanos(), 150);
        assert_eq!((t2 - 50).as_nanos(), 100);
        assert_eq!(t2 - t1, 50);
    }

    #[test]
    fn test_saturating_ops() {
        let t = ControllerTime::from_nanos(100);

        assert_eq!(t.saturating_sub(200), ControllerTime::EPOCH);
        assert_eq!(ControllerTime::MAX.saturating_add(1), ControllerTime::MAX);
    }

    #[test]
    fn test_abs_diff() {
        let t1 = ControllerTime::from_nanos(100);
        let t2 = ControllerTime::from_nanos(250);

        assert_eq!(t1.abs_diff(t2), 150);
        assert_eq!(t2.abs_diff(t1), 150);
    }

    #[test]
    fn test_display() {
        let t = ControllerTime::from_nanos(1_234_567_890);
        assert_eq!(format!("{}", t), "1.234567890s");
    }

    #[test]
    fn test_duration_conversion() {
        let d = Duration::from_secs(5) + Duration::from_nanos(123);
        let t = ControllerTime::from_duration(d);

        assert_eq!(t.as_secs(), 5);
        assert_eq!(t.subsec_nanos(), 123);
        assert_eq!(t.to_duration(), Some(d));
    }

    #[test]
    fn test_serde() {
        let t = ControllerTime::from_nanos(12345);
        let json = serde_json::to_string(&t).unwrap();
        let t2: ControllerTime = serde_json::from_str(&json).unwrap();
        assert_eq!(t, t2);
    }
}
