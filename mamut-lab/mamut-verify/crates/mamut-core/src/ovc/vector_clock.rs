//! Vector Clock - Logical time for causality tracking.
//!
//! `VectorClock` implements a vector clock for tracking causal relationships
//! between events in a distributed system. Each node maintains its own counter,
//! and the clock captures happens-before relationships through merging.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;

use super::observed_time::NodeId;

/// A vector clock for tracking causality in distributed systems.
///
/// A vector clock is a mechanism for tracking logical time across multiple
/// nodes. Each node has its own counter, and the clock as a whole captures
/// the causal history of events.
///
/// # Causality Relationships
///
/// Given two events A and B:
/// - A **happens-before** B if A's clock is strictly less than B's clock
/// - A and B are **concurrent** if neither happens-before the other
/// - A **might-precede** B if A happens-before B or they are concurrent
///
/// # Example
///
/// ```
/// use mamut_core::ovc::{VectorClock, NodeId};
///
/// let mut clock_a = VectorClock::new();
/// let mut clock_b = VectorClock::new();
///
/// let node_a = NodeId::new(1);
/// let node_b = NodeId::new(2);
///
/// // Node A does some work
/// clock_a.increment(node_a);
///
/// // Node B receives a message from A (simulated by merge)
/// clock_b.merge(&clock_a);
/// clock_b.increment(node_b);
///
/// // Now A's original event happens-before B's event
/// assert!(clock_a.happens_before(&clock_b));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct VectorClock {
    clocks: HashMap<NodeId, u64>,
}

impl VectorClock {
    /// Creates a new empty vector clock.
    #[inline]
    pub fn new() -> Self {
        VectorClock {
            clocks: HashMap::new(),
        }
    }

    /// Creates a vector clock with a single node initialized.
    #[inline]
    pub fn with_node(node_id: NodeId, value: u64) -> Self {
        let mut clocks = HashMap::new();
        clocks.insert(node_id, value);
        VectorClock { clocks }
    }

    /// Creates a vector clock from an iterator of (NodeId, u64) pairs.
    pub fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (NodeId, u64)>,
    {
        VectorClock {
            clocks: iter.into_iter().collect(),
        }
    }

    /// Returns the clock value for a specific node.
    ///
    /// Returns 0 if the node has not been seen.
    #[inline]
    pub fn get(&self, node_id: NodeId) -> u64 {
        self.clocks.get(&node_id).copied().unwrap_or(0)
    }

    /// Sets the clock value for a specific node.
    #[inline]
    pub fn set(&mut self, node_id: NodeId, value: u64) {
        if value == 0 {
            self.clocks.remove(&node_id);
        } else {
            self.clocks.insert(node_id, value);
        }
    }

    /// Increments the clock for a specific node and returns the new value.
    #[inline]
    pub fn increment(&mut self, node_id: NodeId) -> u64 {
        let counter = self.clocks.entry(node_id).or_insert(0);
        *counter = counter.saturating_add(1);
        *counter
    }

    /// Returns true if this clock happens-before another clock.
    ///
    /// Clock A happens-before clock B if:
    /// - For all nodes, A[i] <= B[i]
    /// - For at least one node, A[i] < B[i]
    pub fn happens_before(&self, other: &VectorClock) -> bool {
        let mut dominated = false;

        // Check all entries in self
        for (&node_id, &value) in &self.clocks {
            let other_value = other.get(node_id);
            if value > other_value {
                return false;
            }
            if value < other_value {
                dominated = true;
            }
        }

        // Check entries in other that aren't in self
        for (&node_id, &value) in &other.clocks {
            if !self.clocks.contains_key(&node_id) && value > 0 {
                dominated = true;
            }
        }

        dominated
    }

    /// Returns true if this clock and another are concurrent.
    ///
    /// Two clocks are concurrent if neither happens-before the other.
    #[inline]
    pub fn concurrent_with(&self, other: &VectorClock) -> bool {
        !self.happens_before(other) && !other.happens_before(self)
    }

    /// Merges another clock into this one (component-wise maximum).
    ///
    /// This is typically called when receiving a message from another node.
    /// The resulting clock captures the combined causal history.
    pub fn merge(&mut self, other: &VectorClock) {
        for (&node_id, &value) in &other.clocks {
            let current = self.clocks.entry(node_id).or_insert(0);
            *current = (*current).max(value);
        }
    }

