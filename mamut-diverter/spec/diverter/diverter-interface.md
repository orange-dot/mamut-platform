# Diverter Interface

Generic diverter contract. All diverter types must implement this interface.

## Operations

- `activate(target: DivertTarget)` — Activate diversion to target
- `deactivate()` — Return to pass-through
- `get_status() -> DiverterStatus` — Current status
- `self_test() -> TestResult` — Run self-test diagnostics

## Status

- Idle
- Active(target)
- Fault(code)
- Testing

## Pluggable pattern

### Firmware (C)
Function pointer table `mamut_diverter_ops_t`.

### Services (Rust)
`DiverterDriver` async trait.

### Adding a new diverter type
1. One new `.c` file in firmware/src/diverters/
2. One new `.rs` file in services/crates/mamut-diverter/src/drivers/
3. One spec profile in spec/diverter/
4. Config entry in config/defaults/diverter.toml
