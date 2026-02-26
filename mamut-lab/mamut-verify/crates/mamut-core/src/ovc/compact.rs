//! Compact OVC - Storage-efficient serialization format.
//!
//! `OVCCompact` provides a space-efficient representation of OVC timestamps
//! for storage in databases, logs, and network transmission. It sacrifices
//! some precision for significant space savings.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::controller_time::ControllerTime;
use super::fault_context::{TimeFaultContext, TimeFaultType};
use super::observed_time::{NodeId, ObservedTime};
use super::uncertainty::UncertaintyInterval;
use super::vector_clock::VectorClock;
use super::OVC;

/// Compact representation of a vector clock.
///
/// Uses a sorted array of (node_id, value) pairs instead of a HashMap
/// for more efficient serialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactVectorClock {
    /// Sorted array of (node_id, clock_value) pairs.
    entries: Vec<(u64, u64)>,
}

impl CompactVectorClock {
    /// Creates a new empty compact vector clock.
    #[inline]
    pub fn new() -> Self {
        CompactVectorClock {
            entries: Vec::new(),
        }
    }

    /// Creates from a VectorClock.
    pub fn from_vector_clock(vc: &VectorClock) -> Self {
        let mut entries: Vec<(u64, u64)> = vc.iter().map(|(n, &v)| (n.as_u64(), v)).collect();
        entries.sort_by_key(|(n, _)| *n);
        CompactVectorClock { entries }
    }

    /// Converts back to a VectorClock.
    pub fn to_vector_clock(&self) -> VectorClock {
        VectorClock::from_iter(self.entries.iter().map(|&(n, v)| (NodeId::new(n), v)))
    }

    /// Returns the number of entries.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the entries as a slice.
    #[inline]
    pub fn entries(&self) -> &[(u64, u64)] {
        &self.entries
    }
}

impl Default for CompactVectorClock {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&VectorClock> for CompactVectorClock {
    fn from(vc: &VectorClock) -> Self {
        Self::from_vector_clock(vc)
    }
}

impl From<CompactVectorClock> for VectorClock {
    fn from(cvc: CompactVectorClock) -> Self {
        cvc.to_vector_clock()
    }
}

/// Storage-efficient representation of an OVC timestamp.
///
/// This compact format uses:
/// - `u64` instead of `u128` for controller time (microsecond precision)
/// - `u64` for observed time (microsecond precision)
/// - Compact vector clock representation
/// - Tuple for uncertainty interval (microsecond offsets from controller time)
/// - Optional u32 for fault type only
///
/// # Space Savings
///
/// Full OVC: ~200+ bytes (with HashMap overhead)
/// Compact OVC: ~50-100 bytes depending on vector clock size
///
/// # Precision Loss
///
/// - Controller time: Reduced from nanoseconds to microseconds
/// - Observed time: Reduced from nanoseconds to microseconds
/// - Uncertainty bounds: Stored as signed microsecond offsets
/// - Fault context: Only fault type preserved (other fields lost)
///
/// # Example
///
/// ```
/// use mamut_core::ovc::{OVC, OVCCompact, ControllerTime, ObservedTime, NodeId};
///
/// let ovc = OVC::new(
///     ControllerTime::from_micros(1_000_000),
///     ObservedTime::new(1_000_000_000, NodeId::new(1)),
/// );
///
/// let compact = OVCCompact::from_ovc(&ovc);
/// let restored = compact.to_ovc();
///
/// // Microsecond precision is preserved
/// assert_eq!(
///     ovc.controller_time().as_micros(),
///     restored.controller_time().as_micros()
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OVCCompact {
    /// Controller time in microseconds.
    ct: u64,

    /// Observed time in microseconds (can be negative, stored as i64).
    ot: i64,

    /// Node ID for the observed time.
    node: u64,

    /// Compact vector clock.
    lt: CompactVectorClock,

    /// Uncertainty interval as (earliest_offset_us, latest_offset_us) from ct.
    /// Signed to allow bounds before ct.
    u: (i64, i64),

    /// Confidence level (stored as u16 representing 0-10000 for 0.0000-1.0000).
    conf: u16,

    /// Fault context type (None if no fault).
    fc: Option<u32>,
}

impl OVCCompact {
    /// Creates a compact OVC from a full OVC.
    pub fn from_ovc(ovc: &OVC) -> Self {
        let ct_micros = ovc.controller_time().as_micros() as u64;
        let ct_nanos = ovc.controller_time().as_nanos() as i128;

        // Calculate uncertainty bounds as offsets from controller time
        let earliest_offset = (ovc.uncertainty().earliest().as_nanos() as i128 - ct_nanos) / 1000;
        let latest_offset = (ovc.uncertainty().latest().as_nanos() as i128 - ct_nanos) / 1000;

        OVCCompact {
            ct: ct_micros,
            ot: (ovc.observed_time().wall_clock_nanos() / 1000) as i64,
            node: ovc.observed_time().node_id().as_u64(),
            lt: CompactVectorClock::from_vector_clock(ovc.logical_time()),
            u: (earliest_offset as i64, latest_offset as i64),
            conf: (ovc.uncertainty().confidence() * 10000.0) as u16,
            fc: ovc.fault_context().map(|fc| fc.to_compact()),
        }
    }

