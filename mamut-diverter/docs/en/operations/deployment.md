# Deployment Guide

This guide covers Phase 2A deployment for telemetry, health, and metrics.

## Prerequisites

- Rust toolchain installed (`cargo`, `rustc`)
- MQTT broker reachable by services (for full stack runs)
- Ports available:
  - gRPC services: `50051`, `50052`
  - Prometheus scrape endpoint: `9090`

## Telemetry Configuration

Default config: `config/defaults/telemetry.toml`

Key fields:
- `log_level`: tracing level (`info`, `debug`, `warn`, ...)
- `prometheus_listen`: endpoint for metrics exporter
- `service_name`: default service identity for health responses
- `health_default`: initial health state
- `metrics_channel_capacity`: in-process stream buffer size
- `metrics_flush_interval_ms`: metrics publication cadence target

## Build and Test

Run telemetry tests:

```bash
cd services
cargo test -p mamut-telemetry
```

Run telemetry-focused lint:

```bash
cd services
cargo clippy -p mamut-telemetry --all-targets -- -D warnings
```

## Runtime Notes

- `GetHealth` returns service health with timestamp from server time.
- `StreamMetrics` is server-streaming and supports metric-name filters.
- Metrics are Prometheus-compatible and can be scraped from the configured endpoint.
- Slow stream consumers may miss old samples when channel buffers overflow; monitor capacity.

## Observability Checklist

1. Confirm health endpoint returns `healthy=true` for expected service names.
2. Confirm metric stream emits samples after recording events.
3. Confirm Prometheus can scrape telemetry endpoint.
4. Alert on missing telemetry updates or sustained stream lag.
