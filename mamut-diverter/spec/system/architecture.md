# System Architecture

This document is the authoritative architecture reference for the
`mamut-diverter` platform.

## Design Principles

1. Keep device and service layers decoupled by protocol contracts.
2. Preserve one immutable boundary: MQTT topic/payload contract.
3. Keep domain crates hardware-agnostic.
4. Prefer stream-based APIs for state changes and telemetry.

## Layer Responsibilities

| Layer | Runtime components | Responsibilities |
|---|---|---|
| Device layer | Python simulators now, MCU firmware later | Device control loops, sensor reads, actuator commands, local fail-safe behavior |
| Edge layer | `mamut-gateway` | MQTT subscribe/publish, protocol normalization, gRPC bridge |
| Service layer | Rust crates + binaries (`controller`, `wms`, `postal`, `alarm`, `telemetry`) | Domain state, orchestration, decisions, events, APIs |
| UI/Integration layer | Dashboard, SCADA, external systems | Monitoring, control actions, external business integration |

## Simulation Boundary (Contract Line)

The MQTT contract (`spec/protocols/mqtt-contract.md`) is the hardware
abstraction boundary:

- Below boundary: simulators or real MCU firmware.
- Above boundary: gateway, services, dashboard, SCADA.
- Swapping simulator with firmware must not change service-layer logic.

## Deployment Topology (POC)

- Single site namespace (for example `sim`).
- One MQTT broker (single node in POC).
- One gateway process.
- One or more service processes.
- Dashboard connected via gRPC-web.

## Communication Matrix

| Source | Destination | Protocol | Contract |
|---|---|---|---|
| Device layer | Gateway | MQTT | `spec/protocols/mqtt-contract.md` |
| Gateway | Services | gRPC / internal event bus | Proto schemas + crate event model |
| Services | Gateway | gRPC (commands) -> MQTT | Proto command RPCs + MQTT `command` topics |
| Dashboard/SCADA | Services | gRPC-web / gRPC | `proto/mamut/*.proto` |

## Reliability and Failure Domains

- Device comms loss must fail safe at device layer.
- Gateway failure isolates device->service bridge but does not redefine device safety logic.
- Service failure must not disable hard safety interlocks.
- Alarm and telemetry streams are eventually consistent and may drop stale samples for lagged consumers.

## Evolution Rules

1. Proto fields evolve backward-compatibly.
2. MQTT topics and semantic meaning remain stable per site deployment.
3. New diverter or conveyor implementations plug in through existing contracts:
   firmware `mamut_*` interfaces and service traits.