    /// Converts back to a full OVC.
    ///
    /// Note: Some precision and information is lost in the round-trip:
    /// - Nanosecond precision reduced to microseconds
    /// - Fault context details (except type) are lost
    /// - Observed time offset estimate is lost
    pub fn to_ovc(&self) -> OVC {
        let controller_time = ControllerTime::from_micros(self.ct as u128);
        let observed_time =
            ObservedTime::new(self.ot as i128 * 1000, NodeId::new(self.node));
        let logical_time = self.lt.to_vector_clock();

        // Reconstruct uncertainty interval
        let ct_micros = self.ct as i128;
        let earliest_micros = ct_micros + self.u.0 as i128;
        let latest_micros = ct_micros + self.u.1 as i128;
        let confidence = self.conf as f64 / 10000.0;

        let uncertainty = UncertaintyInterval::new(
            ControllerTime::from_micros(earliest_micros.max(0) as u128),
            ControllerTime::from_micros(latest_micros.max(0) as u128),
            confidence.clamp(0.0, 1.0),
        );

        // Reconstruct fault context if present
        let fault_context = self.fc.map(|fc_type| {
            TimeFaultContext::new(
                TimeFaultType::from_compact(fc_type),
                NodeId::new(self.node),
                controller_time,
            )
        });

        OVC {
            controller_time,
            observed_time,
            logical_time,
            uncertainty,
            fault_context,
        }
    }

    /// Returns the controller time in microseconds.
    #[inline]
    pub const fn controller_time_micros(&self) -> u64 {
        self.ct
    }

    /// Returns the observed time in microseconds.
    #[inline]
    pub const fn observed_time_micros(&self) -> i64 {
        self.ot
    }

    /// Returns the node ID.
    #[inline]
    pub const fn node_id(&self) -> u64 {
        self.node
    }

    /// Returns the compact vector clock.
    #[inline]
    pub fn logical_time(&self) -> &CompactVectorClock {
        &self.lt
    }

    /// Returns the uncertainty bounds as (earliest_offset, latest_offset) in microseconds.
    #[inline]
    pub const fn uncertainty_offsets(&self) -> (i64, i64) {
        self.u
    }

    /// Returns the confidence as a value 0-10000.
    #[inline]
    pub const fn confidence_scaled(&self) -> u16 {
        self.conf
    }

    /// Returns the fault type if present.
    #[inline]
    pub const fn fault_type(&self) -> Option<u32> {
        self.fc
    }

    /// Estimates the serialized size in bytes.
    pub fn estimated_size(&self) -> usize {
        // ct: 8, ot: 8, node: 8, conf: 2, u: 16, fc: 4
        // lt: 8 per entry + some overhead
        46 + self.lt.len() * 16
    }
}

impl From<&OVC> for OVCCompact {
    fn from(ovc: &OVC) -> Self {
        Self::from_ovc(ovc)
    }
}

impl From<OVCCompact> for OVC {
    fn from(compact: OVCCompact) -> Self {
        compact.to_ovc()
    }
}

/// Batch of compact OVC records for efficient bulk serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OVCBatch {
    /// The compact OVC records.
    records: Vec<OVCCompact>,

    /// Optional metadata about the batch.
    metadata: Option<HashMap<String, String>>,
}

impl OVCBatch {
    /// Creates a new empty batch.
    pub fn new() -> Self {
        OVCBatch {
            records: Vec::new(),
            metadata: None,
        }
    }

    /// Creates a batch with preallocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        OVCBatch {
            records: Vec::with_capacity(capacity),
            metadata: None,
        }
    }

    /// Creates a batch from OVC records.
    pub fn from_ovcs(ovcs: impl IntoIterator<Item = OVC>) -> Self {
        OVCBatch {
            records: ovcs.into_iter().map(|o| OVCCompact::from_ovc(&o)).collect(),
            metadata: None,
        }
    }

    /// Adds a compact record to the batch.
    pub fn push(&mut self, record: OVCCompact) {
        self.records.push(record);
    }

    /// Adds an OVC to the batch (converting to compact form).
    pub fn push_ovc(&mut self, ovc: &OVC) {
        self.records.push(OVCCompact::from_ovc(ovc));
    }

    /// Returns the number of records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns true if empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Returns the records.
    pub fn records(&self) -> &[OVCCompact] {
        &self.records
    }

    /// Converts all records to full OVCs.
    pub fn to_ovcs(&self) -> Vec<OVC> {
        self.records.iter().map(|r| r.to_ovc()).collect()
    }

    /// Sets metadata for the batch.
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
    }

    /// Gets metadata value.
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.as_ref()?.get(key).map(|s| s.as_str())
    }

    /// Estimates total serialized size.
    pub fn estimated_size(&self) -> usize {
        self.records.iter().map(|r| r.estimated_size()).sum::<usize>()
            + self
                .metadata
                .as_ref()
                .map(|m| m.iter().map(|(k, v)| k.len() + v.len() + 16).sum())
                .unwrap_or(0)
    }
}

