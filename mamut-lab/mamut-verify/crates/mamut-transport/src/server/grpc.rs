//! gRPC server implementation for the Agent service.
//!
//! This module implements the `AgentService` defined in the protocol buffers,
//! allowing the controller to communicate with agents.

use crate::error::TransportError;
use crate::proto::mamut::{
    agent_service_server::{AgentService, AgentServiceServer},
    ApplyFaultRequest, ApplyFaultResponse, ExecuteRequest, ExecuteResponse, HealthRequest,
    HealthResponse, RecoverFaultRequest, RecoverFaultResponse, StreamTimingRequest,
    SyncClockRequest, SyncClockResponse, TimingEvent,
};
use async_trait::async_trait;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{broadcast, oneshot, RwLock};
use tokio_stream::Stream;
use tonic::{Request, Response, Status};
use tracing::{debug, error, info, instrument, warn};

/// Handler trait for agent service operations.
///
/// Implement this trait to provide the actual logic for handling
/// agent service requests. The gRPC server delegates to this handler.
#[async_trait]
pub trait AgentServiceHandler: Send + Sync + 'static {
    /// Execute an operation against the system under test.
    async fn execute(&self, request: ExecuteRequest) -> Result<ExecuteResponse, TransportError>;

    /// Apply a fault injection to the local node.
    async fn apply_fault(
        &self,
        request: ApplyFaultRequest,
    ) -> Result<ApplyFaultResponse, TransportError>;

    /// Recover from a previously applied fault.
    async fn recover_fault(
        &self,
        request: RecoverFaultRequest,
    ) -> Result<RecoverFaultResponse, TransportError>;

    /// Perform a health check.
    async fn health(&self, request: HealthRequest) -> Result<HealthResponse, TransportError>;

    /// Synchronize the OVC clock with the controller.
    async fn sync_clock(
        &self,
        request: SyncClockRequest,
    ) -> Result<SyncClockResponse, TransportError>;

    /// Subscribe to timing events.
    ///
    /// Returns a receiver that will emit timing events as they occur.
    async fn subscribe_timing_events(
        &self,
        request: StreamTimingRequest,
    ) -> Result<broadcast::Receiver<TimingEvent>, TransportError>;
}

/// The gRPC server for the Agent service.
///
/// This server wraps an `AgentServiceHandler` implementation and exposes
/// it as a gRPC service that the controller can call.
pub struct AgentGrpcServer<H: AgentServiceHandler> {
    handler: Arc<H>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    state: Arc<RwLock<ServerState>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ServerState {
    Created,
    Running,
    ShuttingDown,
    Stopped,
}

impl<H: AgentServiceHandler> AgentGrpcServer<H> {
    /// Creates a new agent gRPC server with the given handler.
    pub fn new(handler: H) -> Self {
        Self {
            handler: Arc::new(handler),
            shutdown_tx: None,
            state: Arc::new(RwLock::new(ServerState::Created)),
        }
    }

    /// Starts the gRPC server on the specified address.
    ///
    /// This method will block until the server is shut down via the
    /// `shutdown()` method or receives a shutdown signal.
    #[instrument(skip(self), fields(addr = %addr))]
    pub async fn serve(mut self, addr: SocketAddr) -> Result<(), TransportError> {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        // Update state to running
        {
            let mut state = self.state.write().await;
            *state = ServerState::Running;
        }

        let service = AgentServiceImpl {
            handler: Arc::clone(&self.handler),
            state: Arc::clone(&self.state),
        };

        info!("Starting agent gRPC server on {}", addr);

        let server = tonic::transport::Server::builder()
            .trace_fn(|_| tracing::info_span!("agent_grpc"))
            .add_service(AgentServiceServer::new(service))
            .serve_with_shutdown(addr, async {
                let _ = shutdown_rx.await;
                info!("Received shutdown signal");
            });

        match server.await {
            Ok(()) => {
                info!("Agent gRPC server stopped gracefully");
                let mut state = self.state.write().await;
                *state = ServerState::Stopped;
                Ok(())
            }
            Err(e) => {
                error!("Agent gRPC server error: {}", e);
                Err(TransportError::Grpc(e))
            }
        }
    }

    /// Initiates a graceful shutdown of the server.
    ///
    /// Returns `true` if the shutdown signal was sent successfully.
    pub async fn shutdown(&mut self) -> bool {
        let mut state = self.state.write().await;
        if *state != ServerState::Running {
            warn!("Cannot shutdown server in state {:?}", *state);
            return false;
        }

        *state = ServerState::ShuttingDown;
        drop(state);

        if let Some(tx) = self.shutdown_tx.take() {
            info!("Sending shutdown signal to agent gRPC server");
            tx.send(()).is_ok()
        } else {
            false
        }
    }

    /// Returns a reference to the handler.
    pub fn handler(&self) -> &H {
        &self.handler
    }

