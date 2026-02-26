//! Server-side gRPC implementations.
//!
//! This module provides the server implementations for agents to receive
//! commands from the controller.

mod grpc;

pub use grpc::{AgentGrpcServer, AgentServiceHandler};
