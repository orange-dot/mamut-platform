//! Network topology and configuration types.
//!
//! This module provides types for defining network configurations in
//! distributed system deployments, including network modes, port mappings,
//! and network fault injection configurations.

use mamut_core::node::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;

/// Network mode for container networking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkMode {
    /// Bridge network (default Docker networking).
    Bridge,

    /// Host network (container shares host network namespace).
    Host,

    /// No networking.
    None,

    /// Custom network with the given name.
    Custom(String),
}

impl Default for NetworkMode {
    fn default() -> Self {
        Self::Bridge
    }
}

impl NetworkMode {
    /// Returns the Docker network mode string.
    pub fn as_docker_mode(&self) -> String {
        match self {
            Self::Bridge => "bridge".to_string(),
            Self::Host => "host".to_string(),
            Self::None => "none".to_string(),
            Self::Custom(name) => name.clone(),
        }
    }
}

/// Port mapping configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Container port.
    pub container_port: u16,

    /// Host port (if None, Docker assigns one).
    pub host_port: Option<u16>,

    /// Protocol (tcp/udp).
    pub protocol: PortProtocol,

    /// Host IP to bind to.
    pub host_ip: Option<IpAddr>,
}

impl PortMapping {
    /// Creates a new TCP port mapping.
    pub fn tcp(container_port: u16) -> Self {
        Self {
            container_port,
            host_port: None,
            protocol: PortProtocol::Tcp,
            host_ip: None,
        }
    }

    /// Creates a new UDP port mapping.
    pub fn udp(container_port: u16) -> Self {
        Self {
            container_port,
            host_port: None,
            protocol: PortProtocol::Udp,
            host_ip: None,
        }
    }

    /// Sets the host port.
    pub fn host_port(mut self, port: u16) -> Self {
        self.host_port = Some(port);
        self
    }

    /// Sets the host IP to bind to.
    pub fn host_ip(mut self, ip: IpAddr) -> Self {
        self.host_ip = Some(ip);
        self
    }

    /// Returns the port mapping string for Docker.
    pub fn as_docker_port(&self) -> String {
        let proto = match self.protocol {
            PortProtocol::Tcp => "tcp",
            PortProtocol::Udp => "udp",
        };

        match (self.host_ip, self.host_port) {
            (Some(ip), Some(hp)) => format!("{}:{}:{}/{}", ip, hp, self.container_port, proto),
            (None, Some(hp)) => format!("{}:{}/{}", hp, self.container_port, proto),
            (Some(ip), None) => format!("{}::{}/{}", ip, self.container_port, proto),
            (None, None) => format!("{}/{}", self.container_port, proto),
        }
    }
}

/// Port protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortProtocol {
    /// TCP protocol.
    Tcp,
    /// UDP protocol.
    Udp,
}

impl Default for PortProtocol {
    fn default() -> Self {
        Self::Tcp
    }
}

/// Network specification for a cluster.
///
/// Defines the network configuration including mode, subnet, and
/// driver options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSpec {
    /// Name of the network.
    pub name: String,

    /// Network driver (bridge, overlay, etc.).
    pub driver: String,

    /// Network mode.
    pub mode: NetworkMode,

    /// Subnet configuration (CIDR notation).
    pub subnet: Option<String>,

    /// Gateway address.
    pub gateway: Option<String>,

    /// IP range for container allocation.
    pub ip_range: Option<String>,

    /// Whether the network is internal (no external connectivity).
    pub internal: bool,

    /// Whether to enable IPv6.
    pub enable_ipv6: bool,

    /// Driver-specific options.
    pub driver_opts: HashMap<String, String>,

    /// Network labels.
    pub labels: HashMap<String, String>,
}

impl Default for NetworkSpec {
    fn default() -> Self {
        Self {
            name: "mamut-network".to_string(),
            driver: "bridge".to_string(),
            mode: NetworkMode::default(),
            subnet: None,
            gateway: None,
            ip_range: None,
            internal: false,
            enable_ipv6: false,
            driver_opts: HashMap::new(),
            labels: HashMap::new(),
        }
    }
}

impl NetworkSpec {
    /// Creates a new network specification builder.
    pub fn builder(name: impl Into<String>) -> NetworkSpecBuilder {
        NetworkSpecBuilder::new(name)
    }
}

/// Builder for `NetworkSpec`.
#[derive(Debug)]
pub struct NetworkSpecBuilder {
    spec: NetworkSpec,
}

