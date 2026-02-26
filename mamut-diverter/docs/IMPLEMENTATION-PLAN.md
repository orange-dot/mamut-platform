# mamut-diverter Implementation Plan

> **POC Strategy: Simulation-First**
>
> This is a hardware-free POC. Every physical device (diverter, sensor, conveyor motor, barcode scanner) is replaced by a software simulator that speaks the same MQTT protocol real firmware would. The MQTT topic/message contract is the boundary — services above it don't know or care whether they're talking to a Python simulator or real MCU firmware. When hardware arrives, we swap the simulators for real firmware. Core logic stays untouched.

---

## Architecture: Simulation-First Boundary

```
┌─────────────────────────────────────────────────┐
│                  Dashboard (React)               │
│                   gRPC-web                       │
├─────────────────────────────────────────────────┤
│           Rust Services (gRPC/tonic)             │
│  controller · gateway · wms-bridge · postal      │
├──────────────────── MQTT ────────────────────────┤  ← contract boundary
│         Device Simulators (Python)               │
│  sim-conveyor · sim-diverter · sim-scanner       │
│  sim-sensor · sim-safety                         │
└─────────────────────────────────────────────────┘
         ↕ (future: swap for real firmware)
    ┌──────────────┐
    │  C Firmware   │  ← deferred to hardware phase
    │  on real MCU  │
    └──────────────┘
```

The MQTT topic naming and protobuf message formats defined in Phase 1 are the **immutable contract**. Everything above and below that line is swappable.

---

## Phase 1: Proto Code Generation & Transport Foundation

**Goal**: Establish the generated types, MQTT message contract, and event bus that all services depend on.

### 1A — Proto code generation (medium)

**Docs**: Write `spec/protocols/scada-interface.md` with gRPC service map; update `docs/en/api/grpc.md` and `docs/sr/api/grpc.md` with service reference generated from proto comments.

**Code**:
- Create `services/crates/mamut-proto/` — new crate with `build.rs` using `tonic-build` to compile all 8 `.proto` files under `proto/mamut/`
- Add `mamut-proto` to workspace members in `services/Cargo.toml`
- Add workspace deps: `tonic-build`, `prost-build`
- Implement `tools/proto-gen.sh` to invoke `cargo build -p mamut-proto` (replaces stub)
- Re-export generated types from `mamut-proto/src/lib.rs`

**Files to create/modify**:
- `services/crates/mamut-proto/Cargo.toml` (new)
- `services/crates/mamut-proto/build.rs` (new)
- `services/crates/mamut-proto/src/lib.rs` (new)
- `services/Cargo.toml` (add member + deps)
- `tools/proto-gen.sh` (replace stub)

**Verify**: `make proto && cargo build -p mamut-proto` — generated Rust types compile.

### 1B — MQTT message contract (medium)

**Docs**: Write `spec/protocols/mqtt-contract.md` — the **single source of truth** for the MQTT topic tree and message formats that both simulators and future firmware must follow:
- Topic tree: `mamut/{site}/{device_type}/{device_id}/{msg_type}` (e.g. `mamut/sim/diverter/div-01/status`)
- Message types: status reports (device→services), commands (services→device), telemetry, alarms
- Serialization: JSON for POC (protobuf-c for future firmware)
- QoS levels, retained messages, LWT (Last Will & Testament) for device online/offline

This spec is the **immutable contract** between services and devices — simulators implement it now, real firmware implements it later.

**Files to create/modify**:
- `spec/protocols/mqtt-contract.md` (new — critical document)

### 1C — Transport / event bus crate (medium)

**Docs**: Write `spec/system/data-flow.md` — document event bus channels, MQTT topic naming, gRPC streaming patterns.

**Code** — implement `services/crates/mamut-transport/src/`:
- `lib.rs` — module re-exports
- `event_bus.rs` — in-process event bus using `tokio::sync::broadcast` for `SystemEvent`
- `config.rs` — TOML config loading using `serde` (grpc_listen, mqtt_broker addresses)

