//! Container lifecycle management.
//!
//! This module provides types and functionality for managing the lifecycle
//! of containers, including starting, stopping, health checking, and
//! monitoring container state.

use mamut_core::node::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::Result;

/// Handle to a running container.
///
/// Contains information needed to interact with and manage a container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerHandle {
    /// Container ID assigned by the runtime.
    pub container_id: String,

    /// Node ID this container represents.
    pub node_id: NodeId,

    /// Container name.
    pub name: String,

    /// Current state of the container.
    pub state: ContainerState,

    /// IP address of the container (if available).
    pub ip_address: Option<String>,

    /// Mapped ports (container port -> host port).
    pub port_mappings: HashMap<u16, u16>,

    /// Container creation timestamp (Unix milliseconds).
    pub created_at: u64,

    /// Container start timestamp (Unix milliseconds).
    pub started_at: Option<u64>,
}

impl ContainerHandle {
    /// Creates a new container handle.
    pub fn new(
        container_id: impl Into<String>,
        node_id: NodeId,
        name: impl Into<String>,
        created_at: u64,
    ) -> Self {
        Self {
            container_id: container_id.into(),
            node_id,
            name: name.into(),
            state: ContainerState::Created,
            ip_address: None,
            port_mappings: HashMap::new(),
            created_at,
            started_at: None,
        }
    }

    /// Returns the short container ID (first 12 characters).
    pub fn short_id(&self) -> &str {
        if self.container_id.len() > 12 {
            &self.container_id[..12]
        } else {
            &self.container_id
        }
    }

    /// Returns true if the container is running.
    pub fn is_running(&self) -> bool {
        matches!(self.state, ContainerState::Running)
    }

    /// Returns the host port for a container port.
    pub fn host_port(&self, container_port: u16) -> Option<u16> {
        self.port_mappings.get(&container_port).copied()
    }
}

/// Container state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerState {
    /// Container has been created but not started.
    Created,

    /// Container is starting.
    Starting,

    /// Container is running.
    Running,

    /// Container is paused.
    Paused,

    /// Container is being restarted.
    Restarting,

    /// Container is stopping.
    Stopping,

    /// Container has exited.
    Exited {
        /// Exit code.
        exit_code: i32,
    },

    /// Container is in an error state.
    Error {
        /// Error message.
        message: String,
    },

    /// Container has been removed.
    Removed,
}

impl ContainerState {
    /// Returns true if this is a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Exited { .. } | Self::Removed | Self::Error { .. })
    }

    /// Returns true if the container is in a healthy state.
    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Running | Self::Paused)
    }
}

/// Health status of a container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// No health check configured.
    None,

    /// Health check is starting.
    Starting,

    /// Container is healthy.
    Healthy,

    /// Container is unhealthy.
    Unhealthy {
        /// Number of consecutive failures.
        failures: u32,
        /// Last failure message.
        last_error: Option<String>,
    },
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::None
    }
}

/// Container resource statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContainerStats {
    /// CPU usage percentage.
    pub cpu_percent: f64,

    /// Memory usage in bytes.
    pub memory_bytes: u64,

    /// Memory limit in bytes.
    pub memory_limit_bytes: u64,

    /// Memory usage percentage.
    pub memory_percent: f64,

    /// Network bytes received.
    pub network_rx_bytes: u64,

    /// Network bytes transmitted.
    pub network_tx_bytes: u64,

    /// Block I/O read bytes.
    pub block_read_bytes: u64,

    /// Block I/O write bytes.
    pub block_write_bytes: u64,

    /// Number of PIDs.
    pub pids: u64,

    /// Timestamp of these stats (Unix milliseconds).
    pub timestamp: u64,
}

impl ContainerStats {
    /// Returns the memory usage as a human-readable string.
    pub fn memory_human(&self) -> String {
        humanize_bytes(self.memory_bytes)
    }

    /// Returns the memory limit as a human-readable string.
    pub fn memory_limit_human(&self) -> String {
        humanize_bytes(self.memory_limit_bytes)
    }
}

/// Converts bytes to a human-readable string.
fn humanize_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Container lifecycle event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerEvent {
    /// Node ID of the affected container.
    pub node_id: NodeId,

    /// Container ID.
    pub container_id: String,

    /// Event type.
    pub event_type: ContainerEventType,

    /// Event timestamp (Unix milliseconds).
    pub timestamp: u64,

    /// Additional event details.
    pub details: HashMap<String, String>,
}

