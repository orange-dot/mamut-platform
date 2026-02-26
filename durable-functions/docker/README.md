# Docker Demo Environment

This directory contains Docker configuration for running the Azure Durable Functions orchestration system locally for **demonstration and development purposes**.

> **Important**: This environment uses emulators and is NOT suitable for production use.

## Prerequisites

- [Docker Desktop](https://www.docker.com/products/docker-desktop/) (with Docker Compose v2)
- At least 8GB RAM available for Docker
- 10GB free disk space

## Quick Start

### Windows (PowerShell)

```powershell
cd docker
.\start.ps1 -Detach
```

### Linux / macOS

```bash
cd docker
chmod +x start.sh stop.sh
./start.sh --detach
```

## Services

| Service | Description | Ports | Endpoint |
|---------|-------------|-------|----------|
| **Azurite** | Azure Storage emulator (Blob, Queue, Table) | 10000, 10001, 10002 | `http://localhost:10000` |
| **Service Bus Emulator** | Azure Service Bus emulator | 5672 (AMQP), 5300 | `sb://localhost` |
| **Cosmos DB Emulator** | Azure Cosmos DB (Linux vNext preview) | 8081, 1234 | `https://localhost:8081` |
| **SQL Edge** | SQL Server (Service Bus backend) | 1433 | `localhost,1433` |
| **Azure Functions** | Orchestration Functions host | 7071 | `http://localhost:7071` |

## Connection Strings

### Azure Storage (Azurite)

For local development:
```
UseDevelopmentStorage=true
```

For containers on the same network:
```
DefaultEndpointsProtocol=http;AccountName=devstoreaccount1;AccountKey=Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==;BlobEndpoint=http://azurite:10000/devstoreaccount1;QueueEndpoint=http://azurite:10001/devstoreaccount1;TableEndpoint=http://azurite:10002/devstoreaccount1;
```

### Service Bus Emulator

```
Endpoint=sb://localhost;SharedAccessKeyName=RootManageSharedAccessKey;SharedAccessKey=SAS_KEY_VALUE;UseDevelopmentEmulator=true;
```

### Cosmos DB Emulator

```
AccountEndpoint=https://localhost:8081/;AccountKey=C2y6yDjf5/R+ob0N8A7Cgv30VRDJIWEHLM+4QDU5DE2nQ9nDuVTqobD4b8mGGyPMbIZnqyMsEcaGQy67XIw/Jw==;
```

> **Note**: The Cosmos DB emulator uses a well-known key for development. This is expected and secure for local use.

## Commands

### Start Environment

```bash
# Interactive mode (see all logs)
docker compose up

# Background mode
docker compose up -d

# Rebuild images first
docker compose up -d --build
```

### View Logs

```bash
# All services
docker compose logs -f

# Specific service
docker compose logs -f functions
docker compose logs -f servicebus-emulator
```

### Stop Environment

```bash
# Stop containers (keep data)
docker compose down

# Stop and remove all data
docker compose down -v
```

### Check Status

```bash
docker compose ps
```

## Configuration

### Environment Variables

Copy `.env.example` to `.env` and modify as needed:

```bash
cp .env.example .env
```

Key variables:
- `MSSQL_SA_PASSWORD`: SQL Server password (required for Service Bus emulator)

### Service Bus Configuration

Edit `config/servicebus-config.json` to customize:
- Queues (e.g., `device-events`)
- Topics and subscriptions
- Message properties (TTL, lock duration, etc.)

## Testing the Environment

### 1. Check Services Are Running

```bash
docker compose ps
```

All services should show "healthy" or "running" status.

### 2. Test Azure Functions

```bash
# Health check
curl http://localhost:7071/api/health

# Start a workflow (example)
curl -X POST http://localhost:7071/api/workflows \
  -H "Content-Type: application/json" \
  -d '{"workflowType": "DeviceOnboarding", "entityId": "device-001"}'
```

### 3. Access Cosmos DB Data Explorer

Open http://localhost:1234 in your browser.

### 4. Connect with Azure Storage Explorer

1. Open Azure Storage Explorer
2. Add connection → Local storage emulator
3. Use display name: "Docker Azurite"

## Troubleshooting

### Service Bus Emulator Won't Start

The Service Bus emulator depends on SQL Edge starting first. If it fails:

```bash
# Check SQL Edge logs
docker compose logs sql-edge

# Restart just Service Bus
docker compose restart servicebus-emulator
```

### Cosmos DB Certificate Errors

The Cosmos DB emulator uses a self-signed certificate. For .NET applications:

```csharp
// In development only
CosmosClientOptions options = new()
{
    HttpClientFactory = () => new HttpClient(new HttpClientHandler
    {
        ServerCertificateCustomValidationCallback =
            HttpClientHandler.DangerousAcceptAnyServerCertificateValidator
    })
};
```

### Port Conflicts

If ports are already in use:

1. Check what's using the port: `netstat -ano | findstr :8081`
2. Either stop the conflicting service or modify `docker-compose.yml` port mappings

### Out of Memory

The emulators require significant memory. If containers keep restarting:

1. Increase Docker memory allocation in Docker Desktop settings
2. Minimum recommended: 8GB RAM for Docker

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Docker Network: orchestration-network         │
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │   Azurite    │  │  Cosmos DB   │  │  Service Bus Emulator │  │
│  │  (Storage)   │  │  (NoSQL DB)  │  │      (Messaging)      │  │
│  │ :10000-10002 │  │  :8081,:1234 │  │     :5672,:5300       │  │
│  └──────────────┘  └──────────────┘  └──────────┬────────────┘  │
│         │                │                       │               │
│         │                │               ┌───────┴───────┐       │
│         │                │               │   SQL Edge    │       │
│         │                │               │    :1433      │       │
│         │                │               └───────────────┘       │
│         │                │                                       │
│         └────────────────┼───────────────────────┐               │
│                          │                       │               │
│                   ┌──────┴───────────────────────┴────┐         │
│                   │        Azure Functions            │         │
│                   │    (Orchestration.Functions)      │         │
│                   │           :7071 → :80             │         │
│                   └───────────────────────────────────┘         │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Limitations

### Service Bus Emulator
- No JMS streaming support
- Partitioned entities not supported
- No on-the-fly management operations via SDK
- Data does not persist after container restart

### Cosmos DB Emulator (Linux vNext)
- Only supports NoSQL API in gateway mode
- Subset of features compared to Windows emulator
- Preview - may have bugs

### General
- **Not for production use** - emulators only
- Durable Task Scheduler is not available in Docker (uses Azurite storage provider instead)

## Resources

- [Azure Service Bus Emulator Documentation](https://learn.microsoft.com/en-us/azure/service-bus-messaging/test-locally-with-service-bus-emulator)
- [Azure Cosmos DB Emulator](https://learn.microsoft.com/en-us/azure/cosmos-db/emulator-linux)
- [Azurite Storage Emulator](https://learn.microsoft.com/en-us/azure/storage/common/storage-use-azurite)
- [Azure Functions Docker](https://hub.docker.com/_/microsoft-azure-functions-dotnet-isolated)
