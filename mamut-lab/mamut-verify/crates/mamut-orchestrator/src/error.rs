//! Error types for the orchestrator crate.
//!
//! This module provides error types for container orchestration operations.

use thiserror::Error;

/// Result type for orchestrator operations.
pub type Result<T> = std::result::Result<T, OrchestratorError>;

/// Errors that can occur during orchestration operations.
#[derive(Debug, Error)]
pub enum OrchestratorError {
    /// Container not found.
    #[error("container not found: {0}")]
    ContainerNotFound(String),

    /// Network not found.
    #[error("network not found: {0}")]
    NetworkNotFound(String),

    /// Image not found.
    #[error("image not found: {0}")]
    ImageNotFound(String),

    /// Failed to pull image.
    #[error("failed to pull image {image}: {reason}")]
    ImagePullFailed {
        /// The image that failed to pull.
        image: String,
        /// The reason for the failure.
        reason: String,
    },

    /// Container creation failed.
    #[error("failed to create container {name}: {reason}")]
    ContainerCreationFailed {
        /// The container name.
        name: String,
        /// The reason for the failure.
        reason: String,
    },

    /// Container start failed.
    #[error("failed to start container {container_id}: {reason}")]
    ContainerStartFailed {
        /// The container ID.
        container_id: String,
        /// The reason for the failure.
        reason: String,
    },

    /// Container stop failed.
    #[error("failed to stop container {container_id}: {reason}")]
    ContainerStopFailed {
        /// The container ID.
        container_id: String,
        /// The reason for the failure.
        reason: String,
    },

    /// Container exec failed.
    #[error("exec failed in container {container_id}: {reason}")]
    ExecFailed {
        /// The container ID.
        container_id: String,
        /// The reason for the failure.
        reason: String,
    },

    /// Network creation failed.
    #[error("failed to create network {name}: {reason}")]
    NetworkCreationFailed {
        /// The network name.
        name: String,
        /// The reason for the failure.
        reason: String,
    },

    /// Network configuration failed.
    #[error("failed to configure network: {0}")]
    NetworkConfigFailed(String),

    /// Deployment failed.
    #[error("deployment failed: {0}")]
    DeploymentFailed(String),

    /// Teardown failed.
    #[error("teardown failed: {0}")]
    TeardownFailed(String),

    /// Health check failed.
    #[error("health check failed for {container_id}: {reason}")]
    HealthCheckFailed {
        /// The container ID.
        container_id: String,
        /// The reason for the failure.
        reason: String,
    },

    /// Timeout waiting for condition.
    #[error("timeout waiting for {condition}")]
    Timeout {
        /// The condition that timed out.
        condition: String,
    },

    /// Backend capability not supported.
    #[error("capability not supported: {0}")]
    CapabilityNotSupported(String),

    /// Docker API error.
    #[error("Docker API error: {0}")]
    DockerApi(#[from] bollard::errors::Error),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Configuration error.
    #[error("configuration error: {0}")]
    Configuration(String),

    /// Backend not connected.
    #[error("backend not connected")]
    NotConnected,

    /// Invalid state transition.
    #[error("invalid state transition from {from} to {to}")]
    InvalidStateTransition {
        /// The starting state.
        from: String,
        /// The target state.
        to: String,
    },

    /// Resource limit exceeded.
    #[error("resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),

    /// Validation error.
    #[error("validation error: {0}")]
    Validation(String),
}

impl OrchestratorError {
    /// Creates a container not found error.
    pub fn container_not_found(id: impl Into<String>) -> Self {
        Self::ContainerNotFound(id.into())
    }

    /// Creates a network not found error.
    pub fn network_not_found(name: impl Into<String>) -> Self {
        Self::NetworkNotFound(name.into())
    }

    /// Creates an image not found error.
    pub fn image_not_found(image: impl Into<String>) -> Self {
        Self::ImageNotFound(image.into())
    }

    /// Creates an image pull failed error.
    pub fn image_pull_failed(image: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ImagePullFailed {
            image: image.into(),
            reason: reason.into(),
        }
    }

