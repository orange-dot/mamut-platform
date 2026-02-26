# mamut-diverter

Sistem za postanske i carinske konvejere sa omni-direktnim diverterima.

## Arhitektura

```
Dashboard (gRPC-web) -> Kontroler <- WMS Bridge -> SAP/Manhattan
                            |              |
                         Gateway     Postanski Bridge -> EDI/Carina
                            |
                     Firmware zona (PLC/MCU)
```

## Komponente

| Komponenta | Jezik | Lokacija |
|------------|-------|----------|
| Firmware | C | `firmware/` |
| Backend servisi | Rust | `services/` |
| Web dashboard | TypeScript/React | `dashboard/` |
| Simulacija | Python | `sim/` |
| Protobuf definicije | Protobuf | `proto/` |
| Specifikacije | Markdown | `spec/` |

## Brzi start

```bash
make all          # Kompajliranje svega
make test         # Pokretanje svih testova
make sim          # Pokretanje simulacije
```

## Dokumentacija

- Engleski: `docs/en/`
- Srpski: `docs/sr/`
- Specifikacije: `spec/`

## Licenca

AGPL-3.0-or-later
