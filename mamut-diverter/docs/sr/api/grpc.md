# gRPC API referenca

Autoritativne šeme su u `proto/mamut/*.proto`. Ova stranica je mapa servisa za
integratore tokom rane implementacije.

## Servisi

| Servis | Paket | Ključni RPC-ovi |
|---|---|---|
| ConveyorService | `mamut.conveyor` | `GetZoneStatus`, `SetZoneSpeed`, `StreamZoneEvents` |
| DiverterService | `mamut.diverter` | `Activate`, `Deactivate`, `GetStatus`, `SelfTest` |
| IdentificationService | `mamut.identification` | `GetScanResult`, `StreamScans` |
| TrackingService | `mamut.tracking` | `GetItemPosition`, `StreamPositions` |
| WmsService | `mamut.wms` | `GetSortDecision` |
| AlarmService | `mamut.alarms` | `GetActiveAlarms`, `AcknowledgeAlarm`, `StreamAlarms` |
| TelemetryService | `mamut.telemetry` | `GetHealth`, `StreamMetrics` |

## Napomene o ugovoru

- Za live UI/SCADA prikaz koristiti streaming RPC-ove.
- Identifikatori (`ItemId`, `ZoneId`, `DiverterId`) su neprozirni stringovi.
- Dozvoljena je samo backward-compatible evolucija proto šema.
