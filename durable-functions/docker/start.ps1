# =============================================================================
# PowerShell script to start the Docker demo environment
# Usage: .\start.ps1 [-Build] [-Detach] [-Logs]
# =============================================================================

param(
    [switch]$Build,      # Force rebuild of images
    [switch]$Detach,     # Run in background (detached mode)
    [switch]$Logs,       # Follow logs after starting
    [switch]$Help        # Show help
)

$ErrorActionPreference = "Stop"

# Colors for output
function Write-Header($text) { Write-Host "`n=== $text ===" -ForegroundColor Cyan }
function Write-Success($text) { Write-Host "[OK] $text" -ForegroundColor Green }
function Write-Info($text) { Write-Host "[INFO] $text" -ForegroundColor Yellow }
function Write-Err($text) { Write-Host "[ERROR] $text" -ForegroundColor Red }

if ($Help) {
    Write-Host @"

Azure Durable Functions Demo Environment
========================================

Usage: .\start.ps1 [options]

Options:
  -Build    Force rebuild of Docker images
  -Detach   Run containers in background (detached mode)
  -Logs     Follow container logs after starting
  -Help     Show this help message

Examples:
  .\start.ps1                    # Start with interactive output
  .\start.ps1 -Detach            # Start in background
  .\start.ps1 -Build -Detach     # Rebuild and start in background
  .\start.ps1 -Detach -Logs      # Start in background, then show logs

Services:
  - Azurite (Azure Storage)   : http://localhost:10000 (blob), :10001 (queue), :10002 (table)
  - Service Bus Emulator      : localhost:5672 (AMQP), :5300 (mgmt)
  - Cosmos DB Emulator        : https://localhost:8081, Data Explorer: http://localhost:1234
  - Azure Functions           : http://localhost:7071

"@
    exit 0
}

# Check Docker is running
Write-Header "Checking Prerequisites"
try {
    $dockerVersion = docker version --format '{{.Server.Version}}' 2>&1
    if ($LASTEXITCODE -ne 0) { throw "Docker is not running" }
    Write-Success "Docker is running (v$dockerVersion)"
}
catch {
    Write-Err "Docker is not running. Please start Docker Desktop first."
    exit 1
}

# Check docker-compose
try {
    $composeVersion = docker compose version --short 2>&1
    Write-Success "Docker Compose is available (v$composeVersion)"
}
catch {
    Write-Err "Docker Compose is not available."
    exit 1
}

# Navigate to docker directory
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptDir

# Create .env if it doesn't exist
if (-not (Test-Path ".env")) {
    Write-Info "Creating .env file from template..."
    Copy-Item ".env.example" ".env"
    Write-Success ".env file created"
}

# Build arguments
$composeArgs = @("compose", "up")
if ($Build) { $composeArgs += "--build" }
if ($Detach) { $composeArgs += "-d" }

# Start containers
Write-Header "Starting Demo Environment"
Write-Info "This may take a few minutes on first run..."
Write-Host ""

& docker @composeArgs

if ($LASTEXITCODE -ne 0) {
    Write-Err "Failed to start containers"
    exit 1
}

if ($Detach) {
    Write-Host ""
    Write-Header "Services Started"
    Write-Host @"

Endpoints:
  - Azure Functions API:     http://localhost:7071/api/
  - Azurite Blob:            http://localhost:10000
  - Azurite Queue:           http://localhost:10001
  - Azurite Table:           http://localhost:10002
  - Service Bus (AMQP):      localhost:5672
  - Cosmos DB Gateway:       https://localhost:8081
  - Cosmos DB Data Explorer: http://localhost:1234

Connection Strings (for local development):
  - Storage:     UseDevelopmentStorage=true
  - Service Bus: Endpoint=sb://localhost;SharedAccessKeyName=RootManageSharedAccessKey;SharedAccessKey=SAS_KEY_VALUE;UseDevelopmentEmulator=true;

Commands:
  - View logs:     docker compose logs -f
  - Stop:          docker compose down
  - Stop & clean:  docker compose down -v

"@ -ForegroundColor White

    if ($Logs) {
        Write-Header "Following Logs (Ctrl+C to exit)"
        docker compose logs -f
    }
}

Write-Success "Done!"