impl Default for OVCBatch {
    fn default() -> Self {
        Self::new()
    }
}

impl FromIterator<OVCCompact> for OVCBatch {
    fn from_iter<T: IntoIterator<Item = OVCCompact>>(iter: T) -> Self {
        OVCBatch {
            records: iter.into_iter().collect(),
            metadata: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ovc() -> OVC {
        OVC::new(
            ControllerTime::from_micros(1_000_000),
            ObservedTime::new(1_000_500_000_000, NodeId::new(1)),
        )
        .with_logical_time(VectorClock::from([(NodeId::new(1), 5), (NodeId::new(2), 3)]))
        .with_uncertainty(UncertaintyInterval::new(
            ControllerTime::from_micros(999_000),
            ControllerTime::from_micros(1_001_000),
            0.95,
        ))
    }

    #[test]
    fn test_compact_vector_clock() {
        let vc = VectorClock::from([(NodeId::new(1), 5), (NodeId::new(3), 10), (NodeId::new(2), 7)]);

        let compact = CompactVectorClock::from_vector_clock(&vc);

        // Should be sorted by node ID
        assert_eq!(compact.entries(), &[(1, 5), (2, 7), (3, 10)]);

        // Round-trip
        let restored = compact.to_vector_clock();
        assert_eq!(vc, restored);
    }

    #[test]
    fn test_compact_roundtrip() {
        let ovc = sample_ovc();
        let compact = OVCCompact::from_ovc(&ovc);
        let restored = compact.to_ovc();

        // Microsecond precision should be preserved
        assert_eq!(
            ovc.controller_time().as_micros(),
            restored.controller_time().as_micros()
        );

        // Vector clock should be preserved exactly
        assert_eq!(ovc.logical_time(), restored.logical_time());

        // Uncertainty bounds (at microsecond precision)
        assert_eq!(
            ovc.uncertainty().earliest().as_micros(),
            restored.uncertainty().earliest().as_micros()
        );
        assert_eq!(
            ovc.uncertainty().latest().as_micros(),
            restored.uncertainty().latest().as_micros()
        );
    }

    #[test]
    fn test_compact_with_fault() {
        let ovc = sample_ovc().with_fault_context(TimeFaultContext::new(
            TimeFaultType::ClockSkewBackward,
            NodeId::new(1),
            ControllerTime::from_micros(1_000_000),
        ));

        let compact = OVCCompact::from_ovc(&ovc);
        assert_eq!(compact.fault_type(), Some(1)); // ClockSkewBackward = 1

        let restored = compact.to_ovc();
        assert!(restored.fault_context().is_some());
        assert_eq!(
            restored.fault_context().unwrap().fault_type(),
            TimeFaultType::ClockSkewBackward
        );
    }

    #[test]
    fn test_compact_size() {
        let ovc = sample_ovc();
        let compact = OVCCompact::from_ovc(&ovc);

        // With 2 vector clock entries
        let size = compact.estimated_size();
        assert!(size < 100, "Compact size should be < 100 bytes, got {}", size);
    }

    #[test]
    fn test_batch() {
        let ovcs = vec![sample_ovc(), sample_ovc(), sample_ovc()];

        let mut batch = OVCBatch::from_ovcs(ovcs.clone());
        batch.set_metadata("test_id", "12345");

        assert_eq!(batch.len(), 3);
        assert_eq!(batch.get_metadata("test_id"), Some("12345"));

        let restored = batch.to_ovcs();
        assert_eq!(restored.len(), 3);
    }

    #[test]
    fn test_serde() {
        let ovc = sample_ovc();
        let compact = OVCCompact::from_ovc(&ovc);

        let json = serde_json::to_string(&compact).unwrap();
        let restored: OVCCompact = serde_json::from_str(&json).unwrap();

        assert_eq!(compact, restored);
    }

    #[test]
    fn test_batch_serde() {
        let mut batch = OVCBatch::with_capacity(2);
        batch.push_ovc(&sample_ovc());
        batch.push_ovc(&sample_ovc());
        batch.set_metadata("version", "1.0");

        let json = serde_json::to_string(&batch).unwrap();
        let restored: OVCBatch = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.len(), 2);
        assert_eq!(restored.get_metadata("version"), Some("1.0"));
    }

    #[test]
    fn test_negative_observed_time() {
        // Test handling of times before Unix epoch
        let ovc = OVC::new(
            ControllerTime::from_micros(1_000_000),
            ObservedTime::new(-1_000_000_000, NodeId::new(1)), // Negative time
        );

        let compact = OVCCompact::from_ovc(&ovc);
        assert!(compact.observed_time_micros() < 0);

        let restored = compact.to_ovc();
        assert!(restored.observed_time().wall_clock_nanos() < 0);
    }
}
