//! Transport primitives for mamut services.

pub mod config;
pub mod event_bus;

pub use config::TransportConfig;
pub use event_bus::EventBus;
