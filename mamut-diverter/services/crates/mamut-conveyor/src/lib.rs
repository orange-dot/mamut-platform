//! Conveyor zone management, speed control, and item tracking.

pub mod service;
pub mod zone;

pub use service::{ConveyorGrpcService, ZoneEventStream};
pub use zone::{Zone, ZoneError};
