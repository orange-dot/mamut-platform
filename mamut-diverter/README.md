# mamut-diverter

Full-stack postal and customs conveyor system with omni-directional diverters.

## Architecture

```
Dashboard (gRPC-web) -> Controller <- WMS Bridge -> SAP/Manhattan
                            |              |
                         Gateway     Postal Bridge -> EDI/Customs
                            |
                     Zone firmware (PLC/MCU)
```

## Components

| Component | Language | Location |
|-----------|----------|----------|
| Firmware | C | `firmware/` |
| Backend services | Rust | `services/` |
| Web dashboard | TypeScript/React | `dashboard/` |
| Simulation | Python | `sim/` |
| Protobuf defs | Protobuf | `proto/` |
| Specifications | Markdown | `spec/` |

## Quick start

```bash
make all          # Build everything
make test         # Run all tests
make sim          # Run simulation
```

## Documentation

- English: `docs/en/`
- Serbian: `docs/sr/`
- Specifications: `spec/`

## License

AGPL-3.0-or-later
