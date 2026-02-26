//! Cluster topology definitions.
//!
//! This module defines the structure of a distributed system cluster,
//! including node specifications, roles, and deployed cluster state.

use mamut_core::node::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::container::{ContainerImage, ResourceLimits};
use crate::topology::network::{NetworkSpec, PortMapping};

/// The role a node plays in the cluster.
///
/// Different roles may have different configurations, resource allocations,
/// and behaviors during testing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeRole {
    /// A node that stores and replicates data.
    DataNode,

    /// A node that coordinates operations across the cluster.
    Coordinator,

    /// A node that acts as an entry point for client requests.
    Gateway,

    /// A node used for monitoring and observability.
    Monitor,

    /// A client node that generates operations.
    Client,

    /// A custom role with a user-defined name.
    Custom(String),
}

impl NodeRole {
    /// Creates a new custom role.
    pub fn custom(name: impl Into<String>) -> Self {
        Self::Custom(name.into())
    }

    /// Returns the name of this role.
    pub fn name(&self) -> &str {
        match self {
            Self::DataNode => "data-node",
            Self::Coordinator => "coordinator",
            Self::Gateway => "gateway",
            Self::Monitor => "monitor",
            Self::Client => "client",
            Self::Custom(name) => name,
        }
    }
}

impl Default for NodeRole {
    fn default() -> Self {
        Self::DataNode
    }
}

/// Specification for a single node in the cluster.
///
/// A node spec describes everything needed to deploy and configure
/// a container for a specific node in the distributed system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSpec {
    /// Unique identifier for this node.
    pub id: NodeId,

    /// Human-readable name for the node.
    pub name: String,

    /// The role this node plays in the cluster.
    pub role: NodeRole,

    /// The container image to use for this node.
    pub image: ContainerImage,

    /// Resource limits for the container.
    pub resources: ResourceLimits,

    /// Port mappings for the container.
    pub ports: Vec<PortMapping>,

    /// Environment variables to set in the container.
    pub environment: HashMap<String, String>,

    /// Command to run in the container (overrides image default).
    pub command: Option<Vec<String>>,

    /// Arguments to pass to the command.
    pub args: Option<Vec<String>>,

    /// Volume mounts for the container.
    pub volumes: Vec<VolumeMount>,

    /// Labels to apply to the container.
    pub labels: HashMap<String, String>,

    /// Health check configuration.
    pub health_check: Option<HealthCheck>,

    /// Dependencies on other nodes (must be started first).
    pub depends_on: Vec<NodeId>,

    /// Restart policy for the container.
    pub restart_policy: RestartPolicy,
}

impl NodeSpec {
    /// Creates a new node specification builder.
    pub fn builder(id: NodeId) -> NodeSpecBuilder {
        NodeSpecBuilder::new(id)
    }

    /// Returns the container name for this node.
    pub fn container_name(&self) -> String {
        format!("mamut-{}-{}", self.role.name(), self.id.inner())
    }
}

/// Builder for `NodeSpec`.
#[derive(Debug)]
pub struct NodeSpecBuilder {
    id: NodeId,
    name: Option<String>,
    role: NodeRole,
    image: Option<ContainerImage>,
    resources: ResourceLimits,
    ports: Vec<PortMapping>,
    environment: HashMap<String, String>,
    command: Option<Vec<String>>,
    args: Option<Vec<String>>,
    volumes: Vec<VolumeMount>,
    labels: HashMap<String, String>,
    health_check: Option<HealthCheck>,
    depends_on: Vec<NodeId>,
    restart_policy: RestartPolicy,
}

impl NodeSpecBuilder {
    /// Creates a new builder with default values.
    pub fn new(id: NodeId) -> Self {
        Self {
            id,
            name: None,
            role: NodeRole::default(),
            image: None,
            resources: ResourceLimits::default(),
            ports: Vec::new(),
            environment: HashMap::new(),
            command: None,
            args: None,
            volumes: Vec::new(),
            labels: HashMap::new(),
            health_check: None,
            depends_on: Vec::new(),
            restart_policy: RestartPolicy::default(),
        }
    }

    /// Sets the node name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the node role.
    pub fn role(mut self, role: NodeRole) -> Self {
        self.role = role;
        self
    }

    /// Sets the container image.
    pub fn image(mut self, image: ContainerImage) -> Self {
        self.image = Some(image);
        self
    }

    /// Sets the resource limits.
    pub fn resources(mut self, resources: ResourceLimits) -> Self {
        self.resources = resources;
        self
    }

