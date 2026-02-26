//! Client-side gRPC implementations.
//!
//! This module provides client implementations for agents to communicate
//! with the controller.

mod controller;

pub use controller::ControllerClient;