    /// Returns a new clock that is the merge of this clock and another.
    #[inline]
    pub fn merged(&self, other: &VectorClock) -> VectorClock {
        let mut result = self.clone();
        result.merge(other);
        result
    }

    /// Returns the number of nodes tracked by this clock.
    #[inline]
    pub fn len(&self) -> usize {
        self.clocks.len()
    }

    /// Returns true if this clock has no entries.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.clocks.is_empty()
    }

    /// Returns an iterator over all (NodeId, value) pairs.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&NodeId, &u64)> {
        self.clocks.iter()
    }

    /// Returns the set of node IDs tracked by this clock.
    #[inline]
    pub fn nodes(&self) -> impl Iterator<Item = &NodeId> {
        self.clocks.keys()
    }

    /// Returns the sum of all clock values (useful for total ordering).
    #[inline]
    pub fn sum(&self) -> u64 {
        self.clocks.values().sum()
    }

    /// Returns the maximum clock value across all nodes.
    #[inline]
    pub fn max_value(&self) -> u64 {
        self.clocks.values().copied().max().unwrap_or(0)
    }

    /// Clears all clock entries.
    #[inline]
    pub fn clear(&mut self) {
        self.clocks.clear();
    }

    /// Compares two clocks and returns their relationship.
    ///
    /// Returns:
    /// - `Some(Ordering::Less)` if self happens-before other
    /// - `Some(Ordering::Greater)` if other happens-before self
    /// - `Some(Ordering::Equal)` if the clocks are equal
    /// - `None` if the clocks are concurrent
    pub fn compare(&self, other: &VectorClock) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else if self.happens_before(other) {
            Some(Ordering::Less)
        } else if other.happens_before(self) {
            Some(Ordering::Greater)
        } else {
            None // Concurrent
        }
    }
}

impl PartialOrd for VectorClock {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.compare(other)
    }
}

impl fmt::Display for VectorClock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VC{{")?;
        let mut first = true;
        // Sort by node ID for consistent output
        let mut entries: Vec<_> = self.clocks.iter().collect();
        entries.sort_by_key(|(k, _)| k.0);

        for (node_id, value) in entries {
            if !first {
                write!(f, ", ")?;
            }
            write!(f, "{}:{}", node_id.0, value)?;
            first = false;
        }
        write!(f, "}}")
    }
}

