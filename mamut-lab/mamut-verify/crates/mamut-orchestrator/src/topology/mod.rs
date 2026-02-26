//! Topology definition types for cluster configuration.
//!
//! This module provides types for defining the topology of distributed systems
//! under test, including cluster layouts, node specifications, and network
//! configurations.

mod cluster;
mod network;

pub use cluster::{ClusterTopology, DeployedCluster, DeploymentState, NodeRole, NodeSpec};
pub use network::{NetworkConfig, NetworkMode, NetworkSpec, PortMapping};
