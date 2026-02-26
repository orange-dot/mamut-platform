//! Agent configuration.
//!
//! This module provides configuration types for the Mamut agent,
//! supporting loading from files, environment variables, and CLI arguments.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

/// Agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentConfig {
    /// Agent identification.
    pub agent: AgentIdentity,

    /// gRPC server configuration.
    pub server: ServerConfig,

    /// Controller connection configuration.
    pub controller: ControllerConfig,

    /// Logging configuration.
    pub logging: LoggingConfig,

    /// Fault injection configuration.
    pub faults: FaultConfig,

    /// OVC clock configuration.
    pub clock: ClockConfig,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            agent: AgentIdentity::default(),
            server: ServerConfig::default(),
            controller: ControllerConfig::default(),
            logging: LoggingConfig::default(),
            faults: FaultConfig::default(),
            clock: ClockConfig::default(),
        }
    }
}

/// Agent identity configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentIdentity {
    /// Unique node ID for this agent.
    /// If not specified, will be auto-generated or derived from hostname.
    pub node_id: Option<String>,

    /// Human-readable name for this agent.
    pub name: Option<String>,

    /// Labels for agent categorization.
    #[serde(default)]
    pub labels: std::collections::HashMap<String, String>,

    /// Capabilities this agent supports.
    #[serde(default)]
    pub capabilities: Vec<String>,
}

impl Default for AgentIdentity {
    fn default() -> Self {
        Self {
            node_id: None,
            name: None,
            labels: std::collections::HashMap::new(),
            capabilities: vec![
                "execute".to_string(),
                "fault_injection".to_string(),
                "timing_events".to_string(),
            ],
        }
    }
}

/// gRPC server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Address to bind the gRPC server to.
    pub bind_addr: SocketAddr,

    /// Maximum concurrent requests.
    pub max_concurrent_requests: usize,

    /// Request timeout in seconds.
    pub request_timeout_secs: u64,

    /// Whether to enable reflection service.
    pub enable_reflection: bool,

    /// TLS configuration.
    pub tls: Option<TlsConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:50051".parse().unwrap(),
            max_concurrent_requests: 100,
            request_timeout_secs: 30,
            enable_reflection: true,
            tls: None,
        }
    }
}

/// TLS configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Path to the certificate file.
    pub cert_path: PathBuf,

    /// Path to the private key file.
    pub key_path: PathBuf,

    /// Path to the CA certificate for client verification.
    pub ca_cert_path: Option<PathBuf>,

    /// Whether to require client certificates.
    pub require_client_cert: bool,
}

/// Controller connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ControllerConfig {
    /// Controller endpoint URL.
    pub endpoint: String,

    /// Connection timeout in seconds.
    pub connect_timeout_secs: u64,

    /// Request timeout in seconds.
    pub request_timeout_secs: u64,

    /// Heartbeat interval in seconds.
    pub heartbeat_interval_secs: u64,

    /// Maximum retry attempts.
    pub max_retries: u32,

    /// Retry delay in milliseconds.
    pub retry_delay_ms: u64,

    /// Whether to auto-register with controller on startup.
    pub auto_register: bool,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:50052".to_string(),
            connect_timeout_secs: 10,
            request_timeout_secs: 30,
            heartbeat_interval_secs: 30,
            max_retries: 3,
            retry_delay_ms: 500,
            auto_register: true,
        }
    }
}

impl ControllerConfig {
    /// Returns the connect timeout as a Duration.
    pub fn connect_timeout(&self) -> Duration {
        Duration::from_secs(self.connect_timeout_secs)
    }

    /// Returns the request timeout as a Duration.
    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request_timeout_secs)
    }

    /// Returns the heartbeat interval as a Duration.
    pub fn heartbeat_interval(&self) -> Duration {
        Duration::from_secs(self.heartbeat_interval_secs)
    }

    /// Returns the retry delay as a Duration.
    pub fn retry_delay(&self) -> Duration {
        Duration::from_millis(self.retry_delay_ms)
    }
}

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error).
    pub level: String,

    /// Log format (pretty, json, compact).
    pub format: String,

    /// Whether to include timestamps in logs.
    pub include_timestamps: bool,

    /// Whether to include span information.
    pub include_spans: bool,

    /// Log file path (if logging to file).
    pub file_path: Option<PathBuf>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "pretty".to_string(),
            include_timestamps: true,
            include_spans: true,
            file_path: None,
        }
    }
}

