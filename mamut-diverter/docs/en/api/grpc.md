# gRPC API Reference

Authoritative schemas are in `proto/mamut/*.proto`. This page is a service map
for integrators during early implementation.

## Services

| Service | Package | Key RPCs |
|---|---|---|
| ConveyorService | `mamut.conveyor` | `GetZoneStatus`, `SetZoneSpeed`, `StreamZoneEvents` |
| DiverterService | `mamut.diverter` | `Activate`, `Deactivate`, `GetStatus`, `SelfTest` |
| IdentificationService | `mamut.identification` | `GetScanResult`, `StreamScans` |
| TrackingService | `mamut.tracking` | `GetItemPosition`, `StreamPositions` |
| WmsService | `mamut.wms` | `GetSortDecision` |
| AlarmService | `mamut.alarms` | `GetActiveAlarms`, `AcknowledgeAlarm`, `StreamAlarms` |
| TelemetryService | `mamut.telemetry` | `GetHealth`, `StreamMetrics` |

## Contract Notes

- Use streaming methods for live UI/SCADA updates.
- IDs (`ItemId`, `ZoneId`, `DiverterId`) are opaque strings.
- Backward-compatible proto evolution only (append fields, keep field numbers).
