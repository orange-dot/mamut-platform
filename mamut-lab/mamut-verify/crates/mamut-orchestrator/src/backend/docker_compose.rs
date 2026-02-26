//! Docker Compose backend implementation.
//!
//! This module provides an orchestration backend that uses the Docker API
//! (via bollard) to manage containers, with docker-compose.yml generation
//! for reproducibility.

use async_trait::async_trait;
use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, LogsOptions, RemoveContainerOptions,
    StartContainerOptions, StopContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::CreateImageOptions;
use bollard::network::{CreateNetworkOptions, ListNetworksOptions};
use bollard::Docker;
use futures::StreamExt;
use mamut_core::node::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::backend::r#trait::{BackendCapabilities, ExecOutput, OrchestrationBackend};
use crate::container::{ContainerHandle, ContainerState, LifecycleManager};
use crate::error::{OrchestratorError, Result};
use crate::topology::{
    ClusterTopology, DeployedCluster, DeploymentState, NetworkConfig, NodeSpec, PortMapping,
};

/// Configuration for the Docker Compose backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerComposeConfig {
    /// Project name prefix for containers and networks.
    pub project_name: String,

    /// Working directory for docker-compose.yml generation.
    pub work_dir: PathBuf,

    /// Whether to remove containers on teardown.
    pub remove_on_teardown: bool,

    /// Whether to remove volumes on teardown.
    pub remove_volumes: bool,

    /// Whether to generate docker-compose.yml files.
    pub generate_compose_file: bool,

    /// Timeout for container operations in seconds.
    pub timeout_secs: u64,

    /// Whether to pull images before deployment.
    pub pull_images: bool,

    /// Whether to use host networking.
    pub use_host_network: bool,

    /// Additional Docker labels to apply to all containers.
    pub labels: HashMap<String, String>,
}

impl Default for DockerComposeConfig {
    fn default() -> Self {
        Self {
            project_name: "mamut".to_string(),
            work_dir: std::env::temp_dir().join("mamut-orchestrator"),
            remove_on_teardown: true,
            remove_volumes: true,
            generate_compose_file: true,
            timeout_secs: 120,
            pull_images: true,
            use_host_network: false,
            labels: HashMap::new(),
        }
    }
}

impl DockerComposeConfig {
    /// Creates a new configuration builder.
    pub fn builder() -> DockerComposeConfigBuilder {
        DockerComposeConfigBuilder::default()
    }
}

/// Builder for `DockerComposeConfig`.
#[derive(Debug, Default)]
pub struct DockerComposeConfigBuilder {
    config: DockerComposeConfig,
}

impl DockerComposeConfigBuilder {
    /// Sets the project name.
    pub fn project_name(mut self, name: impl Into<String>) -> Self {
        self.config.project_name = name.into();
        self
    }

    /// Sets the working directory.
    pub fn work_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.work_dir = path.into();
        self
    }

    /// Sets whether to remove containers on teardown.
    pub fn remove_on_teardown(mut self, remove: bool) -> Self {
        self.config.remove_on_teardown = remove;
        self
    }

    /// Sets whether to remove volumes on teardown.
    pub fn remove_volumes(mut self, remove: bool) -> Self {
        self.config.remove_volumes = remove;
        self
    }

    /// Sets whether to generate docker-compose.yml files.
    pub fn generate_compose_file(mut self, generate: bool) -> Self {
        self.config.generate_compose_file = generate;
        self
    }

    /// Sets the timeout for container operations.
    pub fn timeout_secs(mut self, secs: u64) -> Self {
        self.config.timeout_secs = secs;
        self
    }

    /// Sets whether to pull images before deployment.
    pub fn pull_images(mut self, pull: bool) -> Self {
        self.config.pull_images = pull;
        self
    }

    /// Sets whether to use host networking.
    pub fn use_host_network(mut self, use_host: bool) -> Self {
        self.config.use_host_network = use_host;
        self
    }

    /// Adds a label.
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.labels.insert(key.into(), value.into());
        self
    }

    /// Builds the configuration.
    pub fn build(self) -> DockerComposeConfig {
        self.config
    }
}

/// Docker Compose orchestration backend.
///
/// This backend uses the Docker API directly via bollard for container
/// management, while also generating docker-compose.yml files for
/// reproducibility and manual debugging.
pub struct DockerComposeBackend {
    /// Docker client.
    docker: Docker,

