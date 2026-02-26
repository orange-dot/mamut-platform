//! Orchestration backend trait definition.
//!
//! This module defines the core trait that all orchestration backends must
//! implement, along with supporting types for capabilities and execution.

use async_trait::async_trait;
use mamut_core::node::NodeId;
use serde::{Deserialize, Serialize};

use crate::container::ContainerHandle;
use crate::error::Result;
use crate::topology::{ClusterTopology, DeployedCluster, NetworkConfig};

/// Core trait for container orchestration backends.
///
/// An orchestration backend manages the deployment, lifecycle, and networking
/// of containers in a distributed system under test. Different backends can
/// support different container runtimes (Docker, Podman, Kubernetes, etc.).
///
/// # Lifecycle
///
/// 1. Check `capabilities()` to verify the backend supports required features
/// 2. Call `deploy()` to create and start the cluster
/// 3. Use `exec()` to run commands in containers during testing
/// 4. Use `configure_network()` to inject network faults
/// 5. Call `teardown()` to clean up all resources
///
/// # Thread Safety
///
/// Implementations must be thread-safe (`Send + Sync`) to support concurrent
/// operations during distributed system testing.
///
/// # Example
///
/// ```ignore
/// use mamut_orchestrator::backend::{OrchestrationBackend, DockerComposeBackend};
/// use mamut_orchestrator::topology::ClusterTopology;
///
/// async fn deploy_cluster(topology: ClusterTopology) -> Result<DeployedCluster> {
///     let backend = DockerComposeBackend::new().await?;
///
///     // Check capabilities
///     let caps = backend.capabilities();
///     if !caps.supports_network_faults {
///         println!("Warning: Network faults not supported");
///     }
///
///     // Deploy the cluster
///     let cluster = backend.deploy(&topology).await?;
///
///     // Run a command in a container
///     let output = backend.exec(&NodeId::new(1), &["echo", "hello"]).await?;
///     assert_eq!(output.exit_code, 0);
///
///     Ok(cluster)
/// }
/// ```
#[async_trait]
pub trait OrchestrationBackend: Send + Sync {
    /// Deploys a cluster based on the given topology.
    ///
    /// This method should:
    /// - Create the network(s) specified in the topology
    /// - Pull any required images
    /// - Create and start containers for all nodes
    /// - Wait for containers to be healthy (if health checks are configured)
    ///
    /// # Arguments
    ///
    /// * `topology` - The cluster topology to deploy
    ///
    /// # Returns
    ///
    /// A `DeployedCluster` containing references to all created resources,
    /// or an error if deployment failed.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Network creation fails
    /// - Image pull fails
    /// - Container creation or start fails
    /// - Health checks don't pass within the timeout
    async fn deploy(&self, topology: &ClusterTopology) -> Result<DeployedCluster>;

    /// Tears down a deployed cluster.
    ///
    /// This method should:
    /// - Stop all containers
    /// - Remove all containers
    /// - Remove created networks
    /// - Clean up any other resources (volumes, etc.)
    ///
    /// # Arguments
    ///
    /// * `cluster` - The deployed cluster to tear down
    ///
    /// # Returns
    ///
    /// `Ok(())` if teardown was successful, or an error if cleanup failed.
    /// Note that partial cleanup may leave resources in an inconsistent state.
    async fn teardown(&self, cluster: &DeployedCluster) -> Result<()>;

    /// Gets a handle to a container by node ID.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The node ID to look up
    ///
    /// # Returns
    ///
    /// A `ContainerHandle` for the specified node, or an error if the
    /// container is not found.
    async fn get_container(&self, node_id: &NodeId) -> Result<ContainerHandle>;

    /// Executes a command in a container.
    ///
    /// This is the primary mechanism for interacting with nodes during testing.
    /// Commands are executed in an exec session within the running container.
    ///
    /// # Arguments
    ///
    /// * `node_id` - The node to execute the command in
    /// * `cmd` - The command and arguments to execute
    ///
    /// # Returns
    ///
    /// An `ExecOutput` containing stdout, stderr, and the exit code.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let output = backend.exec(&NodeId::new(1), &["redis-cli", "PING"]).await?;
    /// assert_eq!(output.stdout.trim(), "PONG");
    /// ```
    async fn exec(&self, node_id: &NodeId, cmd: &[&str]) -> Result<ExecOutput>;

    /// Configures network conditions between nodes.
    ///
    /// This method is used for network fault injection, including:
    /// - Network partitions
    /// - Latency injection
    /// - Bandwidth limits
    /// - Packet loss
    ///
    /// # Arguments
    ///
    /// * `config` - The network configuration to apply
    ///
    /// # Returns
    ///
    /// `Ok(())` if configuration was successful, or an error if the backend
    /// doesn't support the requested configuration.
    ///
    /// # Note
    ///
    /// Not all backends support all network configurations. Check
    /// `capabilities().supports_network_faults` before calling this method.
    async fn configure_network(&self, config: &NetworkConfig) -> Result<()>;

    /// Returns the capabilities of this backend.
    ///
    /// This method returns information about what features the backend supports,
    /// which can be used to skip incompatible tests or provide warnings.
    fn capabilities(&self) -> BackendCapabilities;

    /// Returns the name of this backend.
    fn name(&self) -> &str;

    /// Checks if the backend is connected and ready.
    async fn is_ready(&self) -> bool {
        true
    }

    /// Pauses a container.
    ///
    /// Default implementation returns an error if not supported.
    async fn pause(&self, node_id: &NodeId) -> Result<()> {
        let _ = node_id;
        Err(crate::error::OrchestratorError::capability_not_supported(
            "pause",
        ))
    }

