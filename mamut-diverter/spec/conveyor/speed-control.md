# Speed Control

This document defines speed control behavior, units, and tuning guidance.

## Canonical Units

- API layer (`proto`): `speed_mps` in meters/second.
- Firmware/config layer: millimeters/second where needed.

Conversion:
- `mm_per_s = mps * 1000`
- `mps = mm_per_s / 1000`

## Operating Limits

Default values from `config/defaults/conveyor.toml`:
- `default_speed_mm_per_s = 500` (0.5 m/s)
- `max_speed_mm_per_s = 2000` (2.0 m/s)

Rules:
- Commanded speed must be clamped to `[0, max_speed]`.
- Negative speed is invalid in Phase 2 model.

## Control Modes

| Mode | Description | Typical trigger |
|---|---|---|
| Stopped | Target speed = 0 | planned stop, interlock, no-flow |
| Constant speed | Fixed target speed | steady throughput operation |
| Ramp profile | Controlled accel/decel | start/stop transitions |
| Fault hold | Force speed = 0 | fault/interlock active |

## Ramp and PID Guidance

Recommended POC defaults:
- Acceleration: `0.5 m/s^2`
- Deceleration: `0.7 m/s^2`
- Jerk limiting: optional, disabled by default in POC

Initial PID baseline (for velocity loop where applicable):
- `Kp = 0.8`
- `Ki = 0.2`
- `Kd = 0.0`

Interpretation for firmware tuning:
- error input: velocity error in `m/s` (`target_speed - measured_speed`)
- control output: normalized drive command (for example PWM duty `0.0..1.0`
  or equivalent drive reference unit)

Implementation notes:
- Apply anti-windup when output saturates.
- Reset integrator on transition to `STOPPED` or `FAULT`.
- Sample period target: `10 ms` control cycle for MCU loop.

## Command Handling

- `SetZoneSpeed` command updates target speed, not instantaneous speed.
- Actual speed converges via ramp/PID profile.
- Command acknowledgement should include accepted target and clamp info when applied.

## Monitoring and Alarms

- Raise warning if measured speed deviates > 10% from target for > 2 s.
- Raise critical alarm if deviation > 25% for > 1 s or drive reports fault.
- Publish periodic telemetry:
  - target speed
  - measured speed
  - motor current
  - control mode