    /// Adds a port mapping.
    pub fn port(mut self, mapping: PortMapping) -> Self {
        self.ports.push(mapping);
        self
    }

    /// Adds an environment variable.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment.insert(key.into(), value.into());
        self
    }

    /// Sets the command to run.
    pub fn command(mut self, cmd: Vec<String>) -> Self {
        self.command = Some(cmd);
        self
    }

    /// Sets the command arguments.
    pub fn args(mut self, args: Vec<String>) -> Self {
        self.args = Some(args);
        self
    }

    /// Adds a volume mount.
    pub fn volume(mut self, mount: VolumeMount) -> Self {
        self.volumes.push(mount);
        self
    }

    /// Adds a label.
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Sets the health check configuration.
    pub fn health_check(mut self, check: HealthCheck) -> Self {
        self.health_check = Some(check);
        self
    }

    /// Adds a dependency on another node.
    pub fn depends_on(mut self, node_id: NodeId) -> Self {
        self.depends_on.push(node_id);
        self
    }

    /// Sets the restart policy.
    pub fn restart_policy(mut self, policy: RestartPolicy) -> Self {
        self.restart_policy = policy;
        self
    }

    /// Builds the node specification.
    ///
    /// # Panics
    ///
    /// Panics if required fields (image) are not set.
    pub fn build(self) -> NodeSpec {
        NodeSpec {
            id: self.id,
            name: self.name.unwrap_or_else(|| format!("node-{}", self.id.inner())),
            role: self.role,
            image: self.image.expect("image is required"),
            resources: self.resources,
            ports: self.ports,
            environment: self.environment,
            command: self.command,
            args: self.args,
            volumes: self.volumes,
            labels: self.labels,
            health_check: self.health_check,
            depends_on: self.depends_on,
            restart_policy: self.restart_policy,
        }
    }
}

/// Volume mount configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    /// Source path on the host or volume name.
    pub source: String,

    /// Target path in the container.
    pub target: String,

    /// Whether the mount is read-only.
    pub read_only: bool,

    /// Volume type.
    pub volume_type: VolumeType,
}

impl VolumeMount {
    /// Creates a new bind mount.
    pub fn bind(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            read_only: false,
            volume_type: VolumeType::Bind,
        }
    }

    /// Creates a new named volume mount.
    pub fn volume(name: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: name.into(),
            target: target.into(),
            read_only: false,
            volume_type: VolumeType::Volume,
        }
    }

    /// Creates a new tmpfs mount.
    pub fn tmpfs(target: impl Into<String>) -> Self {
        Self {
            source: String::new(),
            target: target.into(),
            read_only: false,
            volume_type: VolumeType::Tmpfs,
        }
    }

    /// Sets the mount to read-only.
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }
}

/// Type of volume mount.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VolumeType {
    /// Bind mount from host filesystem.
    Bind,

    /// Named volume managed by Docker.
    Volume,

    /// Temporary filesystem in memory.
    Tmpfs,
}

/// Health check configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Command to run for health check.
    pub test: Vec<String>,

    /// Time to wait between checks.
    pub interval_secs: u32,

    /// Time to wait for a check to complete.
    pub timeout_secs: u32,

    /// Number of consecutive failures for unhealthy status.
    pub retries: u32,

    /// Time to wait before starting health checks.
    pub start_period_secs: u32,
}

impl Default for HealthCheck {
    fn default() -> Self {
        Self {
            test: vec!["CMD-SHELL".to_string(), "exit 0".to_string()],
            interval_secs: 30,
            timeout_secs: 30,
            retries: 3,
            start_period_secs: 0,
        }
    }
}

impl HealthCheck {
    /// Creates a health check using curl.
    pub fn http(url: &str) -> Self {
        Self {
            test: vec![
                "CMD-SHELL".to_string(),
                format!("curl -f {} || exit 1", url),
            ],
            ..Default::default()
        }
    }

    /// Creates a health check using a TCP connection.
    pub fn tcp(host: &str, port: u16) -> Self {
        Self {
            test: vec![
                "CMD-SHELL".to_string(),
                format!("nc -z {} {} || exit 1", host, port),
            ],
            ..Default::default()
        }
    }

    /// Creates a custom command health check.
    pub fn cmd(command: Vec<String>) -> Self {
        let mut test = vec!["CMD".to_string()];
        test.extend(command);
        Self {
            test,
            ..Default::default()
        }
    }
}

