//! Orchestration backend implementations.
//!
//! This module provides the trait definition for orchestration backends
//! and concrete implementations for different container runtimes.

mod docker_compose;
mod r#trait;

pub use docker_compose::{DockerComposeBackend, DockerComposeConfig};
pub use r#trait::{BackendCapabilities, BoxedBackend, ExecOutput, OrchestrationBackend};