impl ContainerEvent {
    /// Creates a new container event.
    pub fn new(
        node_id: NodeId,
        container_id: impl Into<String>,
        event_type: ContainerEventType,
        timestamp: u64,
    ) -> Self {
        Self {
            node_id,
            container_id: container_id.into(),
            event_type,
            timestamp,
            details: HashMap::new(),
        }
    }

    /// Adds a detail to the event.
    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }
}

/// Type of container lifecycle event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerEventType {
    /// Container was created.
    Created,

    /// Container started.
    Started,

    /// Container stopped.
    Stopped,

    /// Container was killed.
    Killed,

    /// Container died (unexpected exit).
    Died {
        /// Exit code.
        exit_code: i32,
    },

    /// Container was paused.
    Paused,

    /// Container was unpaused.
    Unpaused,

    /// Container health status changed.
    HealthChanged {
        /// New health status.
        status: HealthStatus,
    },

    /// Container was removed.
    Removed,

    /// Out of memory event.
    OomKilled,

    /// Container exec completed.
    ExecCompleted {
        /// Exit code.
        exit_code: i32,
    },
}

/// Lifecycle manager for containers.
///
/// Tracks container state and provides lifecycle operations.
pub struct LifecycleManager {
    /// Container handles indexed by node ID.
    containers: Arc<RwLock<HashMap<NodeId, ContainerHandle>>>,

    /// Event history.
    events: Arc<RwLock<Vec<ContainerEvent>>>,

    /// Maximum events to keep in history.
    max_events: usize,
}

impl LifecycleManager {
    /// Creates a new lifecycle manager.
    pub fn new() -> Self {
        Self {
            containers: Arc::new(RwLock::new(HashMap::new())),
            events: Arc::new(RwLock::new(Vec::new())),
            max_events: 10000,
        }
    }

    /// Creates a lifecycle manager with a custom event limit.
    pub fn with_event_limit(max_events: usize) -> Self {
        Self {
            max_events,
            ..Self::new()
        }
    }

    /// Registers a container.
    pub async fn register(&self, handle: ContainerHandle) {
        let mut containers = self.containers.write().await;
        containers.insert(handle.node_id, handle);
    }

    /// Unregisters a container.
    pub async fn unregister(&self, node_id: &NodeId) -> Option<ContainerHandle> {
        let mut containers = self.containers.write().await;
        containers.remove(node_id)
    }

    /// Gets a container handle.
    pub async fn get(&self, node_id: &NodeId) -> Option<ContainerHandle> {
        let containers = self.containers.read().await;
        containers.get(node_id).cloned()
    }

    /// Gets all container handles.
    pub async fn all(&self) -> Vec<ContainerHandle> {
        let containers = self.containers.read().await;
        containers.values().cloned().collect()
    }

    /// Updates a container's state.
    pub async fn update_state(&self, node_id: &NodeId, state: ContainerState) -> Result<()> {
        let mut containers = self.containers.write().await;
        if let Some(handle) = containers.get_mut(node_id) {
            handle.state = state;
            Ok(())
        } else {
            Err(crate::error::OrchestratorError::ContainerNotFound(
                node_id.to_string(),
            ))
        }
    }

    /// Records a container event.
    pub async fn record_event(&self, event: ContainerEvent) {
        let mut events = self.events.write().await;

        // Update container state based on event
        if let Some(new_state) = event.event_type.implied_state() {
            let mut containers = self.containers.write().await;
            if let Some(handle) = containers.get_mut(&event.node_id) {
                handle.state = new_state;
            }
        }

        events.push(event);

        // Trim if necessary
        if events.len() > self.max_events {
            let drain_count = events.len() - self.max_events;
            events.drain(..drain_count);
        }
    }

    /// Gets events for a node.
    pub async fn events_for_node(&self, node_id: &NodeId) -> Vec<ContainerEvent> {
        let events = self.events.read().await;
        events
            .iter()
            .filter(|e| &e.node_id == node_id)
            .cloned()
            .collect()
    }

    /// Gets all events since a timestamp.
    pub async fn events_since(&self, timestamp: u64) -> Vec<ContainerEvent> {
        let events = self.events.read().await;
        events
            .iter()
            .filter(|e| e.timestamp >= timestamp)
            .cloned()
            .collect()
    }

