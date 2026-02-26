//! Uncertainty Interval - Time bounds with confidence levels.
//!
//! `UncertaintyInterval` represents a bounded range of possible controller times
//! when the exact time of an event cannot be determined precisely. This is common
//! in distributed systems where clock synchronization introduces uncertainty.

use serde::{Deserialize, Serialize};
use std::fmt;

use super::controller_time::ControllerTime;

/// A time interval representing uncertainty about when an event occurred.
///
/// In distributed systems, we often cannot know the exact time of an event.
/// Instead, we can bound it: the event occurred somewhere between `earliest`
/// and `latest` controller time, with a given confidence level.
///
/// # Fields
///
/// - `earliest`: The earliest possible time the event could have occurred
/// - `latest`: The latest possible time the event could have occurred
/// - `confidence`: Probability (0.0 to 1.0) that the true time is within bounds
///
/// # Example
///
/// ```
/// use mamut_core::ovc::{UncertaintyInterval, ControllerTime};
///
/// let interval = UncertaintyInterval::new(
///     ControllerTime::from_millis(100),
///     ControllerTime::from_millis(150),
///     0.99,
/// );
///
/// assert!(interval.contains(ControllerTime::from_millis(125)));
/// assert_eq!(interval.width_nanos(), 50_000_000); // 50ms
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UncertaintyInterval {
    /// Earliest possible time for the event.
    earliest: ControllerTime,

    /// Latest possible time for the event.
    latest: ControllerTime,

    /// Confidence level (0.0 to 1.0) that the true time is within bounds.
    confidence: f64,
}

impl UncertaintyInterval {
    /// Creates a new uncertainty interval.
    ///
    /// # Panics
    ///
    /// Panics if `earliest > latest` or if `confidence` is not in [0.0, 1.0].
    pub fn new(earliest: ControllerTime, latest: ControllerTime, confidence: f64) -> Self {
        assert!(
            earliest <= latest,
            "earliest ({:?}) must be <= latest ({:?})",
            earliest,
            latest
        );
        assert!(
            (0.0..=1.0).contains(&confidence),
            "confidence must be in [0.0, 1.0], got {}",
            confidence
        );

        UncertaintyInterval {
            earliest,
            latest,
            confidence,
        }
    }

    /// Creates an uncertainty interval from a point time with symmetric error bounds.
    ///
    /// # Arguments
    ///
    /// * `center` - The estimated time
    /// * `error_nanos` - The error bound in nanoseconds (applied symmetrically)
    /// * `confidence` - Confidence level
    pub fn symmetric(center: ControllerTime, error_nanos: u128, confidence: f64) -> Self {
        let earliest = center.saturating_sub(error_nanos);
        let latest = center.saturating_add(error_nanos);
        Self::new(earliest, latest, confidence)
    }

    /// Creates a point interval (zero uncertainty) with 100% confidence.
    #[inline]
    pub fn exact(time: ControllerTime) -> Self {
        UncertaintyInterval {
            earliest: time,
            latest: time,
            confidence: 1.0,
        }
    }

    /// Returns the earliest bound.
    #[inline]
    pub const fn earliest(&self) -> ControllerTime {
        self.earliest
    }

    /// Returns the latest bound.
    #[inline]
    pub const fn latest(&self) -> ControllerTime {
        self.latest
    }

    /// Returns the confidence level.
    #[inline]
    pub const fn confidence(&self) -> f64 {
        self.confidence
    }

    /// Returns the width of the interval in nanoseconds.
    #[inline]
    pub fn width_nanos(&self) -> u128 {
        self.latest.as_nanos() - self.earliest.as_nanos()
    }

    /// Returns the midpoint of the interval.
    #[inline]
    pub fn midpoint(&self) -> ControllerTime {
        let mid = (self.earliest.as_nanos() + self.latest.as_nanos()) / 2;
        ControllerTime::from_nanos(mid)
    }

    /// Returns true if this is an exact point (zero width).
    #[inline]
    pub fn is_exact(&self) -> bool {
        self.earliest == self.latest
    }

    /// Returns true if a specific time falls within this interval.
    #[inline]
    pub fn contains(&self, time: ControllerTime) -> bool {
        time >= self.earliest && time <= self.latest
    }

    /// Returns true if two intervals overlap.
    ///
    /// Two intervals overlap if there exists some time that could be in both.
    #[inline]
    pub fn overlaps(&self, other: &UncertaintyInterval) -> bool {
        self.earliest <= other.latest && other.earliest <= self.latest
    }

    /// Returns true if this interval must precede another.
    ///
    /// This is true when even the latest possible time for this interval
    /// is before the earliest possible time for the other interval.
    #[inline]
    pub fn must_precede(&self, other: &UncertaintyInterval) -> bool {
        self.latest < other.earliest
    }

    /// Returns true if this interval must follow another.
    #[inline]
    pub fn must_follow(&self, other: &UncertaintyInterval) -> bool {
        other.must_precede(self)
    }

