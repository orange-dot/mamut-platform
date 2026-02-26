# mamut-verify

Adversarial verification harness for event sourcing systems, compliance engines, and API contracts.

> **Status: Early Development** - Nothing here is verified or production-ready. This is experimental research code.

## Overview

mamut-verify is a framework for adversarial testing of distributed systems with a focus on:

- **Event sourcing systems** (EventStoreDB, Marten, Axon, Eventuous)
- **Compliance/regulatory rule engines**
- **API contract adherence**

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Controller                              │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌─────────────┐  │
│  │ Generator │ │  Checker  │ │ Minimizer │ │   Report    │  │
│  └───────────┘ └───────────┘ └───────────┘ └─────────────┘  │
│  ┌───────────┐ ┌───────────┐ ┌───────────┐                  │
│  │  Nemesis  │ │  History  │ │ TLA+/Quint│                  │
│  └───────────┘ └───────────┘ └───────────┘                  │
└─────────────────────────┬───────────────────────────────────┘
                          │ gRPC
        ┌─────────────────┼─────────────────┐
        ▼                 ▼                 ▼
   ┌─────────┐       ┌─────────┐       ┌─────────┐
   │ Agent 1 │       │ Agent 2 │       │ Agent N │
   └────┬────┘       └────┬────┘       └────┬────┘
        │                 │                 │
        ▼                 ▼                 ▼
   ┌─────────────────────────────────────────────┐
   │           System Under Test (SUT)           │
   └─────────────────────────────────────────────┘
```

## Crates

| Crate | Description | Status |
|-------|-------------|--------|
| `mamut-core` | OVC clock, operations, history types | Compiles |
| `mamut-checker` | Linearizability, sequential, causal checkers | Compiles |
| `mamut-nemesis` | Fault injection (network, clock, process) | Compiles |
| `mamut-generator` | Workload generation | Compiles |
| `mamut-minimizer` | Delta debugging (DDMIN) | Compiles |
| `mamut-orchestrator` | Container orchestration | Compiles |
| `mamut-report` | Report generation (JSON, HTML, Markdown) | Compiles |
| `mamut-history` | Event sourcing persistence | Needs fixes |
| `mamut-transport` | gRPC communication | Needs protoc |
| `mamut-ffi` | C ABI for language bindings | Compiles |

## Key Concepts

### OVC (Omniscient Verification Clock)

A 4-dimensional temporal tracking system:

1. **Controller Time** - Ground truth from the test controller
2. **Observed Time** - Wall clock as reported by nodes
3. **Vector Clock** - Causality tracking across nodes
4. **Uncertainty Interval** - Bounds on clock drift

### Consistency Models

- Linearizability (WGL algorithm)
- Sequential consistency
- Causal consistency

### Fault Injection

- Network partitions (iptables)
- Latency injection (tc/netem)
- Clock skew and jumps
- Process signals (SIGSTOP, SIGKILL, SIGTERM)

## Building

```bash
cd mamut-verify
cargo build
```

### Prerequisites

- Rust 2024 edition
- protoc (for mamut-transport)
- Docker (for container orchestration)

## License

AGPL-3.0-or-later
