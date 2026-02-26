# Data Flow

This document defines runtime data flow between firmware/simulators, gateway,
domain services, and dashboard/SCADA clients.

## End-to-End Flow

1. Device simulator (or future MCU firmware) publishes MQTT `status/telemetry/alarm`.
2. Gateway subscribes to MQTT device topics and normalizes payloads.
3. Gateway converts device payloads into internal domain events.
4. Domain events are published to in-process event bus (`mamut-transport`).
5. Service crates consume events and update state/decisions.
6. gRPC APIs expose latest state and stream incremental updates.
7. Dashboard/SCADA consumes gRPC/gRPC-web endpoints.

## Channel Types

| Channel | Transport | Direction | Producer | Consumer |
|---|---|---|---|---|
| Device status | MQTT | device -> gateway | simulator/firmware | gateway |
| Device commands | MQTT | gateway -> device | gateway/services | simulator/firmware |
| Internal events | Tokio broadcast | in-process | gateway/services | services |
| Service APIs | gRPC | bidirectional request/stream | services | dashboard/SCADA/integrations |

## MQTT to gRPC/Event Mapping

| MQTT msg_type | Internal event class | Typical gRPC stream |
|---|---|---|
| `status` | state transition/event | `StreamZoneEvents`, `GetStatus` polling |
| `command_result` | command acknowledgment/outcome | service-specific status/ack streams |
| `telemetry` | metrics sample | `StreamMetrics` |
| `alarm` | alarm raised/updated | `StreamAlarms` |

## Topic Naming Reference

Topic root contract is defined in `spec/protocols/mqtt-contract.md`:

`mamut/{site}/{device_type}/{device_id}/{msg_type}`

## Timing and Delivery Expectations (POC)

- Command path latency target (gateway -> device ack): <= 250 ms
- Status propagation (device -> gRPC stream): <= 500 ms
- Telemetry stream interval: configurable, default 1s
- Alarm delivery: at-least-once semantics via MQTT QoS 1

These are target SLOs for implementation planning and are not yet enforced by
runtime checks in Phase 1.
