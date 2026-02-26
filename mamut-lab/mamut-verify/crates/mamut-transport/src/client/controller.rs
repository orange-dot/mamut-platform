//! Client for agents to communicate with the controller.
//!
//! This module provides the `ControllerClient` which agents use to
//! report status, send timing events, and receive coordination signals.

use crate::error::{TransportError, TransportResult};
use crate::proto::mamut::TimingEvent;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tonic::transport::{Channel, Endpoint};
use tracing::{debug, error, info, instrument, warn};

/// Configuration for the controller client.
#[derive(Debug, Clone)]
pub struct ControllerClientConfig {
    /// The controller endpoint URL.
    pub endpoint: String,

    /// Connection timeout.
    pub connect_timeout: Duration,

    /// Request timeout.
    pub request_timeout: Duration,

    /// Whether to use TLS.
    pub use_tls: bool,

    /// Number of retry attempts for failed connections.
    pub max_retries: u32,

    /// Delay between retry attempts.
    pub retry_delay: Duration,

    /// Keep-alive interval.
    pub keep_alive_interval: Duration,

    /// Keep-alive timeout.
    pub keep_alive_timeout: Duration,
}

impl Default for ControllerClientConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:50052".to_string(),
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
            use_tls: false,
            max_retries: 3,
            retry_delay: Duration::from_millis(500),
            keep_alive_interval: Duration::from_secs(30),
            keep_alive_timeout: Duration::from_secs(10),
        }
    }
}

/// Client state for tracking connection status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientState {
    /// Client is disconnected.
    Disconnected,
    /// Client is connecting.
    Connecting,
    /// Client is connected and ready.
    Connected,
    /// Client is reconnecting after a failure.
    Reconnecting,
    /// Client has been closed.
    Closed,
}

/// Client for agents to communicate with the controller.
///
/// This client maintains a connection to the controller and provides
/// methods for reporting agent status, sending timing events, and
/// receiving coordination signals.
///
/// # Example
///
/// ```ignore
/// use mamut_transport::client::{ControllerClient, ControllerClientConfig};
///
/// let config = ControllerClientConfig {
///     endpoint: "http://controller:50052".to_string(),
///     ..Default::default()
/// };
///
/// let client = ControllerClient::connect(config).await?;
///
/// // Report agent registration
/// client.register(node_id, capabilities).await?;
///
/// // Send timing events
/// client.send_timing_event(event).await?;
/// ```
pub struct ControllerClient {
    config: ControllerClientConfig,
    channel: Arc<RwLock<Option<Channel>>>,
    state: Arc<RwLock<ClientState>>,
    node_id: Arc<RwLock<Option<String>>>,
}

impl ControllerClient {
    /// Creates a new controller client with the given configuration.
    ///
    /// This does not immediately connect; call `connect()` to establish
    /// the connection.
    pub fn new(config: ControllerClientConfig) -> Self {
        Self {
            config,
            channel: Arc::new(RwLock::new(None)),
            state: Arc::new(RwLock::new(ClientState::Disconnected)),
            node_id: Arc::new(RwLock::new(None)),
        }
    }

    /// Connects to the controller.
    ///
    /// This method will retry the connection according to the configured
    /// retry policy.
    #[instrument(skip(self), fields(endpoint = %self.config.endpoint))]
    pub async fn connect(&self) -> TransportResult<()> {
        let mut state = self.state.write().await;
        if *state == ClientState::Connected {
            debug!("Already connected to controller");
            return Ok(());
        }

        *state = ClientState::Connecting;
        drop(state);

        let mut last_error = None;
        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                warn!(
                    attempt = attempt,
                    max_retries = self.config.max_retries,
                    "Retrying connection to controller"
                );
                tokio::time::sleep(self.config.retry_delay).await;
            }

