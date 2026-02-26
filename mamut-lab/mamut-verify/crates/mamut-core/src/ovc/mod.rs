//! Omniscient Verification Clock (OVC) - Multi-dimensional time tracking.
//!
//! The OVC is a composite timestamp that captures multiple perspectives on time
//! in a distributed system under test:
//!
//! - **Controller Time**: Monotonic ground truth from the test controller
//! - **Observed Time**: What nodes in the system believe the time to be
//! - **Logical Time**: Vector clocks for causality tracking
//! - **Uncertainty**: Bounds on when events actually occurred
//! - **Fault Context**: Information about time-related faults
//!
//! This multi-dimensional approach enables verification of distributed systems
//! by correlating different views of time and detecting anomalies.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                         OVC                                 │
//! ├─────────────────────────────────────────────────────────────┤
//! │  ┌──────────────────┐  ┌──────────────────┐                │
//! │  │  ControllerTime  │  │   ObservedTime   │                │
//! │  │  (ground truth)  │  │  (node's view)   │                │
//! │  └──────────────────┘  └──────────────────┘                │
//! │  ┌──────────────────┐  ┌──────────────────┐                │
//! │  │   VectorClock    │  │ UncertaintyInterval│               │
//! │  │   (causality)    │  │    (bounds)      │                │
//! │  └──────────────────┘  └──────────────────┘                │
//! │  ┌──────────────────────────────────────────┐              │
//! │  │         TimeFaultContext (optional)      │              │
//! │  └──────────────────────────────────────────┘              │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```
//! use mamut_core::ovc::{OVC, ControllerTime, ObservedTime, NodeId, VectorClock};
//!
//! // Create an OVC for an event
//! let node = NodeId::new(1);
//! let mut ovc = OVC::new(
//!     ControllerTime::from_millis(1000),
//!     ObservedTime::new(1_000_500_000, node), // Node clock is 0.5ms ahead
//! );
//!
//! // Track causality
//! ovc.logical_time_mut().increment(node);
//!
//! // Check for clock drift
//! let drift = ovc.clock_drift_nanos();
//! assert!(drift.abs() < 1_000_000); // Less than 1ms drift
//! ```

mod compact;
mod controller_time;
mod fault_context;
mod observed_time;
mod uncertainty;
mod vector_clock;

pub use compact::{CompactVectorClock, OVCBatch, OVCCompact};
pub use controller_time::ControllerTime;
pub use fault_context::{FaultSeverity, TimeFaultContext, TimeFaultType};
pub use observed_time::{NodeId, ObservedTime};
pub use uncertainty::UncertaintyInterval;
pub use vector_clock::VectorClock;

use serde::{Deserialize, Serialize};
use std::fmt;

/// Omniscient Verification Clock - A multi-dimensional timestamp.
///
/// The OVC captures multiple perspectives on time for a single event,
/// enabling comprehensive verification of distributed system behavior.
///
/// # Components
///
/// - `controller_time`: The authoritative time from the test controller
/// - `observed_time`: What the node believes the time to be
/// - `logical_time`: Vector clock for happens-before relationships
/// - `uncertainty`: Bounds on the actual event time
/// - `fault_context`: Optional information about time faults
///
/// # Usage Patterns
///
/// ## Event Correlation
///
/// ```
/// use mamut_core::ovc::{OVC, ControllerTime, ObservedTime, NodeId};
///
/// let event_a = OVC::new(
///     ControllerTime::from_millis(100),
///     ObservedTime::new(100_000_000, NodeId::new(1)),
/// );
///
/// let event_b = OVC::new(
///     ControllerTime::from_millis(150),
///     ObservedTime::new(150_000_000, NodeId::new(2)),
/// );
///
/// // Compare using controller time (ground truth)
/// assert!(event_a.controller_time() < event_b.controller_time());
/// ```
///
/// ## Causality Tracking
///
/// ```
/// use mamut_core::ovc::{OVC, ControllerTime, ObservedTime, NodeId, VectorClock};
///
/// let node_a = NodeId::new(1);
/// let node_b = NodeId::new(2);
///
/// let mut send_event = OVC::new(
///     ControllerTime::from_millis(100),
///     ObservedTime::new(100_000_000, node_a),
/// );
/// send_event.logical_time_mut().increment(node_a);
///
/// // Simulate message receive
/// let mut recv_event = OVC::new(
///     ControllerTime::from_millis(110),
///     ObservedTime::new(110_000_000, node_b),
/// );
/// recv_event.logical_time_mut().merge(send_event.logical_time());
/// recv_event.logical_time_mut().increment(node_b);
///
/// // Causality is captured
/// assert!(send_event.happens_before(&recv_event));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OVC {
    /// Monotonic ground truth time from the test controller.
    pub controller_time: ControllerTime,

    /// Time as observed/reported by a node in the system.
    pub observed_time: ObservedTime,

    /// Vector clock for tracking causality.
    pub logical_time: VectorClock,

    /// Uncertainty bounds on when the event actually occurred.
    pub uncertainty: UncertaintyInterval,

    /// Optional context about time-related faults.
    pub fault_context: Option<TimeFaultContext>,
}

