//! Mamut Transport - gRPC transport layer for distributed test coordination.
//!
//! This crate provides the communication infrastructure for the Mamut verification
//! harness, enabling the controller to coordinate with agents deployed across the
//! distributed system under test.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────┐         ┌─────────────────────┐
//! │     Controller      │         │       Agent         │
//! │                     │  gRPC   │                     │
//! │  ┌───────────────┐  │◄───────►│  ┌───────────────┐  │
//! │  │ AgentClient   │  │         │  │ AgentServer   │  │
//! │  └───────────────┘  │         │  └───────────────┘  │
//! │                     │         │                     │
//! │  ┌───────────────┐  │         │  ┌───────────────┐  │
//! │  │ControllerSvc  │◄─┼─────────┼──│ControllerClnt │  │
//! │  └───────────────┘  │         │  └───────────────┘  │
//! └─────────────────────┘         └─────────────────────┘
//! ```
//!
//! # Modules
//!
//! - [`server`]: Server-side gRPC implementations for agents
//! - [`client`]: Client implementations for calling the controller
//! - [`proto`]: Generated Protocol Buffer types and service definitions
//! - [`error`]: Transport-specific error types
//!
//! # Example
//!
//! ```ignore
//! use mamut_transport::server::AgentGrpcServer;
//! use mamut_transport::client::ControllerClient;
//!
//! // Create and start an agent server
//! let server = AgentGrpcServer::new(agent_impl);
//! server.serve("[::1]:50051").await?;
//!
//! // Connect to the controller
//! let client = ControllerClient::connect("http://controller:50052").await?;
//! client.register_agent(registration).await?;
//! ```

pub mod client;
pub mod error;
pub mod server;

/// Generated Protocol Buffer types and gRPC service definitions.
pub mod proto {
    /// The main Mamut protocol definitions.
    pub mod mamut {
        tonic::include_proto!("mamut.v1");
    }
}

// Re-export commonly used types
pub use error::TransportError;
pub use proto::mamut::*;

// Re-export server and client types at the crate root
pub use client::ControllerClient;
pub use server::AgentGrpcServer;