**Files to modify**:
- `services/crates/mamut-transport/Cargo.toml` (add deps: mamut-core, mamut-proto, tokio, toml, serde)
- `services/crates/mamut-transport/src/lib.rs` (replace stub)
- `services/crates/mamut-transport/src/event_bus.rs` (new)
- `services/crates/mamut-transport/src/config.rs` (new)
- `spec/system/data-flow.md` (replace stub)

**Verify**: `cargo test -p mamut-transport` — event publish/subscribe round-trips.

---

## Phase 2: Telemetry, Alarms & Specs Foundation

**Goal**: Cross-cutting concerns needed by every service, plus foundational specs.

### 2A — Telemetry crate (medium)

**Docs**: Update `config/defaults/telemetry.toml` with full schema; write `docs/en/operations/deployment.md` with observability section.

**Code** — implement `services/crates/mamut-telemetry/src/`:
- `lib.rs` — init_tracing(), init_metrics() setup functions
- `health.rs` — `HealthRegistry` struct implementing `TelemetryService::GetHealth` via tonic
- `metrics.rs` — Prometheus-compatible metric recording + `StreamMetrics` server-streaming

**Files to modify**:
- `services/crates/mamut-telemetry/Cargo.toml` (add deps: mamut-proto, tonic, tracing-subscriber, prometheus)
- `services/crates/mamut-telemetry/src/lib.rs`, `health.rs`, `metrics.rs`
- `config/defaults/telemetry.toml` (expand)
- `docs/en/operations/deployment.md` (replace stub)
- `docs/sr/operations/deployment.sr.md` (replace stub)

**Verify**: `cargo test -p mamut-telemetry` — health check returns healthy, metric streaming works.

### 2B — Alarm crate (medium)

**Docs**: Write `spec/system/safety-interlocks.md` — define alarm severities, escalation rules, ISA-18.2 lifecycle.

**Code** — implement `services/crates/mamut-alarm/src/`:
- `lib.rs` — module exports
- `alarm_manager.rs` — `AlarmManager` struct: raise/ack/clear lifecycle, in-memory store
- `service.rs` — tonic `AlarmService` implementation (GetActiveAlarms, AcknowledgeAlarm, StreamAlarms)

**Files to modify**:
- `services/crates/mamut-alarm/Cargo.toml`
- `services/crates/mamut-alarm/src/lib.rs`, `alarm_manager.rs`, `service.rs`
- `spec/system/safety-interlocks.md` (replace stub)

**Verify**: `cargo test -p mamut-alarm` — raise→ack→clear lifecycle; streaming delivers new alarms.

### 2C — Specification flesh-out (medium)

Write substantive content for foundational specs:
- `spec/system/architecture.md` — expand with layer responsibilities, deployment topology, communication matrix. **Explicitly document the simulation boundary**: MQTT is the hardware abstraction layer, simulators live below it, services above it.
- `spec/system/state-machines.md` — define zone states (Idle→Running→Fault→Stopped) and diverter states (Idle→Active→Fault→Testing) with transition tables
- `spec/conveyor/zone-management.md` — zone lifecycle, speed ramping, item handoff protocol
- `spec/conveyor/speed-control.md` — PID parameters, accel/decel profiles, units
- `spec/conveyor/belt-transport.md` — physical model, belt types, tension monitoring

**Files to modify**: 5 spec files listed above (replace stubs with content).

**Verify**: Review for internal consistency with proto definitions and existing diverter-interface.md.

---

## Phase 3: Core Service Crates

**Goal**: Implement the domain logic crates that binaries compose. All crates are hardware-agnostic — they consume events from the event bus and issue commands via gRPC/MQTT. They never talk directly to hardware.

### 3A — Conveyor crate (medium)

**Code** — implement `services/crates/mamut-conveyor/src/`:
- `lib.rs` — re-exports
- `zone.rs` — `Zone` struct with state machine (Idle/Running/Fault/Stopped), speed control
- `service.rs` — tonic `ConveyorService` (GetZoneStatus, SetZoneSpeed, StreamZoneEvents)

