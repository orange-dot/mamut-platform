//! Prometheus metrics, tracing, and health checks.

pub mod config;
pub mod health;
pub mod metrics;

pub use config::TelemetryConfig;
pub use health::HealthRegistry;
pub use metrics::{MetricRecordError, MetricStream, MetricsRegistry};

/// Initializes structured tracing with environment override support.
pub fn init_tracing(
    default_level: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .or_else(|_| tracing_subscriber::EnvFilter::try_new(default_level))
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt().with_env_filter(env_filter).try_init()
}

/// Creates a metrics registry instance used by telemetry gRPC service.
pub fn init_metrics(channel_capacity: usize) -> MetricsRegistry {
    MetricsRegistry::new(channel_capacity)
}
