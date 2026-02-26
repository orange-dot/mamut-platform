//! Observed Time - Node-reported wall clock time.
//!
//! `ObservedTime` captures what a particular node believes the time to be,
//! along with metadata about the observation including the node's identity
//! and any estimated clock offset from the controller's ground truth.

use serde::{Deserialize, Serialize};
use std::fmt;

use super::controller_time::ControllerTime;

/// Unique identifier for a node in the distributed system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

impl NodeId {
    /// Creates a new node ID.
    #[inline]
    pub const fn new(id: u64) -> Self {
        NodeId(id)
    }

    /// Returns the raw ID value.
    #[inline]
    pub const fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "node:{}", self.0)
    }
}

impl From<u64> for NodeId {
    fn from(id: u64) -> Self {
        NodeId(id)
    }
}

impl From<NodeId> for u64 {
    fn from(id: NodeId) -> Self {
        id.0
    }
}

/// A time observation as reported by a node in the system under test.
///
/// This represents what a node believes the current time to be, which may
/// differ from the controller's ground truth due to clock drift, NTP issues,
/// or intentionally injected faults.
///
/// # Fields
///
/// - `wall_clock_nanos`: The wall clock time as reported by the node (signed
///   to handle times before Unix epoch in testing scenarios)
/// - `node_id`: Which node made this observation
/// - `estimated_offset_nanos`: The estimated offset between this node's clock
///   and the controller's ground truth, if known
///
/// # Example
///
/// ```
/// use mamut_core::ovc::{ObservedTime, NodeId};
///
/// let node = NodeId::new(1);
/// let obs = ObservedTime::new(1_000_000_000, node)
///     .with_estimated_offset(50_000); // 50 microseconds fast
///
/// assert_eq!(obs.node_id(), node);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObservedTime {
    /// Wall clock time in nanoseconds as reported by the node.
    /// Signed to handle edge cases and times before Unix epoch.
    wall_clock_nanos: i128,

    /// The node that made this observation.
    node_id: NodeId,

    /// Estimated offset from controller time in nanoseconds.
    /// Positive means the node's clock is ahead of controller time.
    /// None if the offset is unknown.
    estimated_offset_nanos: Option<i128>,
}

impl ObservedTime {
    /// Creates a new observed time from a node.
    #[inline]
    pub const fn new(wall_clock_nanos: i128, node_id: NodeId) -> Self {
        ObservedTime {
            wall_clock_nanos,
            node_id,
            estimated_offset_nanos: None,
        }
    }

    /// Creates an observed time with a known offset.
    #[inline]
    pub const fn with_offset(wall_clock_nanos: i128, node_id: NodeId, offset_nanos: i128) -> Self {
        ObservedTime {
            wall_clock_nanos,
            node_id,
            estimated_offset_nanos: Some(offset_nanos),
        }
    }

    /// Sets the estimated offset from controller time.
    #[inline]
    pub const fn with_estimated_offset(mut self, offset_nanos: i128) -> Self {
        self.estimated_offset_nanos = Some(offset_nanos);
        self
    }

    /// Returns the raw wall clock time in nanoseconds.
    #[inline]
    pub const fn wall_clock_nanos(&self) -> i128 {
        self.wall_clock_nanos
    }

    /// Returns the node that made this observation.
    #[inline]
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Returns the estimated offset from controller time, if known.
    #[inline]
    pub const fn estimated_offset_nanos(&self) -> Option<i128> {
        self.estimated_offset_nanos
    }

    /// Computes the estimated controller time based on the offset.
    ///
    /// Returns `None` if the offset is unknown or if the computation would
    /// result in a negative controller time.
    pub fn estimated_controller_time(&self) -> Option<ControllerTime> {
        let offset = self.estimated_offset_nanos?;
        let corrected = self.wall_clock_nanos - offset;

        if corrected >= 0 {
            Some(ControllerTime::from_nanos(corrected as u128))
        } else {
            None
        }
    }