    /// Backend configuration.
    config: DockerComposeConfig,

    /// Container lifecycle manager.
    lifecycle: Arc<LifecycleManager>,

    /// Current deployment (if any).
    current_deployment: Arc<RwLock<Option<DeployedCluster>>>,

    /// Node ID to container ID mapping.
    node_containers: Arc<RwLock<HashMap<NodeId, String>>>,
}

impl DockerComposeBackend {
    /// Creates a new Docker Compose backend with default configuration.
    pub async fn new() -> Result<Self> {
        Self::with_config(DockerComposeConfig::default()).await
    }

    /// Creates a new Docker Compose backend with the given configuration.
    pub async fn with_config(config: DockerComposeConfig) -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;

        // Verify connection
        docker.ping().await?;

        info!(
            project = %config.project_name,
            "Connected to Docker daemon"
        );

        Ok(Self {
            docker,
            config,
            lifecycle: Arc::new(LifecycleManager::new()),
            current_deployment: Arc::new(RwLock::new(None)),
            node_containers: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Generates a docker-compose.yml file for the topology.
    pub fn generate_compose_yaml(&self, topology: &ClusterTopology) -> Result<String> {
        let mut compose = ComposeFile::new();

        // Add network
        compose.add_network(
            &topology.network.name,
            &topology.network.driver,
            topology.network.internal,
        );

        // Add services
        for node in topology.nodes_in_order() {
            let service = self.node_to_service(node, topology);
            compose.add_service(&node.container_name(), service);
        }

        serde_yaml::to_string(&compose).map_err(|e| OrchestratorError::serialization(e.to_string()))
    }

    /// Converts a node spec to a compose service definition.
    fn node_to_service(&self, node: &NodeSpec, topology: &ClusterTopology) -> ComposeService {
        let mut service = ComposeService {
            image: node.image.reference().to_string(),
            container_name: Some(self.container_name(node)),
            hostname: Some(node.name.clone()),
            networks: vec![topology.network.name.clone()],
            ports: node.ports.iter().map(port_to_compose).collect(),
            environment: merge_env(&topology.global_env, &node.environment),
            command: node.command.clone(),
            volumes: node
                .volumes
                .iter()
                .map(|v| format!("{}:{}", v.source, v.target))
                .collect(),
            depends_on: node
                .depends_on
                .iter()
                .filter_map(|id| topology.get_node(id))
                .map(|n| n.container_name())
                .collect(),
            restart: node.restart_policy.as_compose_str().to_string(),
            labels: merge_labels(&topology.global_labels, &node.labels, &self.config.labels),
            ..Default::default()
        };

        // Add resource limits if set
        if let Some(mem) = node.resources.memory_bytes {
            service.deploy = Some(DeployConfig {
                resources: Some(ResourceConfig {
                    limits: Some(ResourceLimits {
                        cpus: node.resources.cpu_limit().map(|c| format!("{:.2}", c)),
                        memory: Some(format!("{}M", mem / (1024 * 1024))),
                    }),
                    reservations: node.resources.memory_reservation_bytes.map(|r| {
                        ResourceLimits {
                            cpus: None,
                            memory: Some(format!("{}M", r / (1024 * 1024))),
                        }
                    }),
                }),
            });
        }

        // Add health check if configured
        if let Some(health) = &node.health_check {
            service.healthcheck = Some(HealthcheckConfig {
                test: health.test.clone(),
                interval: format!("{}s", health.interval_secs),
                timeout: format!("{}s", health.timeout_secs),
                retries: health.retries,
                start_period: format!("{}s", health.start_period_secs),
            });
        }

        service
    }

    /// Generates the full container name with project prefix.
    fn container_name(&self, node: &NodeSpec) -> String {
        format!("{}-{}", self.config.project_name, node.container_name())
    }

    /// Pulls an image if necessary.
    async fn pull_image(&self, image: &str) -> Result<()> {
        info!(image = %image, "Pulling image");

        let options = CreateImageOptions {
            from_image: image,
            ..Default::default()
        };

        let mut stream = self.docker.create_image(Some(options), None, None);

        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(status) = info.status {
                        debug!(status = %status, "Pull progress");
                    }
                }
                Err(e) => {
                    return Err(OrchestratorError::image_pull_failed(image, e.to_string()));
                }
            }
        }