    /// Creates a container creation failed error.
    pub fn container_creation_failed(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ContainerCreationFailed {
            name: name.into(),
            reason: reason.into(),
        }
    }

    /// Creates a container start failed error.
    pub fn container_start_failed(id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ContainerStartFailed {
            container_id: id.into(),
            reason: reason.into(),
        }
    }

    /// Creates a container stop failed error.
    pub fn container_stop_failed(id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ContainerStopFailed {
            container_id: id.into(),
            reason: reason.into(),
        }
    }

    /// Creates an exec failed error.
    pub fn exec_failed(id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ExecFailed {
            container_id: id.into(),
            reason: reason.into(),
        }
    }

    /// Creates a network creation failed error.
    pub fn network_creation_failed(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::NetworkCreationFailed {
            name: name.into(),
            reason: reason.into(),
        }
    }

    /// Creates a network config failed error.
    pub fn network_config_failed(reason: impl Into<String>) -> Self {
        Self::NetworkConfigFailed(reason.into())
    }

    /// Creates a deployment failed error.
    pub fn deployment_failed(reason: impl Into<String>) -> Self {
        Self::DeploymentFailed(reason.into())
    }

    /// Creates a teardown failed error.
    pub fn teardown_failed(reason: impl Into<String>) -> Self {
        Self::TeardownFailed(reason.into())
    }

    /// Creates a health check failed error.
    pub fn health_check_failed(id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::HealthCheckFailed {
            container_id: id.into(),
            reason: reason.into(),
        }
    }

    /// Creates a timeout error.
    pub fn timeout(condition: impl Into<String>) -> Self {
        Self::Timeout {
            condition: condition.into(),
        }
    }

    /// Creates a capability not supported error.
    pub fn capability_not_supported(cap: impl Into<String>) -> Self {
        Self::CapabilityNotSupported(cap.into())
    }

    /// Creates a serialization error.
    pub fn serialization(reason: impl Into<String>) -> Self {
        Self::Serialization(reason.into())
    }

    /// Creates a configuration error.
    pub fn configuration(reason: impl Into<String>) -> Self {
        Self::Configuration(reason.into())
    }

    /// Creates an invalid state transition error.
    pub fn invalid_state_transition(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self::InvalidStateTransition {
            from: from.into(),
            to: to.into(),
        }
    }

    /// Creates a resource limit exceeded error.
    pub fn resource_limit_exceeded(reason: impl Into<String>) -> Self {
        Self::ResourceLimitExceeded(reason.into())
    }

    /// Creates a validation error.
    pub fn validation(reason: impl Into<String>) -> Self {
        Self::Validation(reason.into())
    }

    /// Returns true if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Timeout { .. }
                | Self::DockerApi(_)
                | Self::Io(_)
                | Self::ContainerStartFailed { .. }
        )
    }

    /// Returns true if this is a not found error.
    pub fn is_not_found(&self) -> bool {
        matches!(
            self,
            Self::ContainerNotFound(_) | Self::NetworkNotFound(_) | Self::ImageNotFound(_)
        )
    }
}

impl From<serde_json::Error> for OrchestratorError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

impl From<serde_yaml::Error> for OrchestratorError {
    fn from(err: serde_yaml::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = OrchestratorError::container_not_found("abc123");
        assert_eq!(err.to_string(), "container not found: abc123");

        let err = OrchestratorError::timeout("container to start");
        assert_eq!(err.to_string(), "timeout waiting for container to start");
    }

    #[test]
    fn test_error_retryable() {
        assert!(OrchestratorError::timeout("test").is_retryable());
        assert!(!OrchestratorError::configuration("test").is_retryable());
        assert!(!OrchestratorError::validation("test").is_retryable());
    }

    #[test]
    fn test_error_is_not_found() {
        assert!(OrchestratorError::container_not_found("test").is_not_found());
        assert!(OrchestratorError::network_not_found("test").is_not_found());
        assert!(OrchestratorError::image_not_found("test").is_not_found());
        assert!(!OrchestratorError::timeout("test").is_not_found());
    }
}