    /// Updates the offset estimate.
    #[inline]
    pub fn set_estimated_offset(&mut self, offset_nanos: i128) {
        self.estimated_offset_nanos = Some(offset_nanos);
    }

    /// Clears the offset estimate.
    #[inline]
    pub fn clear_estimated_offset(&mut self) {
        self.estimated_offset_nanos = None;
    }

    /// Computes the difference between the observed time and a controller time.
    ///
    /// A positive result means the observed time is ahead of the controller time.
    #[inline]
    pub fn drift_from(&self, controller_time: ControllerTime) -> i128 {
        self.wall_clock_nanos - controller_time.as_nanos() as i128
    }

    /// Returns whether this observation is from the same node as another.
    #[inline]
    pub fn same_node(&self, other: &ObservedTime) -> bool {
        self.node_id == other.node_id
    }
}

impl fmt::Display for ObservedTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let secs = self.wall_clock_nanos / 1_000_000_000;
        let nanos = (self.wall_clock_nanos % 1_000_000_000).unsigned_abs() as u32;

        if let Some(offset) = self.estimated_offset_nanos {
            write!(
                f,
                "{}.{:09}s@{} (offset: {}ns)",
                secs, nanos, self.node_id, offset
            )
        } else {
            write!(f, "{}.{:09}s@{}", secs, nanos, self.node_id)
        }
    }
}

impl PartialOrd for ObservedTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Only comparable if from the same node
        if self.node_id == other.node_id {
            Some(self.wall_clock_nanos.cmp(&other.wall_clock_nanos))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_creation() {
        let node = NodeId::new(42);
        let obs = ObservedTime::new(1_000_000_000, node);

        assert_eq!(obs.wall_clock_nanos(), 1_000_000_000);
        assert_eq!(obs.node_id(), node);
        assert_eq!(obs.estimated_offset_nanos(), None);
    }

    #[test]
    fn test_with_offset() {
        let node = NodeId::new(1);
        let obs = ObservedTime::with_offset(1_000_000_000, node, 50_000);

        assert_eq!(obs.estimated_offset_nanos(), Some(50_000));
    }

    #[test]
    fn test_estimated_controller_time() {
        let node = NodeId::new(1);

        // Node clock is 1 second ahead
        let obs = ObservedTime::with_offset(2_000_000_000, node, 1_000_000_000);
        let est = obs.estimated_controller_time().unwrap();

        assert_eq!(est.as_nanos(), 1_000_000_000);
    }

    #[test]
    fn test_drift_from() {
        let node = NodeId::new(1);
        let obs = ObservedTime::new(1_000_500_000, node);
        let ct = ControllerTime::from_nanos(1_000_000_000);

        assert_eq!(obs.drift_from(ct), 500_000); // 500 microseconds ahead
    }

    #[test]
    fn test_partial_ord_same_node() {
        let node = NodeId::new(1);
        let obs1 = ObservedTime::new(100, node);
        let obs2 = ObservedTime::new(200, node);

        assert!(obs1 < obs2);
        assert!(obs2 > obs1);
    }

    #[test]
    fn test_partial_ord_different_nodes() {
        let obs1 = ObservedTime::new(100, NodeId::new(1));
        let obs2 = ObservedTime::new(200, NodeId::new(2));

        assert!(obs1.partial_cmp(&obs2).is_none());
    }

    #[test]
    fn test_display() {
        let node = NodeId::new(1);
        let obs = ObservedTime::new(1_234_567_890, node);

        assert!(format!("{}", obs).contains("1.234567890s"));
        assert!(format!("{}", obs).contains("node:1"));
    }

    #[test]
    fn test_serde() {
        let obs = ObservedTime::with_offset(1_000_000_000, NodeId::new(5), 12345);
        let json = serde_json::to_string(&obs).unwrap();
        let obs2: ObservedTime = serde_json::from_str(&json).unwrap();

        assert_eq!(obs, obs2);
    }

    #[test]
    fn test_node_id_display() {
        let node = NodeId::new(42);
        assert_eq!(format!("{}", node), "node:42");
    }
}
