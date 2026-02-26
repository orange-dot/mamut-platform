# Local Development Guide

Complete guide for setting up and running the Azure Durable Functions orchestration system locally.

## Prerequisites

### Required Software

| Software | Version | Purpose |
|----------|---------|---------|
| [Docker Desktop](https://www.docker.com/products/docker-desktop/) | Latest | Container runtime with Compose v2 |
| [.NET SDK](https://dotnet.microsoft.com/download) | 9.0+ | Building and testing .NET projects |
| [Node.js](https://nodejs.org/) | 20+ LTS | UI development |
| [Git](https://git-scm.com/) | Latest | Version control |

### Optional Tools

- [Azure Storage Explorer](https://azure.microsoft.com/features/storage-explorer/) - Browse Azurite data
- [VS Code](https://code.visualstudio.com/) with C# Dev Kit extension
- [Azure Data Studio](https://azure.microsoft.com/products/data-studio/) - SQL Edge management

### System Requirements

- **RAM**: Minimum 8GB available for Docker (16GB recommended)
- **Disk**: 10GB free space for images and data
- **OS**: Windows 10/11 with WSL2, macOS, or Linux

## Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/orange-dot/durable-functions.git
cd durable-functions
```

### 2. Start Docker Environment

**Windows (PowerShell):**
```powershell
cd docker
.\start.ps1 -Detach
```

**Linux/macOS:**
```bash
cd docker
./start.sh --detach
```

**Or manually:**
```bash
cd docker
docker compose up -d --build
```

### 3. Verify Services

Wait 30-60 seconds for all services to initialize, then verify:

```bash
docker compose ps
```

Expected output - all services should be running/healthy:
```
NAME                      STATUS
azurite                   running (healthy)
cosmosdb-emulator         running
orchestration-functions   running
servicebus-emulator       running
sql-edge                  running (healthy)
```

### 4. Test the API

```bash
# Health check
curl http://localhost:7071/api/health

# Start a demo workflow
curl -X POST http://localhost:7071/api/workflows \
  -H "Content-Type: application/json" \
  -d '{
    "workflowType": "DeviceOnboarding",
    "entityId": "device-001",
    "data": {"deviceName": "Test Device"}
  }'
```

## Service Endpoints

| Service | Port | URL | Purpose |
|---------|------|-----|---------|
| Azure Functions | 7071 | http://localhost:7071/api | REST API |
| Azurite Blob | 10000 | http://localhost:10000 | Blob storage |
| Azurite Queue | 10001 | http://localhost:10001 | Queue storage |
| Azurite Table | 10002 | http://localhost:10002 | Table storage |
| Cosmos DB | 8081 | https://localhost:8081 | Document database |
| Cosmos DB Explorer | 1234 | http://localhost:1234 | Web UI |
| Service Bus | 5672 | amqp://localhost:5672 | Message broker |
| SQL Edge | 1433 | localhost,1433 | Service Bus backend |

## API Reference

### Start Workflow

```http
POST /api/workflows
Content-Type: application/json

{
  "workflowType": "DeviceOnboarding",
  "entityId": "device-001",
  "idempotencyKey": "unique-key-123",
  "data": {
    "deviceName": "My Device",
    "location": "Building A"
  }
}
```

Response:
```json
{
  "instanceId": "workflow-abc123",
  "statusQueryGetUri": "http://localhost:7071/api/workflows/workflow-abc123"
}
```

### Get Workflow Status

```http
GET /api/workflows/{instanceId}
```

Response:
```json
{
  "instanceId": "workflow-abc123",
  "status": "Running",
  "workflowType": "DeviceOnboarding",
  "createdAt": "2025-01-15T10:30:00Z",
  "lastUpdatedAt": "2025-01-15T10:30:05Z",
  "input": { ... },
  "output": { ... }
}
```

### List Workflows

```http
GET /api/workflows?status=Running&top=50
```

### Raise Event

```http
POST /api/workflows/{instanceId}/events/{eventName}
Content-Type: application/json

{
  "approved": true,
  "approvedBy": "admin@example.com"
}
```

### Terminate Workflow

```http
DELETE /api/workflows/{instanceId}
```

## Building the Solution

### .NET Projects

```bash
# Restore and build all projects
cd src
dotnet build Orchestration.sln

# Run tests
dotnet test Orchestration.sln

# Build specific project
dotnet build Orchestration.Functions/Orchestration.Functions.csproj
```

### UI Project

```bash
cd ui

# Install dependencies
npm install

# Start development server
npm run dev

# Build for production
npm run build

# Run linter
npm run lint
```

## Development Workflow

### Making Changes to Functions

1. Make code changes in `src/Orchestration.*`
2. Rebuild the solution:
   ```bash
   dotnet build src/Orchestration.sln
   ```
3. Restart the functions container:
   ```bash
   docker compose restart orchestration-functions
   ```

Or use hot reload by running functions locally (see below).

### Running Functions Locally (Outside Docker)

For faster iteration, run functions directly:

1. Start infrastructure only:
   ```bash
   cd docker
   docker compose up -d azurite sql-edge servicebus-emulator cosmosdb-emulator
   ```

2. Run functions locally:
   ```bash
   cd src/Orchestration.Functions
   func start
   ```

### Viewing Logs

```bash
# All services
docker compose logs -f

# Specific service
docker compose logs -f orchestration-functions

# Filter errors
docker compose logs orchestration-functions 2>&1 | grep -i error
```

### Accessing Data

**Cosmos DB Explorer:**
Open http://localhost:1234 in browser

**Azure Storage Explorer:**
1. Open Azure Storage Explorer
2. Add connection → Local storage emulator
3. Connect to `http://127.0.0.1:10000`

**SQL Edge:**
```bash
# Connect via sqlcmd in container
docker exec -it sql-edge /opt/mssql-tools/bin/sqlcmd -S localhost -U sa -P "YourStrong@Passw0rd"
```

## Configuration

### Environment Variables

Copy and customize the environment file:

```bash
cd docker
cp .env.example .env
```

Key variables:
- `MSSQL_SA_PASSWORD` - SQL Edge password (required)
- `FUNCTIONS_WORKER_RUNTIME` - Should be `dotnet-isolated`

### Workflow Definitions

Workflow definitions are stored in `docker/config/demo-workflow.json`. The system loads this on startup.

To modify:
1. Edit `docker/config/demo-workflow.json`
2. Restart functions: `docker compose restart orchestration-functions`

### Service Bus Queues/Topics

Configure in `docker/config/servicebus-config.json`:

```json
{
  "UserConfig": {
    "Queues": [
      {
        "Name": "device-events",
        "Properties": {
          "RequiresSession": false
        }
      }
    ]
  }
}
```

## Troubleshooting

### Service Bus Emulator Issues

**Symptom:** Service Bus emulator restarts repeatedly

**Solution:**
```bash
# Check SQL Edge is healthy first
docker compose logs sql-edge

# Restart Service Bus after SQL is ready
docker compose restart servicebus-emulator
```

### Cosmos DB Certificate Errors

**Symptom:** SSL/TLS errors connecting to Cosmos DB

**Solution:** The emulator uses a self-signed certificate. For development, certificate validation is disabled in the code. If connecting externally, trust the certificate or disable validation.

### Port Conflicts

**Symptom:** Container fails to start with "port already in use"

**Solution:**
```powershell
# Find what's using the port (Windows)
netstat -ano | findstr :7071

# Kill the process or change port in docker-compose.yml
```

### Out of Memory

**Symptom:** Containers keep restarting, especially Cosmos DB

**Solution:**
1. Increase Docker memory in Docker Desktop → Settings → Resources
2. Minimum 8GB, recommended 12-16GB
3. Close other memory-intensive applications

### Functions Not Starting

**Symptom:** Functions container exits or shows startup errors

**Solution:**
```bash
# Check logs for specific error
docker compose logs orchestration-functions

# Rebuild the image
docker compose build orchestration-functions

# Restart with fresh build
docker compose up -d --build orchestration-functions
```

### Reset Everything

Nuclear option - reset all data and containers:

```bash
cd docker
docker compose down -v
docker system prune -f
docker compose up -d --build
```

## Project Structure

```
azure-durable/
├── docker/                      # Docker environment
│   ├── config/                  # Configuration files
│   │   ├── demo-workflow.json   # Sample workflow definition
│   │   ├── servicebus-config.json
│   │   └── local.settings.json
│   ├── docker-compose.yml       # Main compose file
│   ├── Dockerfile.functions     # Functions image
│   └── start.ps1/start.sh       # Helper scripts
├── docs/                        # Documentation
├── src/                         # .NET source code
│   ├── Orchestration.Core/      # Core library (workflow interpreter)
│   ├── Orchestration.Functions/ # Azure Functions host
│   ├── Orchestration.Infrastructure/ # Data access, external services
│   └── Orchestration.Tests/     # Unit tests
├── ui/                          # React frontend
│   ├── src/
│   │   ├── api/                 # API client
│   │   ├── auth/                # Demo authentication
│   │   ├── components/          # React components
│   │   └── hooks/               # Custom hooks
│   └── package.json
└── opus45design.md              # Architecture design document
```

## Next Steps

- Read [Architecture Documentation](./ARCHITECTURE.md) for system design
- Review [UI Plan](./ui-plan.md) for frontend roadmap
- Check [Implementation Plan](../IMPLEMENTATION_PLAN.md) for development phases