**Files to modify**:
- `services/crates/mamut-conveyor/Cargo.toml` (add deps: mamut-core, mamut-proto, mamut-transport, tonic, tokio)
- `services/crates/mamut-conveyor/src/lib.rs`, `zone.rs`, `service.rs`

**Verify**: `cargo test -p mamut-conveyor` — zone state transitions, speed bounds enforced, streaming events.

### 3B — Identification crate (medium)

**Docs**: Write `spec/identification/barcode-scanning.md` — barcode types, confidence thresholds, retry logic; `spec/identification/item-tracking.md` — tracking model.

**Code** — implement `services/crates/mamut-identification/src/`:
- `lib.rs` — re-exports
- `scanner.rs` — `ScannerService` managing scan results
- `tracker.rs` — `ItemTracker` mapping item_id → zone_id with timestamps
- `service.rs` — tonic `IdentificationService` + `TrackingService`

**Files to modify**:
- `services/crates/mamut-identification/Cargo.toml`
- `services/crates/mamut-identification/src/lib.rs`, `scanner.rs`, `tracker.rs`, `service.rs`
- `spec/identification/barcode-scanning.md`, `spec/identification/item-tracking.md`

**Verify**: `cargo test -p mamut-identification` — scan→track→position query works.

### 3C — WMS crate (small)

**Docs**: Write `spec/protocols/wms-interface.md` — sort decision protocol, SAP/Manhattan adapter patterns.

**Code** — implement `services/crates/mamut-wms/src/`:
- `lib.rs` — re-exports
- `sort_engine.rs` — `SortEngine` applying routing rules to return `SortDecision`
- `service.rs` — tonic `WmsService::GetSortDecision`

**Files to modify**:
- `services/crates/mamut-wms/Cargo.toml`
- `services/crates/mamut-wms/src/lib.rs`, `sort_engine.rs`, `service.rs`
- `spec/protocols/wms-interface.md`

**Verify**: `cargo test -p mamut-wms` — sort decisions resolve for known items.

### 3D — Postal crate (small)

**Docs**: Write `spec/protocols/postal-sorting-plans.md`, `spec/protocols/edi-formats.md`, `spec/protocols/customs-declarations.md`.

**Code** — implement `services/crates/mamut-postal/src/`:
- `lib.rs` — re-exports
- `sorting_plan.rs` — postal sorting plan parser (destination→direction mapping)
- `customs.rs` — customs declaration data model

**Files to modify**:
- `services/crates/mamut-postal/Cargo.toml`
- `services/crates/mamut-postal/src/lib.rs`, `sorting_plan.rs`, `customs.rs`
- 3 spec files listed above

**Verify**: `cargo test -p mamut-postal` — sorting plan parsing, customs data round-trip.

---

## Phase 4: Service Binaries

**Goal**: Wire crates into running gRPC servers. Gateway connects to MQTT broker where simulators (not hardware) publish device messages.

### 4A — mamut-gateway (large)

**Docs**: Update `config/defaults/gateway.toml` with MQTT topic config, reconnect policy, TLS settings.

**Code** — implement `services/binaries/mamut-gateway/src/`:
- `main.rs` — tokio::main, config loading, tonic server startup
- `mqtt.rs` — MQTT client (rumqttc) subscribing to device topics, publishing commands. In POC these come from Python simulators; the gateway doesn't know the difference.
- `bridge.rs` — bidirectional MQTT↔gRPC translation: device sensor/status messages → gRPC events; gRPC commands → MQTT command topics

**Dependencies to add**: `rumqttc`, `mamut-proto`, `mamut-transport`, `mamut-telemetry`, `mamut-conveyor`, `mamut-diverter`

**Files to modify**:
- `services/binaries/mamut-gateway/Cargo.toml`
- `services/binaries/mamut-gateway/src/main.rs`, `mqtt.rs`, `bridge.rs`
- `config/defaults/gateway.toml`

**Verify**: `cargo run -p mamut-gateway -- --config config/defaults/` starts and listens on :50051; health endpoint responds. With MQTT broker running, subscribes to device topics.

### 4B — mamut-controller (large)

