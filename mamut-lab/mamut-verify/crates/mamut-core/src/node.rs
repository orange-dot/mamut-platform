//! Node and process identification types.
//!
//! This module provides types for identifying nodes and processes within the
//! distributed system under verification.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for a node in the distributed system.
///
/// Nodes represent individual machines or logical units in the cluster.
/// Each node can host multiple processes.
///
/// # Examples
///
/// ```
/// use mamut_core::node::NodeId;
///
/// let node = NodeId(1);
/// assert_eq!(node.0, 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeId(pub u32);

impl NodeId {
    /// Creates a new NodeId with the given value.
    #[inline]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Returns the inner value of the NodeId.
    #[inline]
    pub const fn inner(self) -> u32 {
        self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Node({})", self.0)
    }
}

impl From<u32> for NodeId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<NodeId> for u32 {
    fn from(id: NodeId) -> Self {
        id.0
    }
}

/// Unique identifier for a process within the distributed system.
///
/// Processes represent individual threads of execution or logical actors.
/// Multiple processes can exist on a single node, and each process
/// executes operations sequentially.
///
/// # Examples
///
/// ```
/// use mamut_core::node::ProcessId;
///
/// let process = ProcessId(42);
/// assert_eq!(process.0, 42);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProcessId(pub u32);

impl ProcessId {
    /// Creates a new ProcessId with the given value.
    #[inline]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Returns the inner value of the ProcessId.
    #[inline]
    pub const fn inner(self) -> u32 {
        self.0
    }
}

impl fmt::Display for ProcessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Process({})", self.0)
    }
}

impl From<u32> for ProcessId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<ProcessId> for u32 {
    fn from(id: ProcessId) -> Self {
        id.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_creation() {
        let node = NodeId::new(5);
        assert_eq!(node.inner(), 5);
        assert_eq!(node.0, 5);
    }

    #[test]
    fn test_node_id_display() {
        let node = NodeId(42);
        assert_eq!(format!("{}", node), "Node(42)");
    }

    #[test]
    fn test_node_id_equality() {
        let a = NodeId(1);
        let b = NodeId(1);
        let c = NodeId(2);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_node_id_ordering() {
        let a = NodeId(1);
        let b = NodeId(2);
        assert!(a < b);
    }

    #[test]
    fn test_node_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(NodeId(1));
        set.insert(NodeId(2));
        set.insert(NodeId(1)); // duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_node_id_conversion() {
        let node: NodeId = 10u32.into();
        assert_eq!(node.0, 10);

        let value: u32 = node.into();
        assert_eq!(value, 10);
    }

    #[test]
    fn test_process_id_creation() {
        let process = ProcessId::new(5);
        assert_eq!(process.inner(), 5);
        assert_eq!(process.0, 5);
    }

    #[test]
    fn test_process_id_display() {
        let process = ProcessId(42);
        assert_eq!(format!("{}", process), "Process(42)");
    }

    #[test]
    fn test_process_id_equality() {
        let a = ProcessId(1);
        let b = ProcessId(1);
        let c = ProcessId(2);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_process_id_ordering() {
        let a = ProcessId(1);
        let b = ProcessId(2);
        assert!(a < b);
    }

    #[test]
    fn test_process_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ProcessId(1));
        set.insert(ProcessId(2));
        set.insert(ProcessId(1)); // duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_process_id_conversion() {
        let process: ProcessId = 10u32.into();
        assert_eq!(process.0, 10);

        let value: u32 = process.into();
        assert_eq!(value, 10);
    }

    #[test]
    fn test_node_id_serialization() {
        let node = NodeId(123);
        let json = serde_json::to_string(&node).unwrap();
        assert_eq!(json, "123");

        let deserialized: NodeId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, node);
    }

    #[test]
    fn test_process_id_serialization() {
        let process = ProcessId(456);
        let json = serde_json::to_string(&process).unwrap();
        assert_eq!(json, "456");

        let deserialized: ProcessId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, process);
    }
}
