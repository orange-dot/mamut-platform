# mamut-diverter

## Project overview
Full-stack postal/customs conveyor system with omni-directional diverters.

## Languages
- Firmware: C (CMake)
- Services: Rust (Cargo workspace in services/)
- Dashboard: TypeScript/React (Vite, in dashboard/)
- Simulation: Python (in sim/)
- Protocols: Protobuf/gRPC (in proto/)

## Build
- Top-level: `make all`
- Firmware: `cd firmware && cmake -B build && cmake --build build`
- Services: `cd services && cargo build`
- Dashboard: `cd dashboard && npm install && npm run dev`
- Simulation: `cd sim && pip install -e . && pytest`

## Conventions
- Documentation: bilingual (English primary, Serbian .sr.md)
- Specs in spec/ are source of truth
- Config: TOML in config/defaults/, per-site overrides in config/sites/
- Diverter types are pluggable (C: function pointer table, Rust: async trait)
- All proto files under proto/mamut/
- Tests: Unity (C), cargo test (Rust), pytest (Python), Playwright (dashboard)