    /// Returns the intersection of two intervals, if they overlap.
    ///
    /// The confidence of the result is the minimum of the two confidences.
    pub fn intersection(&self, other: &UncertaintyInterval) -> Option<UncertaintyInterval> {
        if !self.overlaps(other) {
            return None;
        }

        let earliest = self.earliest.max(other.earliest);
        let latest = self.latest.min(other.latest);
        let confidence = self.confidence.min(other.confidence);

        Some(UncertaintyInterval {
            earliest,
            latest,
            confidence,
        })
    }

    /// Returns the union (convex hull) of two intervals.
    ///
    /// The confidence of the result is the minimum of the two confidences
    /// if they overlap, or lower if they don't (since we're extrapolating).
    pub fn union(&self, other: &UncertaintyInterval) -> UncertaintyInterval {
        let earliest = self.earliest.min(other.earliest);
        let latest = self.latest.max(other.latest);

        // If intervals don't overlap, we're less confident
        let confidence = if self.overlaps(other) {
            self.confidence.min(other.confidence)
        } else {
            self.confidence.min(other.confidence) * 0.5
        };

        UncertaintyInterval {
            earliest,
            latest,
            confidence,
        }
    }

    /// Expands the interval by a given amount on both sides.
    pub fn expand(&self, nanos: u128) -> UncertaintyInterval {
        UncertaintyInterval {
            earliest: self.earliest.saturating_sub(nanos),
            latest: self.latest.saturating_add(nanos),
            confidence: self.confidence,
        }
    }

    /// Shrinks the interval by a given amount on both sides.
    ///
    /// Returns `None` if shrinking would make earliest > latest.
    pub fn shrink(&self, nanos: u128) -> Option<UncertaintyInterval> {
        let width = self.width_nanos();
        if nanos * 2 > width {
            return None;
        }

        Some(UncertaintyInterval {
            earliest: self.earliest + nanos,
            latest: self.latest - nanos,
            confidence: self.confidence,
        })
    }

    /// Returns a new interval with adjusted confidence.
    #[inline]
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&confidence),
            "confidence must be in [0.0, 1.0]"
        );
        self.confidence = confidence;
        self
    }

    /// Compares the temporal relationship between two intervals.
    ///
    /// Returns:
    /// - `Some(Ordering::Less)` if this must precede other
    /// - `Some(Ordering::Greater)` if this must follow other
    /// - `None` if the intervals overlap (relationship is uncertain)
    pub fn temporal_order(&self, other: &UncertaintyInterval) -> Option<std::cmp::Ordering> {
        if self.must_precede(other) {
            Some(std::cmp::Ordering::Less)
        } else if self.must_follow(other) {
            Some(std::cmp::Ordering::Greater)
        } else {
            None
        }
    }
}

impl Default for UncertaintyInterval {
    fn default() -> Self {
        Self::exact(ControllerTime::EPOCH)
    }
}

impl fmt::Display for UncertaintyInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_exact() {
            write!(f, "[{}]", self.earliest)
        } else {
            write!(
                f,
                "[{}, {}] ({}% confidence)",
                self.earliest,
                self.latest,
                (self.confidence * 100.0) as u32
            )
        }
    }
}

impl Eq for UncertaintyInterval {}