/// Container restart policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestartPolicy {
    /// Never restart.
    No,

    /// Restart on failure.
    OnFailure {
        /// Maximum number of retries.
        max_retries: Option<u32>,
    },

    /// Always restart.
    Always,

    /// Restart unless explicitly stopped.
    UnlessStopped,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self::No
    }
}

impl RestartPolicy {
    /// Returns the Docker Compose restart policy string.
    pub fn as_compose_str(&self) -> &str {
        match self {
            Self::No => "no",
            Self::OnFailure { .. } => "on-failure",
            Self::Always => "always",
            Self::UnlessStopped => "unless-stopped",
        }
    }
}

/// The complete topology of a cluster.
///
/// A cluster topology describes all nodes, their configurations, and
/// the network setup for a distributed system deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterTopology {
    /// Unique name for this cluster.
    pub name: String,

    /// Node specifications indexed by node ID.
    pub nodes: HashMap<NodeId, NodeSpec>,

    /// Network configuration for the cluster.
    pub network: NetworkSpec,

    /// Global environment variables applied to all nodes.
    pub global_env: HashMap<String, String>,

    /// Global labels applied to all containers.
    pub global_labels: HashMap<String, String>,
}

impl ClusterTopology {
    /// Creates a new cluster topology builder.
    pub fn builder(name: impl Into<String>) -> ClusterTopologyBuilder {
        ClusterTopologyBuilder::new(name)
    }

    /// Returns the number of nodes in the cluster.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns an iterator over all node specifications.
    pub fn iter_nodes(&self) -> impl Iterator<Item = &NodeSpec> {
        self.nodes.values()
    }

    /// Returns nodes filtered by role.
    pub fn nodes_by_role(&self, role: &NodeRole) -> impl Iterator<Item = &NodeSpec> {
        self.nodes.values().filter(move |n| &n.role == role)
    }

    /// Gets a node specification by ID.
    pub fn get_node(&self, id: &NodeId) -> Option<&NodeSpec> {
        self.nodes.get(id)
    }

    /// Returns nodes in dependency order (dependencies first).
    pub fn nodes_in_order(&self) -> Vec<&NodeSpec> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();

        fn visit<'a>(
            node: &'a NodeSpec,
            nodes: &'a HashMap<NodeId, NodeSpec>,
            visited: &mut std::collections::HashSet<NodeId>,
            result: &mut Vec<&'a NodeSpec>,
        ) {
            if visited.contains(&node.id) {
                return;
            }

            for dep_id in &node.depends_on {
                if let Some(dep_node) = nodes.get(dep_id) {
                    visit(dep_node, nodes, visited, result);
                }
            }

            visited.insert(node.id);
            result.push(node);
        }

        for node in self.nodes.values() {
            visit(node, &self.nodes, &mut visited, &mut result);
        }

        result
    }
}

/// Builder for `ClusterTopology`.
#[derive(Debug)]
pub struct ClusterTopologyBuilder {
    name: String,
    nodes: HashMap<NodeId, NodeSpec>,
    network: NetworkSpec,
    global_env: HashMap<String, String>,
    global_labels: HashMap<String, String>,
}

impl ClusterTopologyBuilder {
    /// Creates a new builder.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: HashMap::new(),
            network: NetworkSpec::default(),
            global_env: HashMap::new(),
            global_labels: HashMap::new(),
        }
    }

    /// Adds a node to the cluster.
    pub fn node(mut self, spec: NodeSpec) -> Self {
        self.nodes.insert(spec.id, spec);
        self
    }

    /// Sets the network specification.
    pub fn network(mut self, network: NetworkSpec) -> Self {
        self.network = network;
        self
    }

    /// Adds a global environment variable.
    pub fn global_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.global_env.insert(key.into(), value.into());
        self
    }

    /// Adds a global label.
    pub fn global_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.global_labels.insert(key.into(), value.into());
        self
    }

    /// Builds the cluster topology.
    pub fn build(self) -> ClusterTopology {
        ClusterTopology {
            name: self.name,
            nodes: self.nodes,
            network: self.network,
            global_env: self.global_env,
            global_labels: self.global_labels,
        }
    }
}

