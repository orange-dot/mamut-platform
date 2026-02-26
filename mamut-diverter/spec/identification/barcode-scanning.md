# Barcode Scanning

## Scope
This document defines scanner behavior for the POC pipeline (simulated cameras now, MCU-integrated scanners later).  
Output must conform to `mamut.identification.ScanResult`.

## Supported Barcode Types
- `Code128` (primary, default for parcels and tote labels)
- `DataMatrix` (optional in POC, required in production variants)
- `QR` (optional, accepted for test labels)

Unknown symbologies are ignored and should not emit `ScanResult`.

## Data Contract
Each accepted scan publishes:
- `item_id`: stable identifier from decode pipeline (required)
- `barcode`: raw decoded payload (required)
- `confidence`: normalized float `0.0..=1.0` (required)
- `timestamp`: UTC event time (required)

`item_id` and `barcode` must be non-empty after trim.

## Confidence Thresholds
- `>= 0.90`: accept immediately
- `0.75..0.89`: accept with warning metric (`scan_low_confidence_total`)
- `< 0.75`: reject and retry

POC services enforce only range validation (`0.0..=1.0`); policy thresholds are applied by controller/gateway logic.

## Retry Logic
- Max retries per item at one scan point: `3`
- Retry backoff: `50 ms`, `100 ms`, `200 ms`
- If all retries fail, emit operational alarm (`Missing scan`) and continue transport with fallback routing policy

Retry coordination must avoid overwriting a previously accepted high-confidence result for the same item at the same station.  
Current `ScannerService` storage is last-write-wins by `item_id`; station-aware retry policy is enforced in controller/gateway.

## Streaming Behavior
`IdentificationService.StreamScans` streams accepted scans only.  
Lagged subscribers may miss older samples; consumers must treat stream as eventually consistent and use `GetScanResult` for current state.

## Write Path
Proto defines read/stream RPCs only. New scans are written internally by controller/gateway via domain methods (`record_scan` / `insert_scan`), not by external gRPC write RPC.
