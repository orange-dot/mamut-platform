//! Mamut Agent - Distributed agent for executing test operations.
//!
//! The Mamut Agent runs on each node in the distributed system under test,
//! receiving commands from the controller to execute operations, inject faults,
//! and report timing information.
//!
//! # Usage
//!
//! ```bash
//! # Start with default configuration
//! mamut-agent
//!
//! # Start with a configuration file
//! mamut-agent --config /path/to/config.toml
//!
//! # Override specific options
//! mamut-agent --bind 0.0.0.0:50051 --controller http://controller:50052
//! ```
//!
//! # Architecture
//!
//! The agent consists of:
//! - gRPC server for receiving controller commands
//! - Controller client for reporting status and events
//! - Fault injection subsystem for applying local faults
//! - OVC clock synchronization
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                        Mamut Agent                          │
//! ├─────────────────────────────────────────────────────────────┤
//! │  ┌─────────────────┐         ┌─────────────────────────┐   │
//! │  │   gRPC Server   │◄────────│   Controller Client     │   │
//! │  │  (from ctrl)    │         │   (to controller)       │   │
//! │  └────────┬────────┘         └─────────────────────────┘   │
//! │           │                                                 │
//! │  ┌────────▼────────┐  ┌────────────────┐  ┌─────────────┐  │
//! │  │ Operation Exec  │  │ Fault Injector │  │  OVC Clock  │  │
//! │  └─────────────────┘  └────────────────┘  └─────────────┘  │
//! │                              │                              │
//! │  ┌────────────────────────────────────────────────────┐    │
//! │  │           System Under Test (SUT)                   │    │
//! │  └────────────────────────────────────────────────────┘    │
//! └─────────────────────────────────────────────────────────────┘
//! ```

mod config;

use anyhow::{Context, Result};
use clap::Parser;
use config::AgentConfig;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::{broadcast, oneshot, RwLock};
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// CLI arguments for the Mamut Agent.
#[derive(Parser, Debug)]
#[command(
    name = "mamut-agent",
    about = "Distributed agent for executing test operations on target systems",
    version,
    author
)]
pub struct CliArgs {
    /// Path to the configuration file.
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Node ID for this agent.
    #[arg(long, value_name = "ID")]
    node_id: Option<String>,

    /// Address to bind the gRPC server to.
    #[arg(short, long, value_name = "ADDR")]
    bind_addr: Option<SocketAddr>,

    /// Controller endpoint URL.
    #[arg(long, value_name = "URL")]
    controller_endpoint: Option<String>,

    /// Log level (trace, debug, info, warn, error).
    #[arg(short, long, value_name = "LEVEL")]
    log_level: Option<String>,

    /// Enable JSON log output.
    #[arg(long)]
    json_logs: bool,

    /// Skip controller registration on startup.
    #[arg(long)]
    no_register: bool,

    /// Run in standalone mode (no controller connection).
    #[arg(long)]
    standalone: bool,

    /// Print the default configuration and exit.
    #[arg(long)]
    print_config: bool,
}

/// Application state for the agent.
struct AgentApp {
    config: AgentConfig,
    node_id: String,
    shutdown_tx: broadcast::Sender<()>,
    state: Arc<RwLock<AgentState>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgentState {
    Starting,
    Running,
    ShuttingDown,
    Stopped,
}

impl AgentApp {
    fn new(config: AgentConfig) -> Self {
        let node_id = config.node_id();
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            config,
            node_id,
            shutdown_tx,
            state: Arc::new(RwLock::new(AgentState::Starting)),
        }
    }