    /// Unpauses a container.
    ///
    /// Default implementation returns an error if not supported.
    async fn unpause(&self, node_id: &NodeId) -> Result<()> {
        let _ = node_id;
        Err(crate::error::OrchestratorError::capability_not_supported(
            "unpause",
        ))
    }

    /// Restarts a container.
    ///
    /// Default implementation returns an error if not supported.
    async fn restart(&self, node_id: &NodeId) -> Result<()> {
        let _ = node_id;
        Err(crate::error::OrchestratorError::capability_not_supported(
            "restart",
        ))
    }

    /// Kills a container with a signal.
    ///
    /// Default implementation returns an error if not supported.
    async fn kill(&self, node_id: &NodeId, signal: &str) -> Result<()> {
        let _ = (node_id, signal);
        Err(crate::error::OrchestratorError::capability_not_supported(
            "kill",
        ))
    }

    /// Gets logs from a container.
    ///
    /// Default implementation returns an error if not supported.
    async fn logs(&self, node_id: &NodeId, tail: Option<usize>) -> Result<String> {
        let _ = (node_id, tail);
        Err(crate::error::OrchestratorError::capability_not_supported(
            "logs",
        ))
    }
}

/// Capabilities supported by an orchestration backend.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BackendCapabilities {
    /// Whether the backend supports network fault injection.
    pub supports_network_faults: bool,

    /// Whether the backend supports pausing containers.
    pub supports_pause: bool,

    /// Whether the backend supports resource limits.
    pub supports_resource_limits: bool,

    /// Whether the backend supports health checks.
    pub supports_health_checks: bool,

    /// Whether the backend supports container exec.
    pub supports_exec: bool,

    /// Whether the backend supports log streaming.
    pub supports_logs: bool,

    /// Whether the backend supports volume mounts.
    pub supports_volumes: bool,

    /// Whether the backend supports custom networks.
    pub supports_custom_networks: bool,

    /// Whether the backend can capture traffic.
    pub supports_traffic_capture: bool,

    /// Maximum number of containers supported (0 = unlimited).
    pub max_containers: u32,

    /// Additional backend-specific capabilities.
    pub extensions: std::collections::HashMap<String, bool>,
}

impl BackendCapabilities {
    /// Creates capabilities for a fully-featured Docker backend.
    pub fn docker_full() -> Self {
        Self {
            supports_network_faults: true,
            supports_pause: true,
            supports_resource_limits: true,
            supports_health_checks: true,
            supports_exec: true,
            supports_logs: true,
            supports_volumes: true,
            supports_custom_networks: true,
            supports_traffic_capture: false,
            max_containers: 0,
            extensions: std::collections::HashMap::new(),
        }
    }

    /// Creates minimal capabilities.
    pub fn minimal() -> Self {
        Self {
            supports_exec: true,
            ..Default::default()
        }
    }

    /// Checks if a capability is supported.
    pub fn has(&self, capability: &str) -> bool {
        match capability {
            "network_faults" => self.supports_network_faults,
            "pause" => self.supports_pause,
            "resource_limits" => self.supports_resource_limits,
            "health_checks" => self.supports_health_checks,
            "exec" => self.supports_exec,
            "logs" => self.supports_logs,
            "volumes" => self.supports_volumes,
            "custom_networks" => self.supports_custom_networks,
            "traffic_capture" => self.supports_traffic_capture,
            other => self.extensions.get(other).copied().unwrap_or(false),
        }
    }

    /// Adds an extension capability.
    pub fn with_extension(mut self, name: impl Into<String>, supported: bool) -> Self {
        self.extensions.insert(name.into(), supported);
        self
    }
}

/// Output from executing a command in a container.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecOutput {
    /// Standard output from the command.
    pub stdout: String,

    /// Standard error from the command.
    pub stderr: String,

    /// Exit code of the command.
    pub exit_code: i32,
}

impl ExecOutput {
    /// Creates a new exec output.
    pub fn new(stdout: impl Into<String>, stderr: impl Into<String>, exit_code: i32) -> Self {
        Self {
            stdout: stdout.into(),
            stderr: stderr.into(),
            exit_code,
        }
    }

    /// Returns true if the command succeeded (exit code 0).
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }

    /// Returns the combined stdout and stderr.
    pub fn combined_output(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else if self.stdout.is_empty() {
            self.stderr.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }

    /// Returns stdout lines as a vector.
    pub fn stdout_lines(&self) -> Vec<&str> {
        self.stdout.lines().collect()
    }

    /// Returns stderr lines as a vector.
    pub fn stderr_lines(&self) -> Vec<&str> {
        self.stderr.lines().collect()
    }
}

/// A boxed orchestration backend for dynamic dispatch.
pub type BoxedBackend = Box<dyn OrchestrationBackend>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_capabilities() {
        let caps = BackendCapabilities::docker_full();
        assert!(caps.supports_exec);
        assert!(caps.supports_network_faults);
        assert!(caps.has("exec"));
        assert!(caps.has("network_faults"));
        assert!(!caps.has("unknown"));
    }

    #[test]
    fn test_capabilities_extension() {
        let caps = BackendCapabilities::minimal().with_extension("custom_feature", true);
        assert!(caps.has("custom_feature"));
    }

    #[test]
    fn test_exec_output() {
        let output = ExecOutput::new("hello\nworld", "", 0);
        assert!(output.success());
        assert_eq!(output.stdout_lines(), vec!["hello", "world"]);

        let output = ExecOutput::new("", "error", 1);
        assert!(!output.success());
        assert_eq!(output.combined_output(), "error");
    }

    #[test]
    fn test_exec_output_combined() {
        let output = ExecOutput::new("out", "err", 0);
        assert_eq!(output.combined_output(), "out\nerr");
    }
}