**Docs**: Write `docs/en/architecture/overview.md`, `docs/sr/architecture/overview.sr.md` — controller orchestration flow.

**Code** — implement `services/binaries/mamut-controller/src/`:
- `main.rs` — tokio::main, tonic server with all services mounted
- `orchestrator.rs` — main control loop: receives events from event bus, coordinates diverter activation based on scan→sort decision→zone position
- `config.rs` — controller config loading

**Dependencies**: all domain crates + mamut-proto, mamut-transport, mamut-telemetry, mamut-alarm

**Files to modify**:
- `services/binaries/mamut-controller/Cargo.toml`
- `services/binaries/mamut-controller/src/main.rs`, `orchestrator.rs`, `config.rs`
- `docs/en/architecture/overview.md`, `docs/sr/architecture/overview.sr.md`

**Verify**: `cargo run -p mamut-controller` starts, mounts gRPC services on :50052, health check passes.

### 4C — mamut-wms-bridge (small)

**Code** — implement `services/binaries/mamut-wms-bridge/src/`:
- `main.rs` — tokio::main, starts WmsService gRPC server
- `client.rs` — mock WMS client returning configurable sort decisions (no external WMS in POC)

**Files to modify**:
- `services/binaries/mamut-wms-bridge/Cargo.toml`
- `services/binaries/mamut-wms-bridge/src/main.rs`, `client.rs`

### 4D — mamut-postal-bridge (small)

**Code** — implement `services/binaries/mamut-postal-bridge/src/`:
- `main.rs` — tokio::main, starts postal/customs integration service
- `edi_client.rs` — mock EDI handler returning canned responses (no real EDI in POC)

**Files to modify**:
- `services/binaries/mamut-postal-bridge/Cargo.toml`
- `services/binaries/mamut-postal-bridge/src/main.rs`, `edi_client.rs`

**Verify (4C+4D)**: Both binaries start, respond to health checks. `make test-services` passes.

---

## Phase 5: Device Simulators

**Goal**: Build Python processes that emulate every physical device over MQTT, following the contract from `spec/protocols/mqtt-contract.md`. These simulators **are** the hardware for the POC.

### 5A — Simulator framework & conveyor sim (medium)

**Docs**: Write `spec/simulation/simulator-framework.md` — document the simulator architecture: how each sim device connects to MQTT, publishes status, receives commands, and models timing/faults.

**Code**:
- `sim/mamut_sim/mqtt_client.py` (new) — shared MQTT client wrapper (paho-mqtt), connects to broker, provides publish/subscribe helpers with topic templating from the contract
- `sim/mamut_sim/base_device.py` (new) — `BaseDevice` class: MQTT connection, config loading, state machine skeleton, periodic status publishing, command handler registration, LWT setup
- `sim/mamut_sim/conveyor.py` — `SimulatedConveyor`: zone state machine (Idle→Running→Fault→Stopped), configurable belt speed, item tracking within zone, publishes zone status, responds to speed/start/stop commands
- `sim/mamut_sim/sensor.py` (new) — `SimulatedSensor`: photo-eye emulation, generates item-detected/item-cleared events on timers, configurable debounce, publishes to sensor topics

**Dependencies to add to `sim/pyproject.toml`**: `paho-mqtt`

**Files to create/modify**:
- `sim/mamut_sim/mqtt_client.py` (new)
- `sim/mamut_sim/base_device.py` (new)
- `sim/mamut_sim/conveyor.py` (replace stub)
- `sim/mamut_sim/sensor.py` (new)
- `sim/pyproject.toml` (add paho-mqtt)
- `spec/simulation/simulator-framework.md` (new)

**Verify**: `pytest sim/` — `SimulatedConveyor` publishes status to MQTT, responds to speed commands, zone transitions work.

### 5B — Diverter & scanner simulators (medium)

**Code**:
- `sim/mamut_sim/diverter.py` — `SimulatedDiverter`: models activation delay (configurable ms), direction change, fault injection (random or scripted), self-test response. Publishes diverter status, responds to activate/deactivate/self-test commands. Supports all 3 profiles (MDR, popup, shoe) via config — different timing parameters, same MQTT interface.
- `sim/mamut_sim/scanner.py` — `SimulatedScanner`: generates barcode scan events when triggered (by item-in-zone event or timer), configurable scan confidence and miss rate, publishes scan results to identification topics

