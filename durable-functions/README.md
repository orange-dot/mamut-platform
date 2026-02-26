# Azure Durable Functions Orchestration System

A modular, event-driven workflow orchestration system built on Azure Durable Functions with a JSON-based workflow definition language.

## Features

- **JSON Workflow Definitions** - Define workflows using a declarative JSON schema inspired by AWS Step Functions
- **Event-Driven Architecture** - Process events via Azure Service Bus with entity-based accumulation
- **Durable Orchestrations** - Reliable, long-running workflows with automatic state persistence
- **Activity Registry** - Extensible activity system for database operations, external APIs, and notifications
- **Compensation Support** - Built-in saga pattern support for failure recovery
- **Real-Time Monitoring** - REST API for workflow status and management

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────────┐
│  Service Bus    │────▶│  Event Ingress   │────▶│  Entity Accumulator │
│  (Events)       │     │  Function        │     │  (Durable Entity)   │
└─────────────────┘     └──────────────────┘     └──────────┬──────────┘
                                                            │
                                                            ▼
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────────┐
│  REST API       │────▶│  Workflow        │◀───│  Workflow           │
│  (HTTP Trigger) │     │  Orchestrator    │     │  Interpreter        │
└─────────────────┘     └────────┬─────────┘     └─────────────────────┘
                                 │
                    ┌────────────┼────────────┐
                    ▼            ▼            ▼
             ┌──────────┐ ┌──────────┐ ┌──────────┐
             │ Database │ │ External │ │  Notify  │
             │ Activity │ │   API    │ │ Activity │
             └──────────┘ └──────────┘ └──────────┘
```

## Quick Start

### Prerequisites

- Docker Desktop with Docker Compose v2
- .NET 9.0 SDK (for development)
- Node.js 20+ (for UI development)

### Run with Docker

```bash
# Clone the repository
git clone https://github.com/orange-dot/durable-functions.git
cd durable-functions

# Start all services
cd docker
docker compose up -d --build

# Wait ~60 seconds for initialization, then test
curl http://localhost:7071/api/health
```

### Start a Workflow

```bash
curl -X POST http://localhost:7071/api/workflows \
  -H "Content-Type: application/json" \
  -d '{
    "workflowType": "DeviceOnboarding",
    "entityId": "device-001",
    "data": {"deviceName": "Test Device"}
  }'
```

### Check Workflow Status

```bash
curl http://localhost:7071/api/workflows/{instanceId}
```

## Workflow Definition Example

```json
{
  "id": "DeviceOnboarding",
  "version": "1.0",
  "name": "Device Onboarding Workflow",
  "startAt": "ValidateDevice",
  "states": {
    "ValidateDevice": {
      "type": "Task",
      "activity": "ValidateDevice",
      "next": "CheckApprovalRequired"
    },
    "CheckApprovalRequired": {
      "type": "Choice",
      "choices": [
        {
          "condition": {
            "variable": "$.requiresApproval",
            "operator": "equals",
            "value": true
          },
          "next": "WaitForApproval"
        }
      ],
      "default": "CreateRecord"
    },
    "WaitForApproval": {
      "type": "Wait",
      "event": "ApprovalReceived",
      "next": "CreateRecord"
    },
    "CreateRecord": {
      "type": "Task",
      "activity": "CreateRecord",
      "next": "Success"
    },
    "Success": {
      "type": "Succeed",
      "output": {
        "status": "completed"
      }
    }
  }
}
```

## Project Structure

```
├── docker/                  # Docker development environment
├── docs/                    # Documentation
│   ├── ARCHITECTURE.md      # System architecture
│   ├── LOCAL_DEVELOPMENT.md # Development guide
│   └── ui-plan.md           # UI roadmap
├── src/
│   ├── Orchestration.Core/         # Workflow interpreter & models
│   ├── Orchestration.Functions/    # Azure Functions host
│   ├── Orchestration.Infrastructure/ # Data access layer
│   └── Orchestration.Tests/        # Unit tests
├── ui/                      # React frontend (TanStack)
└── opus45design.md          # Design document
```

## Development

See [Local Development Guide](docs/LOCAL_DEVELOPMENT.md) for detailed setup instructions.

```bash
# Build .NET solution
dotnet build src/Orchestration.sln

# Run tests
dotnet test src/Orchestration.sln

# Start UI development server
cd ui && npm install && npm run dev
```

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/workflows` | Start a new workflow |
| GET | `/api/workflows` | List workflows |
| GET | `/api/workflows/{id}` | Get workflow status |
| POST | `/api/workflows/{id}/events/{name}` | Raise event |
| DELETE | `/api/workflows/{id}` | Terminate workflow |

## Technology Stack

- **Runtime**: .NET 9.0, Azure Functions v4 (Isolated Worker)
- **Orchestration**: Azure Durable Functions with MSSQL storage
- **Messaging**: Azure Service Bus
- **Storage**: Azure Cosmos DB, Azurite (Blob/Queue/Table)
- **Frontend**: React 18, TanStack (Router, Query), Tailwind CSS
- **Testing**: xUnit, Moq

## Docker Services

| Service | Port | Purpose |
|---------|------|---------|
| Azure Functions | 7071 | API Host |
| Azurite | 10000-10002 | Storage Emulator |
| Service Bus Emulator | 5672 | Message Broker |
| Cosmos DB Emulator | 8081 | Document Database |
| SQL Edge | 1433 | Durable Task Storage |

## License

MIT

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `dotnet test`
5. Submit a pull request