impl OVC {
    /// Creates a new OVC with the given controller and observed times.
    ///
    /// The logical time starts empty, and the uncertainty interval is
    /// initialized to be exact (matching controller time).
    pub fn new(controller_time: ControllerTime, observed_time: ObservedTime) -> Self {
        OVC {
            controller_time,
            observed_time,
            logical_time: VectorClock::new(),
            uncertainty: UncertaintyInterval::exact(controller_time),
            fault_context: None,
        }
    }

    /// Creates an OVC with all components specified.
    pub fn full(
        controller_time: ControllerTime,
        observed_time: ObservedTime,
        logical_time: VectorClock,
        uncertainty: UncertaintyInterval,
        fault_context: Option<TimeFaultContext>,
    ) -> Self {
        OVC {
            controller_time,
            observed_time,
            logical_time,
            uncertainty,
            fault_context,
        }
    }

    /// Returns the controller time.
    #[inline]
    pub const fn controller_time(&self) -> ControllerTime {
        self.controller_time
    }

    /// Returns the observed time.
    #[inline]
    pub const fn observed_time(&self) -> ObservedTime {
        self.observed_time
    }

    /// Returns a reference to the logical time (vector clock).
    #[inline]
    pub fn logical_time(&self) -> &VectorClock {
        &self.logical_time
    }

    /// Returns a mutable reference to the logical time.
    #[inline]
    pub fn logical_time_mut(&mut self) -> &mut VectorClock {
        &mut self.logical_time
    }

    /// Returns the uncertainty interval.
    #[inline]
    pub const fn uncertainty(&self) -> &UncertaintyInterval {
        &self.uncertainty
    }

    /// Returns the fault context, if any.
    #[inline]
    pub fn fault_context(&self) -> Option<&TimeFaultContext> {
        self.fault_context.as_ref()
    }

    /// Sets the logical time.
    #[inline]
    pub fn with_logical_time(mut self, logical_time: VectorClock) -> Self {
        self.logical_time = logical_time;
        self
    }

    /// Sets the uncertainty interval.
    #[inline]
    pub fn with_uncertainty(mut self, uncertainty: UncertaintyInterval) -> Self {
        self.uncertainty = uncertainty;
        self
    }

    /// Sets the fault context.
    #[inline]
    pub fn with_fault_context(mut self, fault_context: TimeFaultContext) -> Self {
        self.fault_context = Some(fault_context);
        self
    }

    /// Clears the fault context.
    #[inline]
    pub fn clear_fault_context(&mut self) {
        self.fault_context = None;
    }

    /// Returns true if this event happens-before another based on logical time.
    #[inline]
    pub fn happens_before(&self, other: &OVC) -> bool {
        self.logical_time.happens_before(&other.logical_time)
    }

    /// Returns true if this event is concurrent with another based on logical time.
    #[inline]
    pub fn concurrent_with(&self, other: &OVC) -> bool {
        self.logical_time.concurrent_with(&other.logical_time)
    }

    /// Returns the clock drift in nanoseconds (observed - controller).
    ///
    /// Positive values mean the node's clock is ahead of the controller.
    #[inline]
    pub fn clock_drift_nanos(&self) -> i128 {
        self.observed_time.drift_from(self.controller_time)
    }