            match self.try_connect().await {
                Ok(channel) => {
                    let mut ch = self.channel.write().await;
                    *ch = Some(channel);
                    drop(ch);

                    let mut state = self.state.write().await;
                    *state = ClientState::Connected;
                    info!("Connected to controller at {}", self.config.endpoint);
                    return Ok(());
                }
                Err(e) => {
                    error!(
                        attempt = attempt,
                        error = %e,
                        "Failed to connect to controller"
                    );
                    last_error = Some(e);
                }
            }
        }

        let mut state = self.state.write().await;
        *state = ClientState::Disconnected;

        Err(last_error.unwrap_or_else(|| {
            TransportError::connection_failed("max retries exceeded")
        }))
    }

    async fn try_connect(&self) -> TransportResult<Channel> {
        let endpoint = Endpoint::from_shared(self.config.endpoint.clone())
            .map_err(|e| TransportError::connection_failed(e.to_string()))?
            .connect_timeout(self.config.connect_timeout)
            .timeout(self.config.request_timeout)
            .tcp_keepalive(Some(self.config.keep_alive_interval));

        let channel = endpoint.connect().await?;
        Ok(channel)
    }

    /// Disconnects from the controller.
    #[instrument(skip(self))]
    pub async fn disconnect(&self) {
        let mut state = self.state.write().await;
        if *state == ClientState::Closed {
            return;
        }

        *state = ClientState::Disconnected;
        drop(state);

        let mut channel = self.channel.write().await;
        *channel = None;

        info!("Disconnected from controller");
    }

    /// Closes the client permanently.
    #[instrument(skip(self))]
    pub async fn close(&self) {
        let mut state = self.state.write().await;
        *state = ClientState::Closed;
        drop(state);

        let mut channel = self.channel.write().await;
        *channel = None;

        info!("Controller client closed");
    }

    /// Returns the current client state.
    pub async fn state(&self) -> ClientState {
        *self.state.read().await
    }

    /// Returns whether the client is connected.
    pub async fn is_connected(&self) -> bool {
        *self.state.read().await == ClientState::Connected
    }

    /// Sets the node ID for this agent.
    pub async fn set_node_id(&self, node_id: String) {
        let mut id = self.node_id.write().await;
        *id = Some(node_id);
    }

    /// Gets the node ID for this agent.
    pub async fn node_id(&self) -> Option<String> {
        self.node_id.read().await.clone()
    }

    /// Gets the underlying channel, if connected.
    ///
    /// This can be used to create gRPC client stubs for custom services.
    pub async fn channel(&self) -> Option<Channel> {
        self.channel.read().await.clone()
    }

    /// Ensures the client is connected, reconnecting if necessary.
    async fn ensure_connected(&self) -> TransportResult<Channel> {
        let state = self.state.read().await;
        if *state == ClientState::Closed {
            return Err(TransportError::ShuttingDown);
        }
        drop(state);

        // Check if we have a valid channel
        {
            let channel = self.channel.read().await;
            if let Some(ch) = channel.as_ref() {
                return Ok(ch.clone());
            }
        }

        // Need to reconnect
        {
            let mut state = self.state.write().await;
            *state = ClientState::Reconnecting;
        }

        self.connect().await?;

        let channel = self.channel.read().await;
        channel
            .clone()
            .ok_or_else(|| TransportError::connection_failed("failed to get channel after connect"))
    }

    /// Sends a timing event to the controller.
    ///
    /// This is used to report operation timing, fault events, and other
    /// timing-related information to the controller for analysis.
    #[instrument(skip(self, event), fields(event_id = %event.event_id))]
    pub async fn send_timing_event(&self, event: TimingEvent) -> TransportResult<()> {
        let _channel = self.ensure_connected().await?;

        // TODO: Implement when ControllerService is defined in proto
        // For now, just log the event
        debug!(
            event_id = %event.event_id,
            event_type = ?event.event_type,
            node_id = %event.node_id,
            "Would send timing event to controller"
        );

        Ok(())
    }

    /// Reports that the agent is alive and healthy.
    ///
    /// This should be called periodically to maintain the agent's
    /// registration with the controller.
    #[instrument(skip(self))]
    pub async fn heartbeat(&self) -> TransportResult<()> {
        let _channel = self.ensure_connected().await?;

        // TODO: Implement when ControllerService is defined in proto
        debug!("Would send heartbeat to controller");

        Ok(())
    }

    /// Returns the client configuration.
    pub fn config(&self) -> &ControllerClientConfig {
        &self.config
    }
}

impl Drop for ControllerClient {
    fn drop(&mut self) {
        // Note: We can't do async cleanup in Drop, so the connection
        // will be cleaned up by the channel's own Drop implementation.
        debug!("ControllerClient dropped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let config = ControllerClientConfig::default();
        let client = ControllerClient::new(config);

        assert_eq!(client.state().await, ClientState::Disconnected);
        assert!(!client.is_connected().await);
    }

    #[tokio::test]
    async fn test_client_state_transitions() {
        let config = ControllerClientConfig::default();
        let client = ControllerClient::new(config);

        // Set node ID
        client.set_node_id("test-node-1".to_string()).await;
        assert_eq!(client.node_id().await, Some("test-node-1".to_string()));

        // Close the client
        client.close().await;
        assert_eq!(client.state().await, ClientState::Closed);
    }

    #[tokio::test]
    async fn test_closed_client_rejects_operations() {
        let config = ControllerClientConfig::default();
        let client = ControllerClient::new(config);

        client.close().await;

        let result = client.heartbeat().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TransportError::ShuttingDown));
    }

    #[test]
    fn test_config_defaults() {
        let config = ControllerClientConfig::default();

        assert_eq!(config.endpoint, "http://localhost:50052");
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.request_timeout, Duration::from_secs(30));
        assert!(!config.use_tls);
        assert_eq!(config.max_retries, 3);
    }
}
