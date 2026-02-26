# MQTT Contract

This is the single source of truth for MQTT topic layout and payload format
between device layer (simulators now, MCU firmware later) and gateway/services.

## Topic Namespace

Topic format:

`mamut/{site}/{device_type}/{device_id}/{msg_type}`

Example:

`mamut/sim/diverter/div-01/status`

## Allowed Segments

- `site`: logical site (`sim`, `nis-carina`, `belgrade-posta`, ...)
- `device_type`: `conveyor`, `diverter`, `scanner`, `sensor`, `safety`
- `device_id`: stable identifier (`zone-01`, `div-01`, `scan-03`)
- `msg_type`: one of `status`, `command`, `command_result`, `telemetry`, `alarm`

## Message Direction

- Device -> services (through gateway): `status`, `command_result`, `telemetry`, `alarm`
- Services -> device (through gateway): `command`

## Serialization

- POC: JSON payloads (UTF-8)
- Hardware phase target: protobuf-c payloads (same semantic fields)

## QoS, Retained, LWT

| Message type | QoS | Retained | Notes |
|---|---|---|---|
| `status` | 1 | yes | Last known state on reconnect |
| `command` | 1 | no | Must not be replayed implicitly |
| `command_result` | 1 | no | Command acknowledgement and outcome |
| `telemetry` | 0 | no | High-volume, lossy acceptable |
| `alarm` | 1 | no | Delivery reliability preferred |
| `.../online` (LWT status) | 1 | yes | `true/false` availability flag |

LWT topic:

`mamut/{site}/{device_type}/{device_id}/online`

## JSON Payload Examples (POC)

`status`:

```json
{
  "timestamp": "2026-02-15T12:00:00Z",
  "state": "IDLE",
  "last_command_id": "cmd-000123",
  "last_command_status": "OK",
  "details": {
    "zone_id": "zone-01"
  }
}
```

`command`:

```json
{
  "timestamp": "2026-02-15T12:00:01Z",
  "command": "SET_SPEED",
  "params": {
    "speed_mps": 1.2
  },
  "command_id": "cmd-000123"
}
```

`command_result`:

```json
{
  "timestamp": "2026-02-15T12:00:01Z",
  "command_id": "cmd-000123",
  "status": "OK",
  "message": "Speed accepted",
  "details": {
    "speed_mps": 1.2
  }
}
```

`telemetry`:

```json
{
  "timestamp": "2026-02-15T12:00:02Z",
  "metrics": {
    "motor_current_a": 2.4,
    "temperature_c": 37.1
  }
}
```

`alarm`:

```json
{
  "timestamp": "2026-02-15T12:00:03Z",
  "severity": "CRITICAL",
  "code": "DIV_STALL",
  "message": "Diverter actuator stalled"
}
```

## Compatibility Rules

1. Topic segments are immutable once deployed for a site.
2. New JSON fields may be added; existing fields must remain backward compatible.
3. `command_id` must be echoed in `command_result`; if `command_result` is not emitted, include it in `status`.
4. Unknown fields must be ignored by consumers (forward compatibility).