    async fn run(&self) -> Result<()> {
        info!(node_id = %self.node_id, "Starting Mamut Agent");

        // Update state
        {
            let mut state = self.state.write().await;
            *state = AgentState::Running;
        }

        // Create the agent service handler
        let handler = DefaultAgentHandler::new(
            self.node_id.clone(),
            self.config.clone(),
            self.shutdown_tx.subscribe(),
        );

        // Start the gRPC server
        let server = mamut_transport::AgentGrpcServer::new(handler);
        let bind_addr = self.config.server.bind_addr;

        info!(bind_addr = %bind_addr, "Starting gRPC server");

        // Create shutdown signal receiver
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        // Spawn the server
        let server_handle = tokio::spawn(async move {
            server.serve(bind_addr).await
        });

        // Wait for shutdown signal or server error
        tokio::select! {
            result = server_handle => {
                match result {
                    Ok(Ok(())) => {
                        info!("gRPC server stopped");
                    }
                    Ok(Err(e)) => {
                        error!(error = %e, "gRPC server error");
                        return Err(e.into());
                    }
                    Err(e) => {
                        error!(error = %e, "Server task panicked");
                        return Err(anyhow::anyhow!("Server task panicked: {}", e));
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                info!("Received shutdown signal");
            }
        }

        // Update state
        {
            let mut state = self.state.write().await;
            *state = AgentState::Stopped;
        }

        Ok(())
    }

    async fn shutdown(&self) {
        let mut state = self.state.write().await;
        if *state == AgentState::ShuttingDown || *state == AgentState::Stopped {
            return;
        }

        *state = AgentState::ShuttingDown;
        drop(state);

        info!("Initiating graceful shutdown");

        // Send shutdown signal
        let _ = self.shutdown_tx.send(());
    }
}

/// Default implementation of the agent service handler.
struct DefaultAgentHandler {
    node_id: String,
    config: AgentConfig,
    shutdown_rx: broadcast::Receiver<()>,
    timing_tx: broadcast::Sender<mamut_transport::TimingEvent>,
    active_faults: Arc<RwLock<std::collections::HashMap<String, ActiveFault>>>,
}

struct ActiveFault {
    fault_id: String,
    fault_type: i32,
    applied_at: std::time::Instant,
}

impl DefaultAgentHandler {
    fn new(
        node_id: String,
        config: AgentConfig,
        shutdown_rx: broadcast::Receiver<()>,
    ) -> Self {
        let (timing_tx, _) = broadcast::channel(1000);

        Self {
            node_id,
            config,
            shutdown_rx,
            timing_tx,
            active_faults: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    fn current_ovc(&self) -> mamut_transport::Ovc {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i64;

        mamut_transport::Ovc {
            controller_time: now,
            observed_time: now,
            logical_time: 0,
            uncertainty: self.config.clock.max_uncertainty_ns,
        }
    }
}

#[async_trait::async_trait]
impl mamut_transport::server::AgentServiceHandler for DefaultAgentHandler {
    async fn execute(
        &self,
        request: mamut_transport::ExecuteRequest,
    ) -> Result<mamut_transport::ExecuteResponse, mamut_transport::TransportError> {
        let received_at = self.current_ovc();

        // TODO: Actually execute the operation against the SUT
        // For now, just return a successful response
        let outcome = mamut_transport::OperationOutcome {
            result: Some(mamut_transport::operation_outcome::Result::OkValue(
                mamut_transport::OkValue {
                    value: vec![],
                    modified: false,
                    version: 0,
                    metadata: std::collections::HashMap::new(),
                },
            )),
        };

        let completed_at = self.current_ovc();

        Ok(mamut_transport::ExecuteResponse {
            request_id: request.request_id,
            received_at: Some(received_at),
            completed_at: Some(completed_at),
            outcome: Some(outcome),
            timing: if request.record_timing {
                Some(mamut_transport::TimingBreakdown {
                    queue_time_ns: 0,
                    execution_time_ns: 0,
                    network_rtt_ns: 0,
                    coordination_time_ns: 0,
                    storage_time_ns: 0,
                })
            } else {
                None
            },
        })
    }

    async fn apply_fault(
        &self,
        request: mamut_transport::ApplyFaultRequest,
    ) -> Result<mamut_transport::ApplyFaultResponse, mamut_transport::TransportError> {
        if !self.config.faults.enabled {
            return Ok(mamut_transport::ApplyFaultResponse {
                fault_id: request.fault_id,
                applied: false,
                active_at: None,
                error: "Fault injection is disabled".to_string(),
            });
        }

        let active_faults = self.active_faults.read().await;
        if active_faults.len() >= self.config.faults.max_active_faults {
            return Ok(mamut_transport::ApplyFaultResponse {
                fault_id: request.fault_id,
                applied: false,
                active_at: None,
                error: "Maximum active faults reached".to_string(),
            });
        }
        drop(active_faults);

        // TODO: Actually apply the fault
        // For now, just record it
        let fault = ActiveFault {
            fault_id: request.fault_id.clone(),
            fault_type: request.fault_type,
            applied_at: std::time::Instant::now(),
        };

        let mut active_faults = self.active_faults.write().await;
        active_faults.insert(request.fault_id.clone(), fault);

        let active_at = self.current_ovc();

        Ok(mamut_transport::ApplyFaultResponse {
            fault_id: request.fault_id,
            applied: true,
            active_at: Some(active_at),
            error: String::new(),
        })
    }

    async fn recover_fault(
        &self,
        request: mamut_transport::RecoverFaultRequest,
    ) -> Result<mamut_transport::RecoverFaultResponse, mamut_transport::TransportError> {
        let mut active_faults = self.active_faults.write().await;

        if active_faults.remove(&request.fault_id).is_some() {
            let recovered_at = self.current_ovc();

            Ok(mamut_transport::RecoverFaultResponse {
                fault_id: request.fault_id,
                recovered: true,
                recovered_at: Some(recovered_at),
                error: String::new(),
            })
        } else {
            Ok(mamut_transport::RecoverFaultResponse {
                fault_id: request.fault_id,
                recovered: false,
                recovered_at: None,
                error: "Fault not found".to_string(),
            })
        }
    }

    async fn health(
        &self,
        request: mamut_transport::HealthRequest,
    ) -> Result<mamut_transport::HealthResponse, mamut_transport::TransportError> {
        let clock_state = mamut_transport::ClockState {
            current: Some(self.current_ovc()),
            max_drift: 0,
            sync_count: 0,
            synchronized: true,
        };

        let active_faults = self.active_faults.read().await;
        let active_fault_ids: Vec<String> = active_faults.keys().cloned().collect();

        let mut details = std::collections::HashMap::new();
        details.insert("node_id".to_string(), self.node_id.clone());

        if request.deep_check {
            // TODO: Add deep health check logic
            details.insert("deep_check".to_string(), "passed".to_string());
        }

        Ok(mamut_transport::HealthResponse {
            status: mamut_transport::health_response::Status::Healthy.into(),
            clock_state: Some(clock_state),
            active_fault_ids,
            resources: None,
            details,
        })
    }

    async fn sync_clock(
        &self,
        request: mamut_transport::SyncClockRequest,
    ) -> Result<mamut_transport::SyncClockResponse, mamut_transport::TransportError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i64;

        let agent_clock_before = self.current_ovc();

        // Calculate offset estimate
        let offset_estimate_ns = if let Some(ref controller_clock) = request.controller_clock {
            now - controller_clock.controller_time
        } else {
            0
        };

        let agent_clock_after = self.current_ovc();

        Ok(mamut_transport::SyncClockResponse {
            agent_clock_before: Some(agent_clock_before),
            agent_clock_after: Some(agent_clock_after),
            sequence: request.sequence,
            receive_time_ns: now,
            send_time_ns: now,
            offset_estimate_ns,
        })
    }

    async fn subscribe_timing_events(
        &self,
        _request: mamut_transport::StreamTimingRequest,
    ) -> Result<broadcast::Receiver<mamut_transport::TimingEvent>, mamut_transport::TransportError>
    {
        Ok(self.timing_tx.subscribe())
    }
}

/// Initialize tracing/logging.
fn init_tracing(config: &config::LoggingConfig, json_logs: bool) -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.level))
        .context("Failed to parse log filter")?;