**Files to modify**:
- `sim/mamut_sim/diverter.py` (replace stub)
- `sim/mamut_sim/scanner.py` (replace stub)

**Verify**: `pytest sim/` — diverter activates/deactivates with correct timing, scanner generates scan events with configurable confidence.

### 5C — Safety simulator & device launcher (medium)

**Code**:
- `sim/mamut_sim/safety.py` (new) — `SimulatedSafety`: models e-stop, interlock checking (zone must be stopped before diverter activation), watchdog heartbeat publishing. Allows injecting fault conditions for testing alarm flows.
- `sim/mamut_sim/launcher.py` (new) — reads a layout YAML, instantiates all simulated devices for that layout, connects them all to the MQTT broker, manages lifecycle (start/stop/status). CLI: `python -m mamut_sim.launcher --layout sim/layouts/test-bench.yaml --broker localhost:1883`
- `sim/mamut_sim/__main__.py` (new) — entry point for `python -m mamut_sim`

**Config**:
- `config/defaults/simulator.toml` (new) — simulator-specific config: timing multiplier (for fast-forward), fault injection probability, default device profiles
- Update layout YAMLs (`sim/layouts/test-bench.yaml`, `postal-center.yaml`, `customs.yaml`) with full device definitions: each zone lists its sensors, diverters, scanners with config

**Files to create/modify**:
- `sim/mamut_sim/safety.py` (new)
- `sim/mamut_sim/launcher.py` (new)
- `sim/mamut_sim/__main__.py` (new)
- `sim/layouts/test-bench.yaml` (expand)
- `sim/layouts/postal-center.yaml` (expand)
- `sim/layouts/customs.yaml` (expand)
- `config/defaults/simulator.toml` (new)

**Verify**: `python -m mamut_sim --layout sim/layouts/test-bench.yaml` starts all simulated devices, publishes heartbeats to MQTT. `pytest sim/` — safety interlocks prevent diverter activation in faulted zone.

### 5D — Diverter profile specs (small)

Write device specs that simulators model and future firmware will implement:
- `spec/diverter/profile-mdr.md` — MDR timing, fault codes, activation sequence
- `spec/diverter/profile-popup.md` — Pop-up wheel timing, fault codes
- `spec/diverter/profile-shoe.md` — Sliding shoe timing, fault codes
- `spec/identification/ocr-labels.md` — OCR integration protocol (simulated as scanner variant)

---

## Phase 6: System Simulation & Scenarios

**Goal**: End-to-end scenario orchestration — items flowing through the simulated plant, exercising the full stack from simulator through services to dashboard.

### 6A — Layout model & item flow engine (medium)

**Code**:
- `sim/mamut_sim/layout.py` — YAML layout loader, builds a directed graph of zones and diverters, resolves routing paths
- `sim/mamut_sim/item.py` (new) — `SimulatedItem`: represents a package with barcode, destination, current position. Moves through zones based on belt speed, triggers sensor events on zone entry/exit.
- `sim/mamut_sim/runner.py` (new) — `ScenarioRunner`: orchestrates a simulation scenario. Injects items at ingress zones, advances simulation clock, logs routing decisions. Connects to MQTT to observe events and verify outcomes.

**Files to create/modify**:
- `sim/mamut_sim/layout.py` (replace stub)
- `sim/mamut_sim/item.py` (new)
- `sim/mamut_sim/runner.py` (new)

**Verify**: `pytest sim/` — load test-bench layout, inject item, observe it traverse 2 zones with correct sensor events.

### 6B — Scenario definitions & validation (medium)

