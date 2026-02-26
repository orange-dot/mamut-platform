# Item Tracking

## Scope
Defines the in-memory tracking model behind `mamut.tracking.TrackingService` for Phase 3.

## Tracking Model
- Key: `item_id`
- Value: latest `ItemPosition { item_id, zone_id, timestamp }`
- Update rule: last-write-wins by event time arrival at service boundary

Tracker keeps the most recent known zone per item; historical movement is out of scope for POC.

## State Rules
- `item_id` is required and must be non-empty
- `zone_id` is required and must be non-empty
- `timestamp` is required in emitted events (auto-filled if caller omits it)

Invalid updates are rejected before entering state.

## API Semantics
- `GetItemPosition(item_id)` returns the latest known position
- Unknown item returns `NOT_FOUND`
- `StreamPositions` emits each accepted update
- No write RPC exists in proto; writes are internal (`update_position` / `insert_position`) and called by controller/gateway flows

Stream is broadcast-based; slow consumers can lag and should re-sync using `GetItemPosition`.

## Integration With Conveyor Flow
1. Identification layer accepts a scan (`ScanResult`).
2. Controller correlates scan with conveyor zone context.
3. Tracker is updated with `item_id -> zone_id`.
4. Downstream services (sorting, diverter orchestration) query current item position.

## Invariants
- One item has one current zone at a time.
- Updates are idempotent for repeated `{item_id, zone_id}` writes.
- Tracker is hardware-agnostic; it consumes service-level events, not direct MCU interfaces.