    /// Gets the count of containers by state.
    pub async fn state_counts(&self) -> HashMap<String, usize> {
        let containers = self.containers.read().await;
        let mut counts = HashMap::new();

        for handle in containers.values() {
            let state_name = match &handle.state {
                ContainerState::Created => "created",
                ContainerState::Starting => "starting",
                ContainerState::Running => "running",
                ContainerState::Paused => "paused",
                ContainerState::Restarting => "restarting",
                ContainerState::Stopping => "stopping",
                ContainerState::Exited { .. } => "exited",
                ContainerState::Error { .. } => "error",
                ContainerState::Removed => "removed",
            };

            *counts.entry(state_name.to_string()).or_insert(0) += 1;
        }

        counts
    }

    /// Clears all containers and events.
    pub async fn clear(&self) {
        let mut containers = self.containers.write().await;
        let mut events = self.events.write().await;
        containers.clear();
        events.clear();
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ContainerEventType {
    /// Returns the implied container state from this event type.
    fn implied_state(&self) -> Option<ContainerState> {
        match self {
            Self::Created => Some(ContainerState::Created),
            Self::Started => Some(ContainerState::Running),
            Self::Stopped => Some(ContainerState::Exited { exit_code: 0 }),
            Self::Killed => Some(ContainerState::Exited { exit_code: 137 }),
            Self::Died { exit_code } => Some(ContainerState::Exited {
                exit_code: *exit_code,
            }),
            Self::Paused => Some(ContainerState::Paused),
            Self::Unpaused => Some(ContainerState::Running),
            Self::Removed => Some(ContainerState::Removed),
            Self::OomKilled => Some(ContainerState::Exited { exit_code: 137 }),
            Self::HealthChanged { .. } | Self::ExecCompleted { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_handle() {
        let handle = ContainerHandle::new(
            "abc123def456789012345678901234567890123456789012345678901234",
            NodeId::new(1),
            "test-container",
            1000,
        );

        assert_eq!(handle.short_id(), "abc123def456");
        assert!(!handle.is_running());
    }

    #[test]
    fn test_container_state() {
        assert!(!ContainerState::Created.is_terminal());
        assert!(!ContainerState::Running.is_terminal());
        assert!(ContainerState::Exited { exit_code: 0 }.is_terminal());
        assert!(
            ContainerState::Error {
                message: "test".to_string()
            }
            .is_terminal()
        );

        assert!(ContainerState::Running.is_healthy());
        assert!(ContainerState::Paused.is_healthy());
        assert!(!ContainerState::Exited { exit_code: 0 }.is_healthy());
    }

    #[test]
    fn test_container_stats_humanize() {
        let stats = ContainerStats {
            memory_bytes: 1024 * 1024 * 512,
            memory_limit_bytes: 1024 * 1024 * 1024,
            ..Default::default()
        };

        assert_eq!(stats.memory_human(), "512.00 MB");
        assert_eq!(stats.memory_limit_human(), "1.00 GB");
    }

    #[test]
    fn test_humanize_bytes() {
        assert_eq!(humanize_bytes(500), "500 B");
        assert_eq!(humanize_bytes(1024), "1.00 KB");
        assert_eq!(humanize_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(humanize_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[tokio::test]
    async fn test_lifecycle_manager() {
        let manager = LifecycleManager::new();

        let handle = ContainerHandle::new("container-1", NodeId::new(1), "node-1", 1000);

        manager.register(handle).await;

        let retrieved = manager.get(&NodeId::new(1)).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "node-1");

        manager
            .update_state(&NodeId::new(1), ContainerState::Running)
            .await
            .unwrap();

        let updated = manager.get(&NodeId::new(1)).await.unwrap();
        assert!(updated.is_running());
    }

    #[tokio::test]
    async fn test_lifecycle_manager_events() {
        let manager = LifecycleManager::with_event_limit(10);

        let handle = ContainerHandle::new("container-1", NodeId::new(1), "node-1", 1000);
        manager.register(handle).await;

        for i in 0..15 {
            let event = ContainerEvent::new(
                NodeId::new(1),
                "container-1",
                ContainerEventType::Started,
                1000 + i,
            );
            manager.record_event(event).await;
        }

        // Should be trimmed to 10 events
        let events = manager.events_for_node(&NodeId::new(1)).await;
        assert_eq!(events.len(), 10);
    }
}