    /// Returns true if the uncertainty intervals of two OVCs overlap.
    #[inline]
    pub fn uncertainty_overlaps(&self, other: &OVC) -> bool {
        self.uncertainty.overlaps(&other.uncertainty)
    }

    /// Returns true if this OVC's event must have preceded another's.
    ///
    /// This is true when even the latest possible time for this event
    /// is before the earliest possible time for the other event.
    #[inline]
    pub fn must_precede(&self, other: &OVC) -> bool {
        self.uncertainty.must_precede(&other.uncertainty)
    }

    /// Returns true if a fault has been recorded.
    #[inline]
    pub fn has_fault(&self) -> bool {
        self.fault_context.is_some()
    }

    /// Creates a new OVC that represents a derived event.
    ///
    /// The new OVC:
    /// - Has the given new controller and observed times
    /// - Inherits and increments the logical time
    /// - Has exact uncertainty based on new controller time
    /// - Has no fault context
    pub fn derive(
        &self,
        new_controller_time: ControllerTime,
        new_observed_time: ObservedTime,
    ) -> OVC {
        let mut new_logical = self.logical_time.clone();
        new_logical.increment(new_observed_time.node_id());

        OVC {
            controller_time: new_controller_time,
            observed_time: new_observed_time,
            logical_time: new_logical,
            uncertainty: UncertaintyInterval::exact(new_controller_time),
            fault_context: None,
        }
    }

    /// Merges logical time from another OVC (for message receive).
    pub fn merge_logical_time(&mut self, other: &OVC) {
        self.logical_time.merge(&other.logical_time);
    }

    /// Converts to a compact representation for storage.
    #[inline]
    pub fn to_compact(&self) -> OVCCompact {
        OVCCompact::from_ovc(self)
    }

    /// Creates from a compact representation.
    #[inline]
    pub fn from_compact(compact: OVCCompact) -> Self {
        compact.to_ovc()
    }
}

impl Default for OVC {
    fn default() -> Self {
        OVC::new(
            ControllerTime::EPOCH,
            ObservedTime::new(0, NodeId::new(0)),
        )
    }
}

impl fmt::Display for OVC {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "OVC[ct={}, ot={}, lt={}, u={}",
            self.controller_time, self.observed_time, self.logical_time, self.uncertainty
        )?;

        if let Some(fc) = &self.fault_context {
            write!(f, ", fault={}", fc.fault_type())?;
        }

        write!(f, "]")
    }
}

impl PartialOrd for OVC {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Use logical time for partial ordering (causality-based)
        self.logical_time.partial_cmp(&other.logical_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: u64) -> NodeId {
        NodeId::new(id)
    }

    fn ct(millis: u64) -> ControllerTime {
        ControllerTime::from_millis(millis as u128)
    }

    fn ot(millis: i64, node_id: NodeId) -> ObservedTime {
        ObservedTime::new(millis as i128 * 1_000_000, node_id)
    }

    #[test]
    fn test_basic_creation() {
        let ovc = OVC::new(ct(100), ot(100, node(1)));

        assert_eq!(ovc.controller_time(), ct(100));
        assert_eq!(ovc.observed_time().node_id(), node(1));
        assert!(ovc.logical_time().is_empty());
        assert!(ovc.uncertainty().is_exact());
        assert!(ovc.fault_context().is_none());
    }

    #[test]
    fn test_clock_drift() {
        // Node clock is 5ms ahead
        let ovc = OVC::new(ct(100), ot(105, node(1)));

        assert_eq!(ovc.clock_drift_nanos(), 5_000_000);
    }

    #[test]
    fn test_happens_before() {
        let mut ovc_a = OVC::new(ct(100), ot(100, node(1)));
        ovc_a.logical_time_mut().increment(node(1));

        let mut ovc_b = OVC::new(ct(110), ot(110, node(2)));
        ovc_b.merge_logical_time(&ovc_a);
        ovc_b.logical_time_mut().increment(node(2));

        assert!(ovc_a.happens_before(&ovc_b));
        assert!(!ovc_b.happens_before(&ovc_a));
    }