**Code**:
- `sim/scenarios/basic-sort.yaml` (new) — single item: scan → sort decision → diverter activation → item arrives at correct output
- `sim/scenarios/multi-item.yaml` (new) — multiple items with different destinations, tests concurrent routing
- `sim/scenarios/fault-recovery.yaml` (new) — inject diverter fault mid-route, verify alarm raised, item re-routed or held
- `sim/scenarios/e-stop.yaml` (new) — emergency stop during operation, all zones stop, items held in place
- `sim/mamut_sim/validator.py` (new) — `ScenarioValidator`: asserts expected outcomes — item arrived at correct destination, alarms fired/cleared, timing within bounds

**Files to create/modify**:
- `sim/scenarios/` directory (new) with 4 YAML scenario files
- `sim/mamut_sim/validator.py` (new)

**Verify**: `pytest sim/ -k scenario` — all 4 scenarios pass against running services + simulators.

### 6C — Docker simulation stack (small)

Complete Docker Compose environment that runs the full POC with zero hardware:

- `docker-compose.sim.yml` — full stack:
  - `mosquitto` — MQTT broker
  - `mamut-gateway` — connects to mosquitto
  - `mamut-controller` — orchestration
  - `mamut-wms-bridge` — mock sort decisions
  - `mamut-sim` — Python simulator launcher with test-bench layout
  - `mamut-dashboard` — web UI
- Update `docker/Dockerfile.sim` to install paho-mqtt, run launcher

**Verify**: `docker compose -f docker-compose.sim.yml up` — full system runs, items flow through simulated plant, dashboard shows live status.

---

## Phase 7: Dashboard

**Goal**: Operational monitoring dashboard with real-time data from the simulated system.

### 7A — Foundation & types (medium)

**Code**:
- `dashboard/src/types/` — TypeScript types matching proto definitions (generated or hand-written)
- `dashboard/src/api/grpcClient.ts` — gRPC-web client setup (using `@connectrpc/connect-web` or `grpc-web`)
- `dashboard/src/api/services.ts` — typed API functions for each gRPC service
- `dashboard/src/store/store.ts` — Zustand or React context store for system state
- `dashboard/src/hooks/useStreaming.ts` — hook for gRPC streaming subscriptions

**Files**: Create files in api/, types/, store/, hooks/ directories (replacing .gitkeep)

### 7B — Pages & components (large)

**Docs**: Write `docs/en/subsystems/conveyor.md`, `docs/en/subsystems/diverter.md`, `docs/sr/subsystems/conveyor.sr.md`, `docs/sr/subsystems/diverter.sr.md`.

**Code**:
- `dashboard/src/pages/OverviewPage.tsx` — system overview with zone/diverter status grid
- `dashboard/src/pages/DiverterPage.tsx` — diverter detail view with controls and self-test
- `dashboard/src/pages/AlarmsPage.tsx` — active alarms table with ack button
- `dashboard/src/pages/TelemetryPage.tsx` — health status and metrics display
- `dashboard/src/components/ZoneCard.tsx` — zone status card (state, speed, item present)
- `dashboard/src/components/DiverterCard.tsx` — diverter status card (state, direction)
- `dashboard/src/components/AlarmBanner.tsx` — critical alarm notification banner
- `dashboard/src/components/Layout.tsx` — app shell with navigation sidebar
- `dashboard/src/App.tsx` — React Router setup with page routing

**i18n**: Expand `public/locales/en.json` and `public/locales/sr.json` with all UI strings.

**Verify**: `npm run build` passes; `npm test` with component tests; `npm run dev` renders pages with live data from simulated stack.

### 7C — Dashboard Docker & proxy (small)

- Implement `docker/Dockerfile.dashboard` — nginx serving built assets with grpc-web proxy config
- Add Envoy or nginx gRPC-web proxy config to `docker-compose.yml`

**Verify**: `docker compose up dashboard` serves UI and proxies gRPC-web to controller.

---

## Phase 8: Integration, Polish & Hardware Readiness

**Goal**: End-to-end verification of the simulated stack, complete docs, and prepare the architecture for hardware swap.

### 8A — Configuration completeness (small)

- Expand all `config/defaults/*.toml` with full documented schemas
- Flesh out `config/sites/belgrade-posta/site.toml` as reference deployment (uses postal-center layout)
- Flesh out `config/sites/nis-carina/site.toml` as customs variant (uses customs layout)
- Document config layering in `docs/en/operations/deployment.md`