impl NetworkSpecBuilder {
    /// Creates a new builder.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            spec: NetworkSpec {
                name: name.into(),
                ..Default::default()
            },
        }
    }

    /// Sets the network driver.
    pub fn driver(mut self, driver: impl Into<String>) -> Self {
        self.spec.driver = driver.into();
        self
    }

    /// Sets the subnet.
    pub fn subnet(mut self, subnet: impl Into<String>) -> Self {
        self.spec.subnet = Some(subnet.into());
        self
    }

    /// Sets the gateway.
    pub fn gateway(mut self, gateway: impl Into<String>) -> Self {
        self.spec.gateway = Some(gateway.into());
        self
    }

    /// Sets the IP range.
    pub fn ip_range(mut self, range: impl Into<String>) -> Self {
        self.spec.ip_range = Some(range.into());
        self
    }

    /// Sets whether the network is internal.
    pub fn internal(mut self, internal: bool) -> Self {
        self.spec.internal = internal;
        self
    }

    /// Sets whether IPv6 is enabled.
    pub fn enable_ipv6(mut self, enable: bool) -> Self {
        self.spec.enable_ipv6 = enable;
        self
    }

    /// Adds a driver option.
    pub fn driver_opt(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.spec.driver_opts.insert(key.into(), value.into());
        self
    }

    /// Adds a label.
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.spec.labels.insert(key.into(), value.into());
        self
    }

    /// Builds the network specification.
    pub fn build(self) -> NetworkSpec {
        self.spec
    }
}

/// Runtime network configuration for fault injection.
///
/// This is used to configure network conditions between nodes during
/// test execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Network partitions (groups of nodes that can communicate).
    pub partitions: Vec<NetworkPartition>,

    /// Latency configuration between nodes.
    pub latency: Vec<LatencyConfig>,

    /// Bandwidth limits between nodes.
    pub bandwidth: Vec<BandwidthConfig>,

    /// Packet loss configuration.
    pub packet_loss: Vec<PacketLossConfig>,

    /// Whether to enable traffic capture.
    pub capture_traffic: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            partitions: Vec::new(),
            latency: Vec::new(),
            bandwidth: Vec::new(),
            packet_loss: Vec::new(),
            capture_traffic: false,
        }
    }
}

impl NetworkConfig {
    /// Creates a new empty network configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a network partition.
    pub fn add_partition(&mut self, partition: NetworkPartition) {
        self.partitions.push(partition);
    }

    /// Adds latency configuration.
    pub fn add_latency(&mut self, config: LatencyConfig) {
        self.latency.push(config);
    }

    /// Adds bandwidth configuration.
    pub fn add_bandwidth(&mut self, config: BandwidthConfig) {
        self.bandwidth.push(config);
    }

    /// Adds packet loss configuration.
    pub fn add_packet_loss(&mut self, config: PacketLossConfig) {
        self.packet_loss.push(config);
    }

    /// Clears all network configurations.
    pub fn clear(&mut self) {
        self.partitions.clear();
        self.latency.clear();
        self.bandwidth.clear();
        self.packet_loss.clear();
    }

    /// Returns true if any network faults are configured.
    pub fn has_faults(&self) -> bool {
        !self.partitions.is_empty()
            || !self.latency.is_empty()
            || !self.bandwidth.is_empty()
            || !self.packet_loss.is_empty()
    }
}

/// A network partition defines a group of nodes that can communicate.
///
/// Nodes in different partitions cannot communicate with each other.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPartition {
    /// Name of this partition.
    pub name: String,

    /// Nodes in this partition.
    pub nodes: Vec<NodeId>,
}

impl NetworkPartition {
    /// Creates a new partition with the given nodes.
    pub fn new(name: impl Into<String>, nodes: Vec<NodeId>) -> Self {
        Self {
            name: name.into(),
            nodes,
        }
    }
}

/// Latency configuration between nodes or partitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyConfig {
    /// Source node or partition.
    pub source: NetworkTarget,

    /// Destination node or partition.
    pub destination: NetworkTarget,

    /// Latency in milliseconds.
    pub latency_ms: u32,

    /// Jitter in milliseconds (optional random variation).
    pub jitter_ms: Option<u32>,

    /// Correlation percentage for jitter.
    pub correlation_pct: Option<u32>,
}