    /// Returns the current server state.
    pub async fn state(&self) -> ServerState {
        *self.state.read().await
    }
}

/// Internal gRPC service implementation.
struct AgentServiceImpl<H: AgentServiceHandler> {
    handler: Arc<H>,
    state: Arc<RwLock<ServerState>>,
}

impl<H: AgentServiceHandler> AgentServiceImpl<H> {
    async fn check_running(&self) -> Result<(), Status> {
        let state = self.state.read().await;
        match *state {
            ServerState::Running => Ok(()),
            ServerState::ShuttingDown => Err(Status::unavailable("server is shutting down")),
            _ => Err(Status::internal("server is not running")),
        }
    }
}

#[async_trait]
impl<H: AgentServiceHandler> AgentService for AgentServiceImpl<H> {
    #[instrument(skip(self, request), fields(request_id = %request.get_ref().request_id))]
    async fn execute(
        &self,
        request: Request<ExecuteRequest>,
    ) -> Result<Response<ExecuteResponse>, Status> {
        self.check_running().await?;

        let req = request.into_inner();
        debug!(
            request_id = %req.request_id,
            op_type = req.operation.as_ref().map(|o| o.op_type.as_str()).unwrap_or("unknown"),
            "Executing operation"
        );

        match self.handler.execute(req).await {
            Ok(response) => {
                debug!(request_id = %response.request_id, "Operation completed");
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("Execute failed: {}", e);
                Err(e.into())
            }
        }
    }

    #[instrument(skip(self, request), fields(fault_id = %request.get_ref().fault_id))]
    async fn apply_fault(
        &self,
        request: Request<ApplyFaultRequest>,
    ) -> Result<Response<ApplyFaultResponse>, Status> {
        self.check_running().await?;

        let req = request.into_inner();
        debug!(
            fault_id = %req.fault_id,
            fault_type = ?req.fault_type,
            "Applying fault"
        );

        match self.handler.apply_fault(req).await {
            Ok(response) => {
                if response.applied {
                    info!(fault_id = %response.fault_id, "Fault applied successfully");
                } else {
                    warn!(
                        fault_id = %response.fault_id,
                        error = %response.error,
                        "Failed to apply fault"
                    );
                }
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("ApplyFault failed: {}", e);
                Err(e.into())
            }
        }
    }

    #[instrument(skip(self, request), fields(fault_id = %request.get_ref().fault_id))]
    async fn recover_fault(
        &self,
        request: Request<RecoverFaultRequest>,
    ) -> Result<Response<RecoverFaultResponse>, Status> {
        self.check_running().await?;

        let req = request.into_inner();
        debug!(fault_id = %req.fault_id, force = req.force, "Recovering from fault");

        match self.handler.recover_fault(req).await {
            Ok(response) => {
                if response.recovered {
                    info!(fault_id = %response.fault_id, "Fault recovered successfully");
                } else {
                    warn!(
                        fault_id = %response.fault_id,
                        error = %response.error,
                        "Failed to recover fault"
                    );
                }
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("RecoverFault failed: {}", e);
                Err(e.into())
            }
        }
    }

    #[instrument(skip(self, request), fields(deep_check = request.get_ref().deep_check))]
    async fn health(
        &self,
        request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        // Health check should work even during shutdown
        let req = request.into_inner();
        debug!(deep_check = req.deep_check, "Health check requested");

        match self.handler.health(req).await {
            Ok(response) => {
                debug!(status = ?response.status, "Health check completed");
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("Health check failed: {}", e);
                Err(e.into())
            }
        }
    }

    type StreamTimingEventsStream =
        Pin<Box<dyn Stream<Item = Result<TimingEvent, Status>> + Send + 'static>>;

    #[instrument(skip(self, request))]
    async fn stream_timing_events(
        &self,
        request: Request<StreamTimingRequest>,
    ) -> Result<Response<Self::StreamTimingEventsStream>, Status> {
        self.check_running().await?;

        let req = request.into_inner();
        debug!(
            event_types = ?req.event_types,
            include_historical = req.include_historical,
            "Starting timing event stream"
        );

        let receiver = self
            .handler
            .subscribe_timing_events(req)
            .await
            .map_err(|e| {
                error!("Failed to subscribe to timing events: {}", e);
                Status::from(e)
            })?;

        let stream = tokio_stream::wrappers::BroadcastStream::new(receiver).map(|result| {
            result.map_err(|e| {
                warn!("Timing event stream error: {}", e);
                Status::internal(format!("stream error: {}", e))
            })
        });

        Ok(Response::new(Box::pin(stream)))
    }