### 8B — Integration tests (medium)

- `services/tests/integration/` — Rust integration tests: start controller + gateway in-process with embedded MQTT broker (or test container), exercise full scan→sort→divert flow via gRPC clients against simulators
- `dashboard/e2e/` — Playwright tests against running simulated backend (basic smoke: page loads, zones displayed, item routes through)
- Expand `spec/test-vectors/` with comprehensive cross-layer vectors:
  - `conveyor-zones.json` — zone state transition vectors
  - `safety-interlocks.json` — interlock validation vectors
  - `diverter-mdr.json`, `diverter-popup.json`, `diverter-shoe.json`

### 8C — Documentation finalization (medium)

Complete all remaining doc stubs:
- `docs/en/api/grpc.md` — auto-generated service reference
- `docs/en/architecture/overview.md` — comprehensive architecture guide, emphasizing simulation-first approach
- `docs/en/operations/deployment.md` — complete deployment guide (Docker sim stack, config, site setup)
- All `docs/sr/` Serbian translations
- `spec/protocols/scada-interface.md` — SCADA integration notes

### 8D — Hardware readiness checklist (small)

Document the path from POC to hardware:
- `docs/en/operations/hardware-migration.md` (new) — step-by-step guide:
  1. Which Python simulator maps to which firmware module
  2. MQTT topic contract compliance checklist
  3. How to run mixed mode (some real devices, some simulated) for incremental migration
  4. HAL implementation guide for new board targets (reference existing `hal_posix.c`)
  5. Firmware test strategy: replay same test vectors and scenarios against real hardware

### 8E — CI/CD hardening (small)

- Verify `.github/workflows/ci.yml` runs all tests end-to-end (including sim scenarios)
- Add proto generation step to CI
- Add Docker build validation step
- Add `make sim-test` target that spins up Docker sim stack, runs scenarios, tears down

**Final Verification**: `make all && make test` — Rust services, Python sim, and dashboard all build and pass tests. `docker compose -f docker-compose.sim.yml up` — full simulated plant runs. Scenarios execute end-to-end. Dashboard shows live item routing.

---

## Phase Dependency Graph

```
Phase 1 (Proto + MQTT Contract + Transport)
    ├── Phase 2 (Telemetry + Alarms + Specs)
    │       └── Phase 3 (Domain Crates)
    │               └── Phase 4 (Service Binaries)
    │                       ├── Phase 7 (Dashboard)
    │                       └── Phase 8 (Integration)
    └── Phase 5 (Device Simulators) — parallel with Phases 2-4
            └── Phase 6 (System Simulation) — needs Phase 4 + Phase 5
```

**Key difference from hardware-first approach**: Phase 5 (simulators) can start as soon as the MQTT contract is defined (Phase 1B). Simulator development runs in parallel with service development. By Phase 6 the full stack is testable without any hardware.

## Verification Strategy

Each phase has local verification (unit tests, compile checks). Cross-phase verification points:

| Checkpoint | Test |
|------------|------|
| After Phase 1 | `cargo build` — all crates compile with generated proto types |
| After Phase 4 | `cargo test` — all service tests pass; binaries start |
| After Phase 5 | `pytest sim/` — all device simulators publish correct MQTT messages |
| After Phase 6 | `pytest sim/ -k scenario` — full item-routing scenarios pass end-to-end |
| After Phase 7 | `npm run build && npm test` — dashboard builds and component tests pass |
| After Phase 8 | `docker compose -f docker-compose.sim.yml up && make sim-test` — full simulated plant runs, all scenarios green |

## Future: Hardware Integration (not in POC scope)

When hardware arrives, the migration path is:
1. Flash C firmware implementing the same MQTT contract (`spec/protocols/mqtt-contract.md`)
2. Run in mixed mode: real devices + simulators on the same MQTT broker
3. Replace simulators one device at a time, verifying each with the same test scenarios
4. Core Rust services and dashboard require **zero changes** — they only see MQTT messages
