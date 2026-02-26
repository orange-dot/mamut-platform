//! Alarm lifecycle management (ISA-18.2).

pub mod alarm_manager;
pub mod service;

pub use alarm_manager::{AlarmError, AlarmManager};
pub use service::{AlarmGrpcService, AlarmStream};