impl LatencyConfig {
    /// Creates a new latency configuration.
    pub fn new(source: NetworkTarget, destination: NetworkTarget, latency_ms: u32) -> Self {
        Self {
            source,
            destination,
            latency_ms,
            jitter_ms: None,
            correlation_pct: None,
        }
    }

    /// Adds jitter to the latency.
    pub fn with_jitter(mut self, jitter_ms: u32) -> Self {
        self.jitter_ms = Some(jitter_ms);
        self
    }

    /// Adds correlation to the jitter.
    pub fn with_correlation(mut self, correlation_pct: u32) -> Self {
        self.correlation_pct = Some(correlation_pct);
        self
    }
}

/// Bandwidth limit configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthConfig {
    /// Source node or partition.
    pub source: NetworkTarget,

    /// Destination node or partition.
    pub destination: NetworkTarget,

    /// Bandwidth limit in kilobits per second.
    pub rate_kbps: u32,

    /// Burst size in kilobytes.
    pub burst_kb: Option<u32>,
}

impl BandwidthConfig {
    /// Creates a new bandwidth configuration.
    pub fn new(source: NetworkTarget, destination: NetworkTarget, rate_kbps: u32) -> Self {
        Self {
            source,
            destination,
            rate_kbps,
            burst_kb: None,
        }
    }

    /// Sets the burst size.
    pub fn with_burst(mut self, burst_kb: u32) -> Self {
        self.burst_kb = Some(burst_kb);
        self
    }
}

/// Packet loss configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketLossConfig {
    /// Source node or partition.
    pub source: NetworkTarget,

    /// Destination node or partition.
    pub destination: NetworkTarget,

    /// Packet loss percentage (0-100).
    pub loss_pct: f32,

    /// Correlation percentage.
    pub correlation_pct: Option<u32>,
}

impl PacketLossConfig {
    /// Creates a new packet loss configuration.
    pub fn new(source: NetworkTarget, destination: NetworkTarget, loss_pct: f32) -> Self {
        Self {
            source,
            destination,
            loss_pct,
            correlation_pct: None,
        }
    }

    /// Adds correlation to the packet loss.
    pub fn with_correlation(mut self, correlation_pct: u32) -> Self {
        self.correlation_pct = Some(correlation_pct);
        self
    }
}

/// Target for network configuration (node or partition).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkTarget {
    /// A specific node.
    Node(NodeId),

    /// All nodes.
    All,

    /// A named partition.
    Partition(String),
}

impl From<NodeId> for NetworkTarget {
    fn from(id: NodeId) -> Self {
        Self::Node(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_port_mapping() {
        let port = PortMapping::tcp(8080).host_port(80);
        assert_eq!(port.container_port, 8080);
        assert_eq!(port.host_port, Some(80));
        assert_eq!(port.as_docker_port(), "80:8080/tcp");
    }

    #[test]
    fn test_port_mapping_with_ip() {
        let port = PortMapping::tcp(8080)
            .host_port(80)
            .host_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        assert_eq!(port.as_docker_port(), "127.0.0.1:80:8080/tcp");
    }

    #[test]
    fn test_network_spec_builder() {
        let spec = NetworkSpec::builder("test-network")
            .subnet("172.28.0.0/16")
            .gateway("172.28.0.1")
            .internal(true)
            .build();

        assert_eq!(spec.name, "test-network");
        assert_eq!(spec.subnet, Some("172.28.0.0/16".to_string()));
        assert!(spec.internal);
    }

    #[test]
    fn test_network_config() {
        let mut config = NetworkConfig::new();
        assert!(!config.has_faults());

        config.add_latency(LatencyConfig::new(
            NetworkTarget::All,
            NetworkTarget::All,
            100,
        ));
        assert!(config.has_faults());

        config.clear();
        assert!(!config.has_faults());
    }

    #[test]
    fn test_latency_config() {
        let config = LatencyConfig::new(
            NetworkTarget::Node(NodeId::new(1)),
            NetworkTarget::Node(NodeId::new(2)),
            50,
        )
        .with_jitter(10)
        .with_correlation(25);

        assert_eq!(config.latency_ms, 50);
        assert_eq!(config.jitter_ms, Some(10));
        assert_eq!(config.correlation_pct, Some(25));
    }

    #[test]
    fn test_network_partition() {
        let partition =
            NetworkPartition::new("partition-a", vec![NodeId::new(1), NodeId::new(2)]);

        assert_eq!(partition.name, "partition-a");
        assert_eq!(partition.nodes.len(), 2);
    }
}