/// A deployed cluster with runtime information.
///
/// This represents a cluster that has been successfully deployed and
/// contains references to the running containers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployedCluster {
    /// The original topology used for deployment.
    pub topology: ClusterTopology,

    /// Unique deployment ID.
    pub deployment_id: String,

    /// Container IDs indexed by node ID.
    pub container_ids: HashMap<NodeId, String>,

    /// Network ID for the cluster network.
    pub network_id: Option<String>,

    /// Deployment timestamp (Unix milliseconds).
    pub deployed_at: u64,

    /// Current state of the deployment.
    pub state: DeploymentState,
}

impl DeployedCluster {
    /// Creates a new deployed cluster.
    pub fn new(
        topology: ClusterTopology,
        deployment_id: impl Into<String>,
        deployed_at: u64,
    ) -> Self {
        Self {
            topology,
            deployment_id: deployment_id.into(),
            container_ids: HashMap::new(),
            network_id: None,
            deployed_at,
            state: DeploymentState::Pending,
        }
    }

    /// Returns the container ID for a node.
    pub fn container_id(&self, node_id: &NodeId) -> Option<&str> {
        self.container_ids.get(node_id).map(|s| s.as_str())
    }

    /// Returns true if all nodes are running.
    pub fn is_running(&self) -> bool {
        matches!(self.state, DeploymentState::Running)
    }
}

/// State of a cluster deployment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeploymentState {
    /// Deployment is pending.
    Pending,

    /// Cluster is starting up.
    Starting,

    /// Cluster is running.
    Running,

    /// Cluster is stopping.
    Stopping,

    /// Cluster has stopped.
    Stopped,

    /// Deployment failed.
    Failed {
        /// Error message.
        error: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::ContainerImage;

    #[test]
    fn test_node_role_name() {
        assert_eq!(NodeRole::DataNode.name(), "data-node");
        assert_eq!(NodeRole::custom("kafka-broker").name(), "kafka-broker");
    }

    #[test]
    fn test_node_spec_builder() {
        let spec = NodeSpec::builder(NodeId::new(1))
            .name("test-node")
            .role(NodeRole::DataNode)
            .image(ContainerImage::new("redis:7"))
            .env("REDIS_PORT", "6379")
            .build();

        assert_eq!(spec.id, NodeId::new(1));
        assert_eq!(spec.name, "test-node");
        assert_eq!(spec.role, NodeRole::DataNode);
        assert_eq!(spec.environment.get("REDIS_PORT"), Some(&"6379".to_string()));
    }

    #[test]
    fn test_cluster_topology_builder() {
        let node1 = NodeSpec::builder(NodeId::new(1))
            .image(ContainerImage::new("redis:7"))
            .build();

        let node2 = NodeSpec::builder(NodeId::new(2))
            .image(ContainerImage::new("redis:7"))
            .depends_on(NodeId::new(1))
            .build();

        let topology = ClusterTopology::builder("test-cluster")
            .node(node1)
            .node(node2)
            .global_env("CLUSTER_NAME", "test")
            .build();

        assert_eq!(topology.name, "test-cluster");
        assert_eq!(topology.node_count(), 2);
    }

    #[test]
    fn test_nodes_in_order() {
        let node1 = NodeSpec::builder(NodeId::new(1))
            .image(ContainerImage::new("redis:7"))
            .build();

        let node2 = NodeSpec::builder(NodeId::new(2))
            .image(ContainerImage::new("redis:7"))
            .depends_on(NodeId::new(1))
            .build();

        let node3 = NodeSpec::builder(NodeId::new(3))
            .image(ContainerImage::new("redis:7"))
            .depends_on(NodeId::new(2))
            .build();

        let topology = ClusterTopology::builder("test")
            .node(node3)
            .node(node1)
            .node(node2)
            .build();

        let ordered: Vec<NodeId> = topology.nodes_in_order().iter().map(|n| n.id).collect();

        // Node 1 must come before node 2, node 2 before node 3
        let pos1 = ordered.iter().position(|id| *id == NodeId::new(1)).unwrap();
        let pos2 = ordered.iter().position(|id| *id == NodeId::new(2)).unwrap();
        let pos3 = ordered.iter().position(|id| *id == NodeId::new(3)).unwrap();

        assert!(pos1 < pos2);
        assert!(pos2 < pos3);
    }

    #[test]
    fn test_restart_policy() {
        assert_eq!(RestartPolicy::No.as_compose_str(), "no");
        assert_eq!(RestartPolicy::Always.as_compose_str(), "always");
        assert_eq!(
            RestartPolicy::OnFailure { max_retries: Some(3) }.as_compose_str(),
            "on-failure"
        );
    }
}
