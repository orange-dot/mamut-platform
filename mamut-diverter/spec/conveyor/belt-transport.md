# Belt Transport

This document defines belt transport assumptions for simulation and firmware.

## Physical Model (POC)

Each belt zone is modeled by:
- `length_m`
- current speed
- occupancy state

Travel-time estimate:
- `travel_time_s = length_m / speed_mps` (for `speed_mps > 0`)

This model is sufficient for routing, handoff timing, and divergence decisions.

## Belt Types (Reference)

| Type | Typical use | Notes |
|---|---|---|
| Flat belt | General parcel transport | Simple model, low complexity |
| Modular belt | Heavy-duty segments | Better durability, slightly higher inertia |
| Friction-top variants | Incline/decline support | Extra constraints on acceleration and load |

POC assumes flat-belt behavior unless explicitly configured otherwise.

## Load and Throughput Assumptions

- Item spacing is managed by upstream scheduling and zone readiness.
- Zone occupancy is binary in POC model (`item_present`), not continuous density.
- High-density and accumulation physics are deferred to later phases.

## Tension and Mechanical Health Monitoring

Minimum telemetry for belt health:
- motor current trend
- slip indicator (estimated from speed mismatch)
- maintenance counters (runtime hours, starts/stops)

Alarm recommendations:
- warning on sustained current drift above baseline
- critical on stall/slip conditions with transfer impact

## Sensor Placement Guidelines

- Entry sensor near upstream boundary.
- Exit sensor near downstream transfer point.
- Optional midpoint sensor for long zones or high-speed lanes.

Sensor signals drive:
- occupancy updates
- handoff confirmation
- timeout/fault detection

## Simulation Alignment

Simulator layouts (`sim/layouts/*.yaml`) define zone lengths and diverter placement.
Runtime behavior should preserve these invariants:

1. Item cannot teleport between non-adjacent zones.
2. Transfer requires downstream availability.
3. Faulted zone blocks downstream admission unless explicit bypass policy exists.