impl<const N: usize> From<[(NodeId, u64); N]> for VectorClock {
    fn from(arr: [(NodeId, u64); N]) -> Self {
        VectorClock::from_iter(arr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: u64) -> NodeId {
        NodeId::new(id)
    }

    #[test]
    fn test_new_clock() {
        let clock = VectorClock::new();
        assert!(clock.is_empty());
        assert_eq!(clock.get(node(1)), 0);
    }

    #[test]
    fn test_increment() {
        let mut clock = VectorClock::new();

        assert_eq!(clock.increment(node(1)), 1);
        assert_eq!(clock.increment(node(1)), 2);
        assert_eq!(clock.increment(node(2)), 1);

        assert_eq!(clock.get(node(1)), 2);
        assert_eq!(clock.get(node(2)), 1);
    }

    #[test]
    fn test_happens_before_simple() {
        let clock_a = VectorClock::from([(node(1), 1)]);
        let clock_b = VectorClock::from([(node(1), 2)]);

        assert!(clock_a.happens_before(&clock_b));
        assert!(!clock_b.happens_before(&clock_a));
    }

    #[test]
    fn test_happens_before_multi_node() {
        let clock_a = VectorClock::from([(node(1), 2), (node(2), 1)]);
        let clock_b = VectorClock::from([(node(1), 2), (node(2), 2)]);

        assert!(clock_a.happens_before(&clock_b));
        assert!(!clock_b.happens_before(&clock_a));
    }

    #[test]
    fn test_happens_before_equal() {
        let clock_a = VectorClock::from([(node(1), 2)]);
        let clock_b = VectorClock::from([(node(1), 2)]);

        assert!(!clock_a.happens_before(&clock_b));
        assert!(!clock_b.happens_before(&clock_a));
    }

    #[test]
    fn test_concurrent() {
        // Two clocks that each advanced independently
        let clock_a = VectorClock::from([(node(1), 2), (node(2), 1)]);
        let clock_b = VectorClock::from([(node(1), 1), (node(2), 2)]);

        assert!(!clock_a.happens_before(&clock_b));
        assert!(!clock_b.happens_before(&clock_a));
        assert!(clock_a.concurrent_with(&clock_b));
    }

    #[test]
    fn test_concurrent_disjoint_nodes() {
        let clock_a = VectorClock::from([(node(1), 1)]);
        let clock_b = VectorClock::from([(node(2), 1)]);

        assert!(clock_a.concurrent_with(&clock_b));
    }

    #[test]
    fn test_merge() {
        let mut clock_a = VectorClock::from([(node(1), 2), (node(2), 1)]);
        let clock_b = VectorClock::from([(node(1), 1), (node(2), 3), (node(3), 1)]);

        clock_a.merge(&clock_b);

        assert_eq!(clock_a.get(node(1)), 2); // max(2, 1)
        assert_eq!(clock_a.get(node(2)), 3); // max(1, 3)
        assert_eq!(clock_a.get(node(3)), 1); // max(0, 1)
    }

    #[test]
    fn test_merged() {
        let clock_a = VectorClock::from([(node(1), 2)]);
        let clock_b = VectorClock::from([(node(2), 3)]);

        let merged = clock_a.merged(&clock_b);

        assert_eq!(merged.get(node(1)), 2);
        assert_eq!(merged.get(node(2)), 3);

        // Original clocks unchanged
        assert_eq!(clock_a.get(node(2)), 0);
        assert_eq!(clock_b.get(node(1)), 0);
    }

    #[test]
    fn test_compare() {
        let a = VectorClock::from([(node(1), 1)]);
        let b = VectorClock::from([(node(1), 2)]);
        let c = VectorClock::from([(node(1), 1)]);
        let d = VectorClock::from([(node(2), 1)]);

        assert_eq!(a.compare(&b), Some(Ordering::Less));
        assert_eq!(b.compare(&a), Some(Ordering::Greater));
        assert_eq!(a.compare(&c), Some(Ordering::Equal));
        assert_eq!(a.compare(&d), None); // Concurrent
    }

    #[test]
    fn test_partial_ord() {
        let a = VectorClock::from([(node(1), 1)]);
        let b = VectorClock::from([(node(1), 2)]);

        assert!(a < b);
        assert!(b > a);
    }

    #[test]
    fn test_sum_and_max() {
        let clock = VectorClock::from([(node(1), 5), (node(2), 3), (node(3), 7)]);

        assert_eq!(clock.sum(), 15);
        assert_eq!(clock.max_value(), 7);
    }

    #[test]
    fn test_display() {
        let clock = VectorClock::from([(node(1), 2), (node(2), 3)]);
        let s = format!("{}", clock);

        assert!(s.starts_with("VC{"));
        assert!(s.ends_with("}"));
        assert!(s.contains("1:2"));
        assert!(s.contains("2:3"));
    }

    #[test]
    fn test_serde() {
        let clock = VectorClock::from([(node(1), 5), (node(2), 10)]);
        let json = serde_json::to_string(&clock).unwrap();
        let clock2: VectorClock = serde_json::from_str(&json).unwrap();

        assert_eq!(clock, clock2);
    }

    #[test]
    fn test_happens_before_with_missing_entries() {
        // Clock B has an entry for node 2 that A doesn't have
        let clock_a = VectorClock::from([(node(1), 1)]);
        let clock_b = VectorClock::from([(node(1), 1), (node(2), 1)]);

        assert!(clock_a.happens_before(&clock_b));
        assert!(!clock_b.happens_before(&clock_a));
    }

    #[test]
    fn test_empty_clock_happens_before() {
        let empty = VectorClock::new();
        let non_empty = VectorClock::from([(node(1), 1)]);

        assert!(empty.happens_before(&non_empty));
        assert!(!non_empty.happens_before(&empty));
    }

    #[test]
    fn test_scenario_message_passing() {
        // Simulate: A sends message to B, B processes it
        let node_a = node(1);
        let node_b = node(2);

        // Node A creates an event
        let mut clock_a = VectorClock::new();
        clock_a.increment(node_a);

        // Node A sends message with its clock
        let sent_clock = clock_a.clone();

        // Node B receives message and merges
        let mut clock_b = VectorClock::new();
        clock_b.merge(&sent_clock);
        clock_b.increment(node_b);

        // Node A's send event happens-before Node B's receive event
        assert!(clock_a.happens_before(&clock_b));

        // Node A does more work (independent of B's work)
        clock_a.increment(node_a);

        // Now A and B are concurrent
        assert!(clock_a.concurrent_with(&clock_b));
    }
}
