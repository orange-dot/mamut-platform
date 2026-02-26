# WMS Interface

## Scope
Defines the WMS integration contract used by controller and SCADA clients via `mamut.wms.WmsService`.

## gRPC Contract
- RPC: `GetSortDecision(GetSortDecisionRequest) -> SortDecision`
Request fields:
- `item_id` (required): immutable parcel identifier
- `barcode` (required unless item override exists): decoded scanner payload

Response fields:
- `item_id`: echoed request item
- `target`: `DIRECTION_LEFT | DIRECTION_RIGHT | DIRECTION_STRAIGHT`
- `destination`: logical chute/lane code used by downstream routing

Unknown routing inputs return `NOT_FOUND`. Invalid inputs return `INVALID_ARGUMENT`.

## Decision Resolution Order
1. Item override lookup (`item_id` exact match) for pre-planned exceptions.
2. Barcode prefix rule matching (case-insensitive).
3. If no match exists, return `NOT_FOUND`.

Current default prefix rules in the POC:
- `RS* -> postal-rs -> LEFT`
- `EU* -> export-eu -> RIGHT`
- `LOC* -> local-sort -> STRAIGHT`

## Adapter Patterns
### SAP Adapter
- Pulls shipping destination/order class.
- Converts ERP destination groups into internal `destination` codes.
- Updates item overrides before parcels reach diverter decision point.

### Manhattan Adapter
- Consumes wave/manifest routing feed.
- Maps wave lane assignment to `target` direction + `destination`.
- Can run as periodic sync or event-driven webhook consumer.

## Ownership Boundary
- `mamut-wms` crate provides read/query API for routing decisions.
- Write-side updates (`set_item_override`, barcode prefix rules) are internal service operations, called by bridge/controller integrations, not exposed as public write RPC in proto.