/// Fault injection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FaultConfig {
    /// Whether fault injection is enabled.
    pub enabled: bool,

    /// Maximum number of concurrent active faults.
    pub max_active_faults: usize,

    /// Default fault duration in seconds (0 = indefinite).
    pub default_duration_secs: u64,

    /// Whether to allow faults that could corrupt data.
    pub allow_destructive: bool,
}

impl Default for FaultConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_active_faults: 10,
            default_duration_secs: 0,
            allow_destructive: false,
        }
    }
}

/// OVC clock configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ClockConfig {
    /// Maximum allowed uncertainty before forcing resync (nanoseconds).
    pub max_uncertainty_ns: u64,

    /// Sync interval in milliseconds.
    pub sync_interval_ms: u64,

    /// Number of samples for drift estimation.
    pub drift_samples: u32,

    /// Whether to use RTT estimation for sync.
    pub use_rtt_estimation: bool,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            max_uncertainty_ns: 10_000_000, // 10ms
            sync_interval_ms: 1000,          // 1 second
            drift_samples: 10,
            use_rtt_estimation: true,
        }
    }
}

impl AgentConfig {
    /// Loads configuration from a file.
    pub fn from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Merges CLI arguments into the configuration.
    pub fn merge_cli_args(&mut self, args: &super::CliArgs) {
        // Override node ID if specified
        if let Some(ref node_id) = args.node_id {
            self.agent.node_id = Some(node_id.clone());
        }

        // Override bind address if specified
        if let Some(bind_addr) = args.bind_addr {
            self.server.bind_addr = bind_addr;
        }

        // Override controller endpoint if specified
        if let Some(ref endpoint) = args.controller_endpoint {
            self.controller.endpoint = endpoint.clone();
        }

        // Override log level if specified
        if let Some(ref level) = args.log_level {
            self.logging.level = level.clone();
        }
    }

    /// Validates the configuration.
    pub fn validate(&self) -> anyhow::Result<()> {
        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.to_lowercase().as_str()) {
            anyhow::bail!("Invalid log level: {}", self.logging.level);
        }

        // Validate log format
        let valid_formats = ["pretty", "json", "compact"];
        if !valid_formats.contains(&self.logging.format.to_lowercase().as_str()) {
            anyhow::bail!("Invalid log format: {}", self.logging.format);
        }

        // Validate TLS configuration if present
        if let Some(ref tls) = self.server.tls {
            if !tls.cert_path.exists() {
                anyhow::bail!("TLS certificate file not found: {:?}", tls.cert_path);
            }
            if !tls.key_path.exists() {
                anyhow::bail!("TLS key file not found: {:?}", tls.key_path);
            }
            if let Some(ref ca_path) = tls.ca_cert_path {
                if !ca_path.exists() {
                    anyhow::bail!("TLS CA certificate file not found: {:?}", ca_path);
                }
            }
        }

        Ok(())
    }

    /// Returns the node ID, generating one if not configured.
    pub fn node_id(&self) -> String {
        self.agent.node_id.clone().unwrap_or_else(|| {
            // Generate a node ID from hostname and a random suffix
            let hostname = hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string());
            let suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
            format!("{}-{}", hostname, suffix)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AgentConfig::default();

        assert!(config.agent.node_id.is_none());
        assert_eq!(config.server.bind_addr.port(), 50051);
        assert_eq!(config.controller.endpoint, "http://localhost:50052");
        assert_eq!(config.logging.level, "info");
        assert!(config.faults.enabled);
    }

    #[test]
    fn test_config_validation() {
        let mut config = AgentConfig::default();

        // Valid config should pass
        assert!(config.validate().is_ok());

        // Invalid log level should fail
        config.logging.level = "invalid".to_string();
        assert!(config.validate().is_err());

        config.logging.level = "info".to_string();

        // Invalid log format should fail
        config.logging.format = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_node_id_generation() {
        let config = AgentConfig::default();
        let node_id = config.node_id();

        // Should generate a node ID
        assert!(!node_id.is_empty());

        // Should be consistent format (hostname-uuid_prefix)
        assert!(node_id.contains('-'));
    }

    #[test]
    fn test_controller_config_durations() {
        let config = ControllerConfig::default();

        assert_eq!(config.connect_timeout(), Duration::from_secs(10));
        assert_eq!(config.request_timeout(), Duration::from_secs(30));
        assert_eq!(config.heartbeat_interval(), Duration::from_secs(30));
        assert_eq!(config.retry_delay(), Duration::from_millis(500));
    }

    #[test]
    fn test_config_serialization() {
        let config = AgentConfig::default();
        let toml_str = toml::to_string(&config).unwrap();

        // Should be able to deserialize back
        let parsed: AgentConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.server.bind_addr, config.server.bind_addr);
    }
}
