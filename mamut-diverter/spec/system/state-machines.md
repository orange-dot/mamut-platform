# State Machines

This document defines canonical runtime state machines for conveyor zones and
diverters. Names are aligned with proto enums and firmware concepts.

## Conveyor Zone State Machine

Proto enum mapping: `mamut.conveyor.ZoneState`
- `ZONE_STATE_IDLE`
- `ZONE_STATE_RUNNING`
- `ZONE_STATE_FAULT`
- `ZONE_STATE_STOPPED`

### Transition Table

| From | Event | Guard | To | Action |
|---|---|---|---|---|
| `IDLE` | `SetZoneSpeed(0)` | none | `IDLE` | no-op acknowledgement, keep speed target at zero |
| `IDLE` | `SetZoneSpeed(speed > 0)` | zone healthy | `RUNNING` | apply ramp-up profile, publish status |
| `RUNNING` | `SetZoneSpeed(0)` | command accepted; deceleration profile active | `STOPPED` | commit stop transition, speed converges to zero via ramp, publish status |
| `STOPPED` | `SetZoneSpeed(speed > 0)` | interlocks clear | `RUNNING` | ramp-up, publish status |
| `RUNNING` | fault detected (jam, drive fault, timeout) | none | `FAULT` | emergency/controlled stop, raise alarm |
| `IDLE` | fault detected | none | `FAULT` | raise alarm |
| `STOPPED` | fault detected | none | `FAULT` | raise alarm |
| `FAULT` | reset + self-check passed | operator/system reset allowed | `IDLE` | clear fault alarm, publish status |

### Invalid Transitions

- `FAULT` -> `RUNNING` without reset is invalid.
- `RUNNING` -> `IDLE` direct transition is invalid; must pass through `STOPPED`.
- Commands violating guard rules must return command failure (`command_result`) and keep state unchanged.

## Diverter State Machine

Interface alignment: `spec/diverter/diverter-interface.md`  
Proto enum mapping: `mamut.diverter.DiverterState`
- `DIVERTER_STATE_IDLE`
- `DIVERTER_STATE_ACTIVE`
- `DIVERTER_STATE_FAULT`
- `DIVERTER_STATE_TESTING`

### Transition Table

| From | Event | Guard | To | Action |
|---|---|---|---|---|
| `IDLE` | `Activate(target)` | target valid, interlocks clear | `ACTIVE` | actuate diverter, publish state |
| `ACTIVE` | `Deactivate()` | none | `IDLE` | return pass-through position |
| `IDLE` | `SelfTest()` | maintenance window or safe condition | `TESTING` | run diagnostics |
| `TESTING` | self-test passed | none | `IDLE` | publish test result |
| `TESTING` | self-test failed | none | `FAULT` | raise alarm and lock command path |
| `IDLE`/`ACTIVE` | actuator/sensor fault | none | `FAULT` | raise alarm |
| `FAULT` | reset + diagnostics passed | authorized reset | `IDLE` | clear alarm and resume |

### Invalid Transitions

- `FAULT` -> `ACTIVE` direct activation is invalid.
- `TESTING` rejects `Activate`/`Deactivate` commands until test completes.

## Command and Status Semantics

- Every successful transition publishes updated `status`.
- Every failed transition publishes `command_result` with failure reason
  (MQTT `command_result` at device layer; gRPC `Status` at service layer).
- State transitions that imply safety impact also raise an alarm event.

## Timing Constraints (POC Targets)

- Command-to-state update visibility: <= 250 ms.
- Fault-to-alarm publication: <= 500 ms.
- Self-test timeout: configurable, default 5 s for POC.
