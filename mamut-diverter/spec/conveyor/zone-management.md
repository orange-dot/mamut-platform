# Zone Management

This document defines zone-level control behavior for conveyor segments.

## Zone Model

Each zone is an independently controllable segment identified by `ZoneId`.
Minimum zone status fields:
- current state
- target/current speed
- item presence
- last transition timestamp (target field; not present in current `ZoneStatus`
  proto and tracked via event/status metadata until schema evolves)

Proto alignment: `mamut.conveyor.ZoneStatus`.

## Lifecycle

1. **Initialize**: zone boots in `IDLE`, speed target `0`.
2. **Start**: command sets non-zero speed, transition to `RUNNING`.
3. **Operate**: continuous sensor monitoring and handoff coordination.
4. **Stop**: command or planned stop transitions to `STOPPED`.
5. **Fault**: any critical condition transitions to `FAULT`.
6. **Recover**: reset + validation returns zone to `IDLE`.

## Item Handoff Protocol (Upstream -> Downstream)

1. Upstream zone detects item near transfer boundary.
2. Upstream checks downstream readiness:
   - downstream not in `FAULT`
   - downstream has transfer capacity
   - interlocks allow transfer
3. Upstream commits transfer and emits handoff event.
4. Downstream confirms item entry (sensor or timeout-based fallback).
5. If confirmation fails within timeout, raise transfer alarm and move to configured fallback.

## Timeouts and Fault Handling

- `zone_timeout_ms` from config defines maximum wait for expected item progress.
- Timeout in `RUNNING` triggers fault or degraded mode depending on policy.
- Persistent no-confirmation handoffs must raise warning/critical alarms per safety policy.

## Scheduling and Fairness

- Adjacent zones should avoid starvation by bounded wait policies.
- Backpressure propagates upstream through readiness checks.
- Command processing priority:
1. Safety/fault commands
2. Stop commands
3. Speed/optimization commands

## Invariants

1. A zone in `FAULT` cannot accept movement commands.
2. Item cannot be logically present in two zones at the same timestamp.
3. Handoff must be idempotent at event-processing level.
4. Recovery requires explicit reset; implicit auto-restart is disallowed for critical faults.
