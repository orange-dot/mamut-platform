# Repository Guidelines

## Project Structure & Module Organization
This repository is a multi-language monorepo for conveyor/diverter control.
- `firmware/`: C11 firmware (`include/mamut/` headers, `src/` implementation, `test/` C tests).
- `services/`: Rust workspace (`crates/` shared libraries, `binaries/` runnable services).
- `dashboard/`: React + TypeScript UI (`src/` app code, `public/locales/` i18n assets).
- `sim/`: Python simulation package (`mamut_sim/`) and tests in `sim/tests/`.
- `proto/`: protobuf contracts; `config/`: default/site TOML configs; `spec/`: system/protocol specs.

## Build, Test, and Development Commands
Use root `Makefile` targets for consistent workflows:
- `make all`: build firmware, services, and dashboard.
- `make test`: run all subsystem tests.
- `make sim`: install simulator package and run `pytest`.
- `make lint`: `cargo clippy` for Rust and `npm run lint` for dashboard.
- `make format`: `cargo fmt` and Prettier formatting.
- `make proto`: run `tools/proto-gen.sh`.

Useful direct commands:
- `cd services && cargo test`
- `cd firmware && cmake -B build -DMAMUT_PLATFORM=posix && cmake --build build --target test`
- `cd dashboard && npm ci && npm run build`
- `cd sim && pip install -e ".[dev]" && pytest`

## Coding Style & Naming Conventions
- Rust: format with `cargo fmt`; keep clippy clean (`cargo clippy -- -D warnings` in CI). Use `snake_case` for functions/modules and `CamelCase` for types.
- C (firmware): C11, 4-space indentation, and `mamut_*` naming for public APIs.
- TypeScript/React: ESLint + Prettier (`npm run lint`, `npm run format`); `PascalCase` components and `camelCase` variables.
- Python: PEP 8 style, 4-space indentation, `snake_case`; use `ruff` when editing simulator code.

## Testing Guidelines
- Run everything with `make test` (or `tools/test-runner.sh`).
- Add tests with every behavioral change:
- Firmware: `firmware/test/test_*.c` (register in CMake).
- Rust: unit/integration tests in each crate.
- Dashboard: Vitest tests as `*.test.ts`/`*.test.tsx` under `dashboard/src/`.
- Simulation: `sim/tests/test_*.py` with `pytest`.
- No enforced coverage percentage yet; regression tests are expected for bug fixes.

## Commit & Pull Request Guidelines
- Keep commit subjects short and imperative (history example: `Initial project structure for mamut-diverter`).
- Recommended format: `area: imperative summary` (example: `services: add telemetry event mapping`).
- Keep commits scoped to one subsystem when possible.
- PRs should include purpose, touched paths, linked issue/spec, and exact test commands run.
- Verify CI-relevant checks pass for every changed component before requesting review.