impl PartialOrd for UncertaintyInterval {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.temporal_order(other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ct(nanos: u128) -> ControllerTime {
        ControllerTime::from_nanos(nanos)
    }

    #[test]
    fn test_basic_creation() {
        let interval = UncertaintyInterval::new(ct(100), ct(200), 0.95);

        assert_eq!(interval.earliest(), ct(100));
        assert_eq!(interval.latest(), ct(200));
        assert_eq!(interval.confidence(), 0.95);
    }

    #[test]
    fn test_symmetric_creation() {
        let interval = UncertaintyInterval::symmetric(ct(1000), 100, 0.99);

        assert_eq!(interval.earliest(), ct(900));
        assert_eq!(interval.latest(), ct(1100));
    }

    #[test]
    fn test_exact() {
        let interval = UncertaintyInterval::exact(ct(500));

        assert!(interval.is_exact());
        assert_eq!(interval.width_nanos(), 0);
        assert_eq!(interval.confidence(), 1.0);
    }

    #[test]
    fn test_width_and_midpoint() {
        let interval = UncertaintyInterval::new(ct(100), ct(300), 0.9);

        assert_eq!(interval.width_nanos(), 200);
        assert_eq!(interval.midpoint(), ct(200));
    }

    #[test]
    fn test_contains() {
        let interval = UncertaintyInterval::new(ct(100), ct(200), 0.9);

        assert!(interval.contains(ct(100)));
        assert!(interval.contains(ct(150)));
        assert!(interval.contains(ct(200)));
        assert!(!interval.contains(ct(99)));
        assert!(!interval.contains(ct(201)));
    }

    #[test]
    fn test_overlaps() {
        let a = UncertaintyInterval::new(ct(100), ct(200), 0.9);
        let b = UncertaintyInterval::new(ct(150), ct(250), 0.9);
        let c = UncertaintyInterval::new(ct(300), ct(400), 0.9);

        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
        assert!(!a.overlaps(&c));
        assert!(!c.overlaps(&a));
    }

    #[test]
    fn test_must_precede() {
        let a = UncertaintyInterval::new(ct(100), ct(200), 0.9);
        let b = UncertaintyInterval::new(ct(201), ct(300), 0.9);
        let c = UncertaintyInterval::new(ct(150), ct(250), 0.9);

        assert!(a.must_precede(&b));
        assert!(!b.must_precede(&a));
        assert!(!a.must_precede(&c)); // They overlap
    }

    #[test]
    fn test_must_follow() {
        let a = UncertaintyInterval::new(ct(100), ct(200), 0.9);
        let b = UncertaintyInterval::new(ct(201), ct(300), 0.9);

        assert!(b.must_follow(&a));
        assert!(!a.must_follow(&b));
    }

    #[test]
    fn test_intersection() {
        let a = UncertaintyInterval::new(ct(100), ct(200), 0.95);
        let b = UncertaintyInterval::new(ct(150), ct(250), 0.90);

        let intersection = a.intersection(&b).unwrap();
        assert_eq!(intersection.earliest(), ct(150));
        assert_eq!(intersection.latest(), ct(200));
        assert_eq!(intersection.confidence(), 0.90);
    }

    #[test]
    fn test_intersection_none() {
        let a = UncertaintyInterval::new(ct(100), ct(200), 0.9);
        let b = UncertaintyInterval::new(ct(300), ct(400), 0.9);

        assert!(a.intersection(&b).is_none());
    }

    #[test]
    fn test_union() {
        let a = UncertaintyInterval::new(ct(100), ct(200), 0.95);
        let b = UncertaintyInterval::new(ct(150), ct(250), 0.90);

        let union = a.union(&b);
        assert_eq!(union.earliest(), ct(100));
        assert_eq!(union.latest(), ct(250));
        assert_eq!(union.confidence(), 0.90);
    }

    #[test]
    fn test_union_disjoint() {
        let a = UncertaintyInterval::new(ct(100), ct(200), 0.90);
        let b = UncertaintyInterval::new(ct(300), ct(400), 0.80);

        let union = a.union(&b);
        assert_eq!(union.earliest(), ct(100));
        assert_eq!(union.latest(), ct(400));
        // Confidence is halved for disjoint intervals
        assert_eq!(union.confidence(), 0.40);
    }

    #[test]
    fn test_expand() {
        let interval = UncertaintyInterval::new(ct(100), ct(200), 0.9);
        let expanded = interval.expand(50);

        assert_eq!(expanded.earliest(), ct(50));
        assert_eq!(expanded.latest(), ct(250));
    }

    #[test]
    fn test_shrink() {
        let interval = UncertaintyInterval::new(ct(100), ct(200), 0.9);

        let shrunk = interval.shrink(20).unwrap();
        assert_eq!(shrunk.earliest(), ct(120));
        assert_eq!(shrunk.latest(), ct(180));

        // Can't shrink too much
        assert!(interval.shrink(60).is_none());
    }

    #[test]
    fn test_temporal_order() {
        let a = UncertaintyInterval::new(ct(100), ct(200), 0.9);
        let b = UncertaintyInterval::new(ct(300), ct(400), 0.9);
        let c = UncertaintyInterval::new(ct(150), ct(250), 0.9);

        assert_eq!(a.temporal_order(&b), Some(std::cmp::Ordering::Less));
        assert_eq!(b.temporal_order(&a), Some(std::cmp::Ordering::Greater));
        assert_eq!(a.temporal_order(&c), None); // Overlapping
    }

    #[test]
    fn test_display() {
        let exact = UncertaintyInterval::exact(ct(1_000_000_000));
        assert!(format!("{}", exact).contains("1."));

        let interval = UncertaintyInterval::new(ct(1_000_000_000), ct(2_000_000_000), 0.95);
        let display = format!("{}", interval);
        assert!(display.contains("95% confidence"));
    }

    #[test]
    fn test_serde() {
        let interval = UncertaintyInterval::new(ct(100), ct(200), 0.95);
        let json = serde_json::to_string(&interval).unwrap();
        let interval2: UncertaintyInterval = serde_json::from_str(&json).unwrap();

        assert_eq!(interval, interval2);
    }

    #[test]
    #[should_panic(expected = "earliest")]
    fn test_invalid_bounds_panics() {
        UncertaintyInterval::new(ct(200), ct(100), 0.9);
    }

    #[test]
    #[should_panic(expected = "confidence")]
    fn test_invalid_confidence_panics() {
        UncertaintyInterval::new(ct(100), ct(200), 1.5);
    }
}