        Ok(())
    }

    /// Creates the cluster network.
    async fn create_network(&self, topology: &ClusterTopology) -> Result<String> {
        let network_name = format!("{}-{}", self.config.project_name, topology.network.name);

        // Check if network already exists
        let filters: HashMap<String, Vec<String>> =
            [("name".to_string(), vec![network_name.clone()])]
                .into_iter()
                .collect();

        let options = ListNetworksOptions { filters };
        let existing = self.docker.list_networks(Some(options)).await?;

        if let Some(network) = existing.first() {
            if let Some(id) = &network.id {
                info!(network = %network_name, id = %id, "Using existing network");
                return Ok(id.clone());
            }
        }

        // Create new network - build labels with &str references
        let mut labels_owned = topology.network.labels.clone();
        labels_owned.insert(
            "mamut.project".to_string(),
            self.config.project_name.clone(),
        );

        // Convert to &str HashMap as required by bollard API
        let labels: HashMap<&str, &str> = labels_owned
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let config = CreateNetworkOptions {
            name: network_name.as_str(),
            driver: topology.network.driver.as_str(),
            internal: topology.network.internal,
            enable_ipv6: topology.network.enable_ipv6,
            labels,
            ..Default::default()
        };

        let response = self.docker.create_network(config).await?;

        // The response.id is a String, not Option<String> in newer bollard versions
        let id = if response.id.is_empty() {
            return Err(OrchestratorError::network_creation_failed(
                &network_name,
                "no ID returned",
            ));
        } else {
            response.id
        };

        info!(network = %network_name, id = %id, "Created network");
        Ok(id)
    }

    /// Creates and starts a container for a node.
    async fn create_container(&self, node: &NodeSpec, topology: &ClusterTopology) -> Result<String> {
        let container_name = self.container_name(node);
        let network_name = format!("{}-{}", self.config.project_name, topology.network.name);

        // Build container config
        let mut env: Vec<String> = topology
            .global_env
            .iter()
            .chain(node.environment.iter())
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        // Add node-specific environment
        env.push(format!("MAMUT_NODE_ID={}", node.id.inner()));
        env.push(format!("MAMUT_NODE_NAME={}", node.name));
        env.push(format!("MAMUT_NODE_ROLE={}", node.role.name()));

        let mut labels = topology.global_labels.clone();
        labels.extend(node.labels.clone());
        labels.extend(self.config.labels.clone());
        labels.insert(
            "mamut.project".to_string(),
            self.config.project_name.clone(),
        );
        labels.insert("mamut.node_id".to_string(), node.id.inner().to_string());
        labels.insert("mamut.node_name".to_string(), node.name.clone());
        labels.insert("mamut.role".to_string(), node.role.name().to_string());

        let exposed_ports: HashMap<String, HashMap<(), ()>> = node
            .ports
            .iter()
            .map(|p| (format!("{}/tcp", p.container_port), HashMap::new()))
            .collect();

        let port_bindings: HashMap<String, Option<Vec<bollard::service::PortBinding>>> = node
            .ports
            .iter()
            .map(|p| {
                let binding = bollard::service::PortBinding {
                    host_ip: p.host_ip.map(|ip| ip.to_string()),
                    host_port: p.host_port.map(|port| port.to_string()),
                };
                (format!("{}/tcp", p.container_port), Some(vec![binding]))
            })
            .collect();

        let host_config = bollard::service::HostConfig {
            port_bindings: Some(port_bindings),
            network_mode: Some(network_name.clone()),
            memory: node.resources.memory_bytes.map(|b| b as i64),
            memory_reservation: node.resources.memory_reservation_bytes.map(|b| b as i64),
            nano_cpus: node.resources.nano_cpus(),
            cpu_shares: node.resources.cpu_shares.map(|s| s as i64),
            pids_limit: node.resources.pids_limit,
            oom_kill_disable: Some(node.resources.oom_kill_disable),
            ..Default::default()
        };

        let config = Config {
            image: Some(node.image.reference().to_string()),
            hostname: Some(node.name.clone()),
            env: Some(env),
            labels: Some(labels),
            exposed_ports: Some(exposed_ports),
            host_config: Some(host_config),
            cmd: node.command.clone(),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: container_name.as_str(),
            platform: None,
        };

        let response = self.docker.create_container(Some(options), config).await?;

        info!(
            container = %container_name,
            id = %response.id,
            "Created container"
        );

        // Start the container
        self.docker
            .start_container(&response.id, None::<StartContainerOptions<String>>)
            .await?;

        info!(container = %container_name, "Started container");

        Ok(response.id)
    }

    /// Waits for a container to be healthy.
    async fn wait_for_healthy(&self, container_id: &str, timeout_secs: u64) -> Result<()> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);

        loop {
            if start.elapsed() > timeout {
                return Err(OrchestratorError::timeout(format!(
                    "container {} to become healthy",
                    container_id
                )));
            }

            let inspect = self.docker.inspect_container(container_id, None).await?;

            if let Some(state) = inspect.state {
                if let Some(health) = state.health {
                    match health.status {
                        Some(bollard::secret::HealthStatusEnum::HEALTHY) => {
                            return Ok(());
                        }
                        Some(bollard::secret::HealthStatusEnum::UNHEALTHY) => {
                            return Err(OrchestratorError::health_check_failed(
                                container_id,
                                "container is unhealthy",
                            ));
                        }
                        _ => {}
                    }
                } else {
                    // No health check configured, container running is good enough
                    if state.running == Some(true) {
                        return Ok(());
                    }
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }

    /// Stops and removes a container.
    async fn remove_container(&self, container_id: &str) -> Result<()> {
        // Stop the container
        let stop_options = StopContainerOptions { t: 10 };
        if let Err(e) = self.docker.stop_container(container_id, Some(stop_options)).await {
            warn!(container = %container_id, error = %e, "Failed to stop container");
        }

        // Remove the container
        let remove_options = RemoveContainerOptions {
            force: true,
            v: self.config.remove_volumes,
            ..Default::default()
        };

        self.docker
            .remove_container(container_id, Some(remove_options))
            .await?;

        info!(container = %container_id, "Removed container");
        Ok(())
    }

    /// Removes the cluster network.
    async fn remove_network(&self, network_id: &str) -> Result<()> {
        self.docker.remove_network(network_id).await?;
        info!(network = %network_id, "Removed network");
        Ok(())
    }

    /// Gets containers with a specific project label.
    #[allow(dead_code)]
    async fn list_project_containers(&self) -> Result<Vec<bollard::secret::ContainerSummary>> {
        let filters: HashMap<String, Vec<String>> = [(
            "label".to_string(),
            vec![format!("mamut.project={}", self.config.project_name)],
        )]
        .into_iter()
        .collect();

        let options = ListContainersOptions {
            all: true,
            filters,
            ..Default::default()
        };

        Ok(self.docker.list_containers(Some(options)).await?)
    }
}

#[async_trait]
impl OrchestrationBackend for DockerComposeBackend {
    async fn deploy(&self, topology: &ClusterTopology) -> Result<DeployedCluster> {
        info!(
            cluster = %topology.name,
            nodes = topology.node_count(),
            "Deploying cluster"
        );

        // Generate compose file if configured
        if self.config.generate_compose_file {
            let yaml = self.generate_compose_yaml(topology)?;
            let compose_path = self.config.work_dir.join(format!(
                "docker-compose-{}.yml",
                topology.name
            ));

            std::fs::create_dir_all(&self.config.work_dir)?;
            std::fs::write(&compose_path, &yaml)?;
            info!(path = %compose_path.display(), "Generated docker-compose.yml");
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let deployment_id = format!("{}-{}", topology.name, now);
        let mut cluster = DeployedCluster::new(topology.clone(), &deployment_id, now);
        cluster.state = DeploymentState::Starting;

        // Create network
        let network_id = self.create_network(topology).await?;
        cluster.network_id = Some(network_id);

        // Pull images if configured
        if self.config.pull_images {
            let images: std::collections::HashSet<_> = topology
                .iter_nodes()
                .map(|n| n.image.reference())
                .collect();

            for image in images {
                self.pull_image(image).await?;
            }
        }

        // Create containers in dependency order
        let mut node_containers = self.node_containers.write().await;

        for node in topology.nodes_in_order() {
            match self.create_container(node, topology).await {
                Ok(container_id) => {
                    cluster.container_ids.insert(node.id, container_id.clone());
                    node_containers.insert(node.id, container_id.clone());

                    // Register with lifecycle manager
                    let handle = ContainerHandle::new(
                        &container_id,
                        node.id,
                        self.container_name(node),
                        now,
                    );
                    self.lifecycle.register(handle).await;
                }
                Err(e) => {
                    error!(node = %node.name, error = %e, "Failed to create container");
                    cluster.state = DeploymentState::Failed {
                        error: e.to_string(),
                    };

                    // Cleanup already created containers
                    drop(node_containers);
                    let _ = self.teardown(&cluster).await;
                    return Err(e);
                }
            }
        }

        // Wait for containers to be healthy
        for (node_id, container_id) in &cluster.container_ids {
            if let Some(node) = topology.get_node(node_id) {
                if node.health_check.is_some() {
                    self.wait_for_healthy(container_id, self.config.timeout_secs)
                        .await?;
                }
            }
        }

        cluster.state = DeploymentState::Running;

        // Store current deployment
        let mut current = self.current_deployment.write().await;
        *current = Some(cluster.clone());

        info!(
            cluster = %topology.name,
            deployment_id = %deployment_id,
            "Cluster deployed successfully"
        );

        Ok(cluster)
    }

    async fn teardown(&self, cluster: &DeployedCluster) -> Result<()> {
        info!(
            cluster = %cluster.topology.name,
            deployment_id = %cluster.deployment_id,
            "Tearing down cluster"
        );

        let mut errors = Vec::new();

        // Remove containers
        for (node_id, container_id) in &cluster.container_ids {
            if let Err(e) = self.remove_container(container_id).await {
                error!(
                    node_id = %node_id.inner(),
                    container = %container_id,
                    error = %e,
                    "Failed to remove container"
                );
                errors.push(e.to_string());
            }

            // Unregister from lifecycle manager
            self.lifecycle.unregister(node_id).await;
        }

        // Remove network
        if let Some(network_id) = &cluster.network_id {
            if let Err(e) = self.remove_network(network_id).await {
                error!(network = %network_id, error = %e, "Failed to remove network");
                errors.push(e.to_string());
            }
        }

        // Clear node containers mapping
        let mut node_containers = self.node_containers.write().await;
        for node_id in cluster.container_ids.keys() {
            node_containers.remove(node_id);
        }

        // Clear current deployment
        let mut current = self.current_deployment.write().await;
        if current.as_ref().map(|c| &c.deployment_id) == Some(&cluster.deployment_id) {
            *current = None;
        }

        if errors.is_empty() {
            info!(cluster = %cluster.topology.name, "Cluster teardown complete");
            Ok(())
        } else {
            Err(OrchestratorError::teardown_failed(errors.join("; ")))
        }
    }

    async fn get_container(&self, node_id: &NodeId) -> Result<ContainerHandle> {
        self.lifecycle
            .get(node_id)
            .await
            .ok_or_else(|| OrchestratorError::container_not_found(node_id.to_string()))
    }

    async fn exec(&self, node_id: &NodeId, cmd: &[&str]) -> Result<ExecOutput> {
        let containers = self.node_containers.read().await;
        let container_id = containers
            .get(node_id)
            .ok_or_else(|| OrchestratorError::container_not_found(node_id.to_string()))?;

        debug!(
            node_id = %node_id.inner(),
            container = %container_id,
            cmd = ?cmd,
            "Executing command"
        );

        let exec_options = CreateExecOptions {
            cmd: Some(cmd.iter().map(|s| s.to_string()).collect()),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };

        let exec = self.docker.create_exec(container_id, exec_options).await?;

        let start_result = self.docker.start_exec(&exec.id, None).await?;

        let mut stdout = String::new();
        let mut stderr = String::new();

        if let StartExecResults::Attached { mut output, .. } = start_result {
            while let Some(result) = output.next().await {
                match result {
                    Ok(bollard::container::LogOutput::StdOut { message }) => {
                        stdout.push_str(&String::from_utf8_lossy(&message));
                    }
                    Ok(bollard::container::LogOutput::StdErr { message }) => {
                        stderr.push_str(&String::from_utf8_lossy(&message));
                    }
                    Ok(_) => {}
                    Err(e) => {
                        return Err(OrchestratorError::exec_failed(
                            container_id.clone(),
                            e.to_string(),
                        ));
                    }
                }
            }
        }

        // Get exit code
        let exec_inspect = self.docker.inspect_exec(&exec.id).await?;
        let exit_code = exec_inspect.exit_code.unwrap_or(-1) as i32;

        Ok(ExecOutput {
            stdout,
            stderr,
            exit_code,
        })
    }

    async fn configure_network(&self, config: &NetworkConfig) -> Result<()> {
        if !config.has_faults() {
            return Ok(());
        }

        // Network fault injection would require iptables/tc rules inside containers
        // This is a basic implementation that logs the intended configuration
        warn!(
            partitions = config.partitions.len(),
            latency_rules = config.latency.len(),
            bandwidth_rules = config.bandwidth.len(),
            packet_loss_rules = config.packet_loss.len(),
            "Network fault injection not fully implemented - would apply rules"
        );

        // For full implementation, you would:
        // 1. Use `tc` (traffic control) for latency/bandwidth/loss
        // 2. Use `iptables` for network partitions
        // 3. Execute these commands inside containers or on the host network namespace

        Ok(())
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            supports_network_faults: false, // Would need tc/iptables setup
            supports_pause: true,
            supports_resource_limits: true,
            supports_health_checks: true,
            supports_exec: true,
            supports_logs: true,
            supports_volumes: true,
            supports_custom_networks: true,
            supports_traffic_capture: false,
            max_containers: 0,
            extensions: HashMap::new(),
        }
    }

    fn name(&self) -> &str {
        "docker-compose"
    }

    async fn is_ready(&self) -> bool {
        self.docker.ping().await.is_ok()
    }

    async fn pause(&self, node_id: &NodeId) -> Result<()> {
        let containers = self.node_containers.read().await;
        let container_id = containers
            .get(node_id)
            .ok_or_else(|| OrchestratorError::container_not_found(node_id.to_string()))?;

        self.docker.pause_container(container_id).await?;
        self.lifecycle
            .update_state(node_id, ContainerState::Paused)
            .await?;

        info!(node_id = %node_id.inner(), "Paused container");
        Ok(())
    }

    async fn unpause(&self, node_id: &NodeId) -> Result<()> {
        let containers = self.node_containers.read().await;
        let container_id = containers
            .get(node_id)
            .ok_or_else(|| OrchestratorError::container_not_found(node_id.to_string()))?;

        self.docker.unpause_container(container_id).await?;
        self.lifecycle
            .update_state(node_id, ContainerState::Running)
            .await?;

        info!(node_id = %node_id.inner(), "Unpaused container");
        Ok(())
    }

    async fn restart(&self, node_id: &NodeId) -> Result<()> {
        let containers = self.node_containers.read().await;
        let container_id = containers
            .get(node_id)
            .ok_or_else(|| OrchestratorError::container_not_found(node_id.to_string()))?;

        self.lifecycle
            .update_state(node_id, ContainerState::Restarting)
            .await?;

        self.docker
            .restart_container(container_id, Some(bollard::container::RestartContainerOptions { t: 10 }))
            .await?;

        self.lifecycle
            .update_state(node_id, ContainerState::Running)
            .await?;

        info!(node_id = %node_id.inner(), "Restarted container");
        Ok(())
    }

    async fn kill(&self, node_id: &NodeId, signal: &str) -> Result<()> {
        let containers = self.node_containers.read().await;
        let container_id = containers
            .get(node_id)
            .ok_or_else(|| OrchestratorError::container_not_found(node_id.to_string()))?;

        let options = bollard::container::KillContainerOptions { signal };
        self.docker.kill_container(container_id, Some(options)).await?;

        info!(node_id = %node_id.inner(), signal = %signal, "Killed container");
        Ok(())
    }

    async fn logs(&self, node_id: &NodeId, tail: Option<usize>) -> Result<String> {
        let containers = self.node_containers.read().await;
        let container_id = containers
            .get(node_id)
            .ok_or_else(|| OrchestratorError::container_not_found(node_id.to_string()))?;

        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            tail: tail.map(|n| n.to_string()).unwrap_or_else(|| "all".to_string()),
            ..Default::default()
        };

        let mut stream = self.docker.logs(container_id, Some(options));
        let mut output = String::new();

        while let Some(result) = stream.next().await {
            match result {
                Ok(log) => {
                    output.push_str(&log.to_string());
                }
                Err(e) => {
                    return Err(OrchestratorError::DockerApi(e));
                }
            }
        }

        Ok(output)
    }
}

