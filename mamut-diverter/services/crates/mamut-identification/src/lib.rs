//! Barcode scanning, OCR, and label identification.

pub mod scanner;
pub mod service;
pub mod tracker;

pub use scanner::{ScanError, ScannerService};
pub use service::{IdentificationGrpcService, ItemPositionStream, ScanResultStream};
pub use tracker::{ItemTracker, TrackingError};
