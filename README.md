# Mamut Platform

> **Note**: This is an experimental consolidation created as part of agent development work.

Mamut Platform is a unified repository containing all four core components of the Mamut ecosystem. All code is included locally in a single repository structure for easier development, testing, and deployment.

## Components

### 1. **mamut-diverter/**
Multi-stack component built with Rust, C, TypeScript, and Python. Handles traffic diversion and routing logic.

- Original Repository: https://github.com/orange-dot/mamut-diverter.git
- Primary Languages: Rust, C, TypeScript, Python
- Directory: `./mamut-diverter/`

### 2. **mamut-lab/**
Research and experimental component focused on Rust-based implementations and prototypes.

- Original Repository: https://github.com/orange-dot/mamut-lab.git
- Primary Language: Rust
- Directory: `./mamut-lab/`

### 3. **cosmosflink/**
Big data and stream processing component leveraging Apache Flink ecosystem.

- Original Repository: https://github.com/orange-dot/cosmosflink.git
- Primary Languages: Java, Scala, C#
- Directory: `./cosmosflink/`

### 4. **durable-functions/**
Cloud-native serverless orchestration component using Azure Durable Functions.

- Original Repository: https://github.com/orange-dot/durable-functions.git
- Primary Languages: .NET, Go, React
- Directory: `./durable-functions/`

## Getting Started

### Clone the platform

```bash
git clone https://github.com/orange-dot/mamut-platform.git
cd mamut-platform
```

All components are included - no need for submodule initialization.

## Project Structure

```
mamut-platform/
├── README.md                    # This file
├── mamut-diverter/              # Rust/C/TS/Python traffic routing
│   ├── services/
│   ├── dashboard/
│   ├── docker/
│   ├── proto/
│   └── ...
├── mamut-lab/                   # Rust experimental components
│   ├── mamut-verify/
│   ├── verification-products-research.md
│   └── ...
├── cosmosflink/                 # Java/Scala/C# stream processing
│   ├── flink-job-java/
│   ├── flink-job-scala/
│   ├── producer/
│   ├── consumer/
│   └── ...
└── durable-functions/           # .NET/Go/React serverless orchestration
    ├── src/
    ├── ui/
    ├── simulator/
    ├── docker/
    └── ...
```

## Architecture Overview

The Mamut Platform is organized as a unified ecosystem:

- **Diverter**: Entry point and traffic management
- **Lab**: Experimentation and R&D
- **CosmosFlink**: Data processing backbone
- **Durable Functions**: Serverless orchestration and state management

## Development

Each component has its own development setup, build process, and dependencies. Refer to the individual component READMEs for:

- Setup instructions
- Build requirements
- Testing procedures
- Deployment guidelines

### Quick Links

- [mamut-diverter README](./mamut-diverter/README.md)
- [mamut-lab README](./mamut-lab/README.md)
- [cosmosflink README](./cosmosflink/README.md)
- [durable-functions README](./durable-functions/README.md)

## Contributing

Each component maintains its own contribution guidelines and CI/CD pipelines. Please refer to individual component documentation for contribution instructions.

## License

Please refer to individual component repositories for their specific license information.