    let format = if json_logs || config.format == "json" {
        "json"
    } else {
        &config.format
    };

    match format {
        "json" => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().json())
                .try_init()
                .map_err(|e| anyhow::anyhow!("Failed to initialize tracing: {}", e))?;
        }
        "compact" => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().compact())
                .try_init()
                .map_err(|e| anyhow::anyhow!("Failed to initialize tracing: {}", e))?;
        }
        _ => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().pretty())
                .try_init()
                .map_err(|e| anyhow::anyhow!("Failed to initialize tracing: {}", e))?;
        }
    }

    Ok(())
}

/// Setup signal handlers for graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let args = CliArgs::parse();

    // Handle --print-config
    if args.print_config {
        let config = AgentConfig::default();
        println!("{}", toml::to_string_pretty(&config)?);
        return Ok(());
    }

    // Load configuration
    let mut config = if let Some(ref config_path) = args.config {
        AgentConfig::from_file(config_path)
            .with_context(|| format!("Failed to load config from {:?}", config_path))?
    } else {
        AgentConfig::default()
    };

    // Merge CLI arguments
    config.merge_cli_args(&args);

    // Validate configuration
    config.validate().context("Invalid configuration")?;

    // Initialize tracing
    init_tracing(&config.logging, args.json_logs)?;

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "Mamut Agent starting"
    );

    // Create and run the application
    let app = AgentApp::new(config);

    // Setup shutdown handler
    let shutdown_tx = app.shutdown_tx.clone();
    tokio::spawn(async move {
        shutdown_signal().await;
        info!("Shutdown signal received");
        let _ = shutdown_tx.send(());
    });

    // Run the application
    if let Err(e) = app.run().await {
        error!(error = %e, "Agent failed");
        return Err(e);
    }

    info!("Mamut Agent stopped");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_args_parsing() {
        let args = CliArgs::parse_from(["mamut-agent"]);
        assert!(args.config.is_none());
        assert!(args.node_id.is_none());
        assert!(!args.json_logs);
    }

    #[test]
    fn test_cli_args_with_options() {
        let args = CliArgs::parse_from([
            "mamut-agent",
            "--node-id",
            "test-node",
            "--bind-addr",
            "127.0.0.1:50051",
            "--log-level",
            "debug",
            "--json-logs",
        ]);

        assert_eq!(args.node_id, Some("test-node".to_string()));
        assert_eq!(
            args.bind_addr,
            Some("127.0.0.1:50051".parse().unwrap())
        );
        assert_eq!(args.log_level, Some("debug".to_string()));
        assert!(args.json_logs);
    }

    #[test]
    fn test_config_merge() {
        let mut config = AgentConfig::default();
        let args = CliArgs::parse_from([
            "mamut-agent",
            "--node-id",
            "override-node",
            "--controller-endpoint",
            "http://custom:50052",
        ]);

        config.merge_cli_args(&args);

        assert_eq!(config.agent.node_id, Some("override-node".to_string()));
        assert_eq!(config.controller.endpoint, "http://custom:50052");
    }
}
