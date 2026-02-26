# Safety Interlocks

This document defines alarm severity, escalation, and interlock behavior for
mamut-diverter. It follows ISA-18.2 lifecycle principles for alarm management.

## Scope

- Conveyor zones
- Diverter actuators
- Scanner and sensor stations
- E-stop and protective devices (light curtains, guard switches)

## Alarm Severity Model

| Severity | Proto enum | Operational meaning | Typical response time |
|---|---|---|---|
| Info | `SEVERITY_INFO` | Informational event, no safety impact | Best-effort |
| Warning | `SEVERITY_WARNING` | Degraded operation, possible near-term fault | <= 5 min |
| Critical | `SEVERITY_CRITICAL` | Immediate risk or unsafe state | Immediate (seconds) |

## Alarm Lifecycle (ISA-18.2 aligned)

States are represented by `AlarmState` in `proto/mamut/alarms.proto`:
- `ALARM_STATE_ACTIVE`
- `ALARM_STATE_ACKNOWLEDGED`
- `ALARM_STATE_CLEARED`

Lifecycle:
1. **Raise**: condition detected by simulator/firmware or service logic.
2. **Acknowledge**: operator/system confirms awareness.
3. **Clear**: root condition removed and system returns to safe envelope.

Rules:
- Acknowledgement does not imply clear.
- Cleared alarms are excluded from active alarm lists.
- Every state transition must emit an alarm stream event.

## Escalation Rules

| Trigger | Escalation |
|---|---|
| Critical alarm active > 30s without ack | Notify supervisor and force zone stop |
| 3+ warnings from same source in 10 min | Escalate next warning to critical |
| Repeated clear->active oscillation (>= 3 cycles/5 min) | Mark source as unstable and isolate zone |

## Safety Interlock Classes

| Interlock class | Examples | Required behavior |
|---|---|---|
| Hard safety interlock | E-stop chain, safety relay, light curtain | Immediate motion inhibit; recovery requires manual reset |
| Process safety interlock | Overcurrent, jam detection, diverter stall | Controlled stop and alarm raise; restart after clear policy |
| Operational interlock | Missing scan, communication timeout | Soft stop or reroute depending on configured policy |

## Fail-Safe and Recovery

- Loss of comms to safety device defaults to fail-safe stop.
- Gateway/service availability must not bypass hard safety chains.
- Restart sequence after critical clear:
1. Validate interlock source healthy.
2. Confirm operator acknowledgement where required.
3. Resume zone by configured ramp profile.

## Telemetry and Audit

- Alarm events are streamed through `AlarmService::StreamAlarms`.
- Active alarms are queried by `AlarmService::GetActiveAlarms`.
- Audit trail must preserve alarm ID, source, severity, timestamps, and
  transition sequence (raise -> acknowledge -> clear).
