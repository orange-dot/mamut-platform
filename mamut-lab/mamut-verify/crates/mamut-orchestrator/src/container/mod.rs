//! Container management types and lifecycle.
//!
//! This module provides types for container specifications, resource limits,
//! and container lifecycle management.

mod lifecycle;
mod spec;

pub use lifecycle::{
    ContainerEvent, ContainerHandle, ContainerState, ContainerStats, HealthStatus, LifecycleManager,
};
pub use spec::{ContainerImage, ImagePullPolicy, ResourceLimits};