    #[instrument(skip(self, request), fields(sequence = request.get_ref().sequence))]
    async fn sync_clock(
        &self,
        request: Request<SyncClockRequest>,
    ) -> Result<Response<SyncClockResponse>, Status> {
        self.check_running().await?;

        let req = request.into_inner();
        debug!(sequence = req.sequence, "Clock sync requested");

        match self.handler.sync_clock(req).await {
            Ok(response) => {
                debug!(
                    sequence = response.sequence,
                    offset_estimate_ns = response.offset_estimate_ns,
                    "Clock sync completed"
                );
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("Clock sync failed: {}", e);
                Err(e.into())
            }
        }
    }
}

// We need to use tokio_stream for the BroadcastStream
use tokio_stream::StreamExt;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::mamut::{
        health_response::Status as HealthStatus, ClockState, Ovc, OperationOutcome, TimingBreakdown,
    };
    use std::collections::HashMap;
    use tokio::sync::broadcast;

    /// A mock handler for testing.
    struct MockHandler {
        timing_tx: broadcast::Sender<TimingEvent>,
    }

    impl MockHandler {
        fn new() -> Self {
            let (timing_tx, _) = broadcast::channel(100);
            Self { timing_tx }
        }
    }

    #[async_trait]
    impl AgentServiceHandler for MockHandler {
        async fn execute(
            &self,
            request: ExecuteRequest,
        ) -> Result<ExecuteResponse, TransportError> {
            Ok(ExecuteResponse {
                request_id: request.request_id,
                received_at: Some(Ovc::default()),
                completed_at: Some(Ovc::default()),
                outcome: Some(OperationOutcome { result: None }),
                timing: Some(TimingBreakdown::default()),
            })
        }

        async fn apply_fault(
            &self,
            request: ApplyFaultRequest,
        ) -> Result<ApplyFaultResponse, TransportError> {
            Ok(ApplyFaultResponse {
                fault_id: request.fault_id,
                applied: true,
                active_at: Some(Ovc::default()),
                error: String::new(),
            })
        }

        async fn recover_fault(
            &self,
            request: RecoverFaultRequest,
        ) -> Result<RecoverFaultResponse, TransportError> {
            Ok(RecoverFaultResponse {
                fault_id: request.fault_id,
                recovered: true,
                recovered_at: Some(Ovc::default()),
                error: String::new(),
            })
        }

        async fn health(&self, _request: HealthRequest) -> Result<HealthResponse, TransportError> {
            Ok(HealthResponse {
                status: HealthStatus::Healthy.into(),
                clock_state: Some(ClockState::default()),
                active_fault_ids: vec![],
                resources: None,
                details: HashMap::new(),
            })
        }

        async fn sync_clock(
            &self,
            request: SyncClockRequest,
        ) -> Result<SyncClockResponse, TransportError> {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as i64;

            Ok(SyncClockResponse {
                agent_clock_before: Some(Ovc::default()),
                agent_clock_after: Some(Ovc::default()),
                sequence: request.sequence,
                receive_time_ns: now,
                send_time_ns: now,
                offset_estimate_ns: 0,
            })
        }

        async fn subscribe_timing_events(
            &self,
            _request: StreamTimingRequest,
        ) -> Result<broadcast::Receiver<TimingEvent>, TransportError> {
            Ok(self.timing_tx.subscribe())
        }
    }

    #[tokio::test]
    async fn test_server_creation() {
        let handler = MockHandler::new();
        let server = AgentGrpcServer::new(handler);
        assert_eq!(server.state().await, ServerState::Created);
    }

    #[tokio::test]
    async fn test_health_check() {
        let handler = MockHandler::new();
        let service = AgentServiceImpl {
            handler: Arc::new(handler),
            state: Arc::new(RwLock::new(ServerState::Running)),
        };

        let request = Request::new(HealthRequest { deep_check: false });
        let response = service.health(request).await.unwrap();
        assert_eq!(response.get_ref().status, HealthStatus::Healthy as i32);
    }

    #[tokio::test]
    async fn test_execute() {
        let handler = MockHandler::new();
        let service = AgentServiceImpl {
            handler: Arc::new(handler),
            state: Arc::new(RwLock::new(ServerState::Running)),
        };

        let request = Request::new(ExecuteRequest {
            request_id: "test-123".to_string(),
            operation: None,
            issued_at: None,
            timeout_ns: 5000000000,
            record_timing: true,
            context: HashMap::new(),
        });

        let response = service.execute(request).await.unwrap();
        assert_eq!(response.get_ref().request_id, "test-123");
    }

    #[tokio::test]
    async fn test_shutting_down_rejects_requests() {
        let handler = MockHandler::new();
        let service = AgentServiceImpl {
            handler: Arc::new(handler),
            state: Arc::new(RwLock::new(ServerState::ShuttingDown)),
        };

        let request = Request::new(ExecuteRequest {
            request_id: "test-123".to_string(),
            operation: None,
            issued_at: None,
            timeout_ns: 5000000000,
            record_timing: true,
            context: HashMap::new(),
        });

        let result = service.execute(request).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::Unavailable);
    }
}