// Docker Compose YAML types

#[derive(Debug, Default, Serialize, Deserialize)]
struct ComposeFile {
    version: String,
    services: HashMap<String, ComposeService>,
    networks: HashMap<String, ComposeNetwork>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    volumes: HashMap<String, ComposeVolume>,
}

impl ComposeFile {
    fn new() -> Self {
        Self {
            version: "3.8".to_string(),
            services: HashMap::new(),
            networks: HashMap::new(),
            volumes: HashMap::new(),
        }
    }

    fn add_service(&mut self, name: &str, service: ComposeService) {
        self.services.insert(name.to_string(), service);
    }

    fn add_network(&mut self, name: &str, driver: &str, internal: bool) {
        self.networks.insert(
            name.to_string(),
            ComposeNetwork {
                driver: driver.to_string(),
                internal,
            },
        );
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ComposeService {
    image: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    container_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hostname: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    networks: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    ports: Vec<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    environment: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    volumes: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    depends_on: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    restart: String,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    labels: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deploy: Option<DeployConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    healthcheck: Option<HealthcheckConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ComposeNetwork {
    driver: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    internal: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ComposeVolume {}

#[derive(Debug, Serialize, Deserialize)]
struct DeployConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    resources: Option<ResourceConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResourceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    limits: Option<ResourceLimits>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reservations: Option<ResourceLimits>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResourceLimits {
    #[serde(skip_serializing_if = "Option::is_none")]
    cpus: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct HealthcheckConfig {
    test: Vec<String>,
    interval: String,
    timeout: String,
    retries: u32,
    start_period: String,
}

// Helper functions

fn port_to_compose(port: &PortMapping) -> String {
    match port.host_port {
        Some(hp) => format!("{}:{}", hp, port.container_port),
        None => port.container_port.to_string(),
    }
}

fn merge_env(
    global: &HashMap<String, String>,
    local: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut result = global.clone();
    result.extend(local.clone());
    result
}

fn merge_labels(
    global: &HashMap<String, String>,
    local: &HashMap<String, String>,
    config: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut result = global.clone();
    result.extend(local.clone());
    result.extend(config.clone());
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::ContainerImage;
    use crate::topology::{NetworkSpec, NodeRole, NodeSpec};

    fn create_test_topology() -> ClusterTopology {
        let node1 = NodeSpec::builder(NodeId::new(1))
            .name("redis-1")
            .role(NodeRole::DataNode)
            .image(ContainerImage::new("redis:7-alpine"))
            .env("REDIS_PORT", "6379")
            .build();

        let node2 = NodeSpec::builder(NodeId::new(2))
            .name("redis-2")
            .role(NodeRole::DataNode)
            .image(ContainerImage::new("redis:7-alpine"))
            .depends_on(NodeId::new(1))
            .build();

        ClusterTopology::builder("test-cluster")
            .node(node1)
            .node(node2)
            .network(NetworkSpec::builder("test-network").build())
            .global_env("CLUSTER_NAME", "test")
            .build()
    }

    #[test]
    fn test_compose_yaml_generation() {
        // Verify that default config can be created
        let _config = DockerComposeConfig::default();

        // We can't instantiate DockerComposeBackend without Docker,
        // so test the compose file structure directly
        let topology = create_test_topology();

        let mut compose = ComposeFile::new();
        compose.add_network(
            &topology.network.name,
            &topology.network.driver,
            topology.network.internal,
        );

        let yaml = serde_yaml::to_string(&compose).unwrap();
        assert!(yaml.contains("version:"));
        assert!(yaml.contains("networks:"));
    }

    #[test]
    fn test_docker_compose_config_builder() {
        let config = DockerComposeConfig::builder()
            .project_name("myproject")
            .timeout_secs(60)
            .pull_images(false)
            .build();

        assert_eq!(config.project_name, "myproject");
        assert_eq!(config.timeout_secs, 60);
        assert!(!config.pull_images);
    }

    #[test]
    fn test_port_to_compose() {
        let port = PortMapping::tcp(6379).host_port(16379);
        assert_eq!(port_to_compose(&port), "16379:6379");

        let port = PortMapping::tcp(8080);
        assert_eq!(port_to_compose(&port), "8080");
    }

    #[test]
    fn test_merge_env() {
        let mut global = HashMap::new();
        global.insert("A".to_string(), "1".to_string());
        global.insert("B".to_string(), "2".to_string());

        let mut local = HashMap::new();
        local.insert("B".to_string(), "3".to_string());
        local.insert("C".to_string(), "4".to_string());

        let merged = merge_env(&global, &local);

        assert_eq!(merged.get("A"), Some(&"1".to_string()));
        assert_eq!(merged.get("B"), Some(&"3".to_string())); // Local overrides
        assert_eq!(merged.get("C"), Some(&"4".to_string()));
    }
}