    #[test]
    fn test_concurrent() {
        let mut ovc_a = OVC::new(ct(100), ot(100, node(1)));
        ovc_a.logical_time_mut().increment(node(1));

        let mut ovc_b = OVC::new(ct(100), ot(100, node(2)));
        ovc_b.logical_time_mut().increment(node(2));

        assert!(ovc_a.concurrent_with(&ovc_b));
    }

    #[test]
    fn test_derive() {
        let mut ovc = OVC::new(ct(100), ot(100, node(1)));
        ovc.logical_time_mut().increment(node(1));

        let derived = ovc.derive(ct(110), ot(110, node(1)));

        assert_eq!(derived.controller_time(), ct(110));
        assert_eq!(derived.logical_time().get(node(1)), 2);
        assert!(ovc.happens_before(&derived));
    }

    #[test]
    fn test_uncertainty_must_precede() {
        let ovc_a = OVC::new(ct(100), ot(100, node(1))).with_uncertainty(
            UncertaintyInterval::new(ct(95), ct(105), 0.99),
        );

        let ovc_b = OVC::new(ct(200), ot(200, node(2))).with_uncertainty(
            UncertaintyInterval::new(ct(195), ct(205), 0.99),
        );

        assert!(ovc_a.must_precede(&ovc_b));
        assert!(!ovc_b.must_precede(&ovc_a));
    }

    #[test]
    fn test_uncertainty_overlaps() {
        let ovc_a = OVC::new(ct(100), ot(100, node(1))).with_uncertainty(
            UncertaintyInterval::new(ct(90), ct(110), 0.99),
        );

        let ovc_b = OVC::new(ct(105), ot(105, node(2))).with_uncertainty(
            UncertaintyInterval::new(ct(100), ct(115), 0.99),
        );

        assert!(ovc_a.uncertainty_overlaps(&ovc_b));
    }

    #[test]
    fn test_with_fault_context() {
        let fault = TimeFaultContext::new(
            TimeFaultType::ClockSkewBackward,
            node(1),
            ct(100),
        );

        let ovc = OVC::new(ct(100), ot(95, node(1))).with_fault_context(fault);

        assert!(ovc.has_fault());
        assert_eq!(
            ovc.fault_context().unwrap().fault_type(),
            TimeFaultType::ClockSkewBackward
        );
    }

    #[test]
    fn test_compact_roundtrip() {
        let ovc = OVC::new(ct(1000), ot(1005, node(1)))
            .with_logical_time(VectorClock::from([(node(1), 5), (node(2), 3)]))
            .with_uncertainty(UncertaintyInterval::new(ct(995), ct(1005), 0.95));

        let compact = ovc.to_compact();
        let restored = OVC::from_compact(compact);

        // Check microsecond-level precision is preserved
        assert_eq!(
            ovc.controller_time().as_micros(),
            restored.controller_time().as_micros()
        );
        assert_eq!(ovc.logical_time(), restored.logical_time());
    }

    #[test]
    fn test_display() {
        let ovc = OVC::new(ct(1000), ot(1000, node(1)));
        let display = format!("{}", ovc);

        assert!(display.starts_with("OVC["));
        assert!(display.contains("ct="));
        assert!(display.contains("ot="));
        assert!(display.contains("lt="));
    }

    #[test]
    fn test_serde() {
        let ovc = OVC::new(ct(1000), ot(1000, node(1)))
            .with_logical_time(VectorClock::from([(node(1), 5)]))
            .with_fault_context(TimeFaultContext::new(
                TimeFaultType::NtpSyncFailure,
                node(1),
                ct(1000),
            ));

        let json = serde_json::to_string(&ovc).unwrap();
        let restored: OVC = serde_json::from_str(&json).unwrap();

        assert_eq!(ovc, restored);
    }

    #[test]
    fn test_partial_ord() {
        let mut ovc_a = OVC::new(ct(100), ot(100, node(1)));
        ovc_a.logical_time_mut().increment(node(1));

        let mut ovc_b = OVC::new(ct(200), ot(200, node(1)));
        ovc_b.logical_time_mut().set(node(1), 5);

        assert!(ovc_a < ovc_b);
    }
}
