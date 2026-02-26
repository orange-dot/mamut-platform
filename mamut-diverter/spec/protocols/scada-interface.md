# SCADA Interface

This document defines the SCADA-facing gRPC surface for mamut-diverter.
The protocol buffers in `proto/mamut/*.proto` are authoritative.

## Service Map

| Service | Proto package | Purpose |
|---|---|---|
| `ConveyorService` | `mamut.conveyor` | Zone status, speed commands, zone event streaming |
| `DiverterService` | `mamut.diverter` | Diverter activation/deactivation, status, self-test |
| `IdentificationService` | `mamut.identification` | Query scan result, stream scan events |
| `TrackingService` | `mamut.tracking` | Query item position, stream position updates |
| `WmsService` | `mamut.wms` | Sort decision lookup based on item/barcode |
| `AlarmService` | `mamut.alarms` | Active alarms, acknowledge, alarm stream |
| `TelemetryService` | `mamut.telemetry` | Health status and metric stream |

## Interface Principles

1. gRPC APIs are transport-stable and versioned through proto evolution rules.
2. Device specifics are hidden behind MQTT contract + gateway translation.
3. Streaming RPCs are preferred for event-like data (`Stream*` methods).
4. Message IDs (`ItemId`, `ZoneId`, `DiverterId`) are immutable within a run.

## Ownership Boundary

- SCADA and dashboard integrations consume gRPC interfaces only.
- Device simulators and future MCU firmware integrate through MQTT contract.
- Gateway is responsible for MQTT<->gRPC adaptation and normalization.
