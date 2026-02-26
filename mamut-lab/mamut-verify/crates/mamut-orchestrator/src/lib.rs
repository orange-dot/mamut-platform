//! Container orchestration for distributed system verification.
//!
//! This crate provides infrastructure for deploying, managing, and tearing down
//! distributed systems during verification testing. It supports multiple
//! container orchestration backends (Docker, Docker Compose, etc.) through a
//! common trait interface.
//!
//! # Overview
//!
//! The orchestrator handles:
//! - **Cluster deployment**: Creating networks, pulling images, and starting containers
//! - **Container lifecycle**: Starting, stopping, pausing, and monitoring containers
//! - **Command execution**: Running commands inside containers during tests
//! - **Network configuration**: Configuring network conditions for fault injection
//! - **Teardown**: Cleaning up all resources after tests complete
//!
//! # Architecture
//!
//! The crate is organized into several modules:
//!
//! - [`backend`]: Orchestration backend trait and implementations
//! - [`topology`]: Cluster topology definitions (nodes, networks, configurations)
//! - [`container`]: Container specifications and lifecycle management
//! - [`error`]: Error types for orchestration operations
//!
//! # Example
//!
//! ```ignore
//! use mamut_orchestrator::{
//!     backend::{DockerComposeBackend, OrchestrationBackend},
//!     container::ContainerImage,
//!     topology::{ClusterTopology, NodeSpec, NodeRole, NetworkSpec},
//! };
//! use mamut_core::node::NodeId;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Define cluster topology
//!     let topology = ClusterTopology::builder("my-cluster")
//!         .node(
//!             NodeSpec::builder(NodeId::new(1))
//!                 .name("node-1")
//!                 .role(NodeRole::DataNode)
//!                 .image(ContainerImage::new("redis:7-alpine"))
//!                 .build()
//!         )
//!         .node(
//!             NodeSpec::builder(NodeId::new(2))
//!                 .name("node-2")
//!                 .role(NodeRole::DataNode)
//!                 .image(ContainerImage::new("redis:7-alpine"))
//!                 .depends_on(NodeId::new(1))
//!                 .build()
//!         )
//!         .network(NetworkSpec::builder("cluster-net").build())
//!         .build();
//!
//!     // Create backend and deploy
//!     let backend = DockerComposeBackend::new().await?;
//!     let cluster = backend.deploy(&topology).await?;
//!
//!     // Execute commands
//!     let output = backend.exec(&NodeId::new(1), &["redis-cli", "PING"]).await?;
//!     println!("Response: {}", output.stdout);
//!
//!     // Teardown
//!     backend.teardown(&cluster).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Backends
//!
//! Currently supported backends:
//!
//! - [`DockerComposeBackend`](backend::DockerComposeBackend): Uses the Docker API
//!   directly via bollard, with docker-compose.yml generation for reproducibility.
//!
//! # Docker Compose Generation
//!
//! The [`DockerComposeBackend`](backend::DockerComposeBackend) can generate
//! `docker-compose.yml` files from cluster topologies. This is useful for:
//!
//! - Debugging failed test deployments manually
//! - Reproducing test environments outside the test harness
//! - Documenting the exact container configuration used
//!
//! ```ignore
//! use mamut_orchestrator::backend::{DockerComposeBackend, DockerComposeConfig};
//!
//! let backend = DockerComposeBackend::with_config(
//!     DockerComposeConfig::builder()
//!         .project_name("my-test")
//!         .generate_compose_file(true)
//!         .work_dir("/tmp/compose-files")
//!         .build()
//! ).await?;
//!
//! let cluster = backend.deploy(&topology).await?;
//! // docker-compose.yml is now at /tmp/compose-files/docker-compose-my-cluster.yml
//! ```

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod backend;
pub mod container;
pub mod error;
pub mod topology;

// Re-export commonly used types at the crate root
pub use backend::{BackendCapabilities, DockerComposeBackend, DockerComposeConfig, ExecOutput, OrchestrationBackend};
pub use container::{ContainerHandle, ContainerImage, ContainerState, ResourceLimits};
pub use error::{OrchestratorError, Result};
pub use topology::{ClusterTopology, DeployedCluster, NetworkConfig, NetworkSpec, NodeRole, NodeSpec};
