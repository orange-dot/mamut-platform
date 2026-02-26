#!/bin/bash
# =============================================================================
# Bash script to start the Docker demo environment
# Usage: ./start.sh [--build] [--detach] [--logs] [--help]
# =============================================================================

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Flags
BUILD=false
DETACH=false
LOGS=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --build|-b)
            BUILD=true
            shift
            ;;
        --detach|-d)
            DETACH=true
            shift
            ;;
        --logs|-l)
            LOGS=true
            shift
            ;;
        --help|-h)
            echo ""
            echo "Azure Durable Functions Demo Environment"
            echo "========================================"
            echo ""
            echo "Usage: ./start.sh [options]"
            echo ""
            echo "Options:"
            echo "  --build, -b    Force rebuild of Docker images"
            echo "  --detach, -d   Run containers in background (detached mode)"
            echo "  --logs, -l     Follow container logs after starting"
            echo "  --help, -h     Show this help message"
            echo ""
            echo "Examples:"
            echo "  ./start.sh                    # Start with interactive output"
            echo "  ./start.sh --detach           # Start in background"
            echo "  ./start.sh --build --detach   # Rebuild and start in background"
            echo "  ./start.sh --detach --logs    # Start in background, then show logs"
            echo ""
            echo "Services:"
            echo "  - Azurite (Azure Storage)   : http://localhost:10000 (blob), :10001 (queue), :10002 (table)"
            echo "  - Service Bus Emulator      : localhost:5672 (AMQP), :5300 (mgmt)"
            echo "  - Cosmos DB Emulator        : https://localhost:8081, Data Explorer: http://localhost:1234"
            echo "  - Azure Functions           : http://localhost:7071"
            echo ""
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

print_header() {
    echo -e "\n${CYAN}=== $1 ===${NC}"
}

print_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

print_info() {
    echo -e "${YELLOW}[INFO]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check Docker is running
print_header "Checking Prerequisites"
if ! docker version --format '{{.Server.Version}}' > /dev/null 2>&1; then
    print_error "Docker is not running. Please start Docker first."
    exit 1
fi
DOCKER_VERSION=$(docker version --format '{{.Server.Version}}')
print_success "Docker is running (v$DOCKER_VERSION)"

# Check docker compose
if ! docker compose version --short > /dev/null 2>&1; then
    print_error "Docker Compose is not available."
    exit 1
fi
COMPOSE_VERSION=$(docker compose version --short)
print_success "Docker Compose is available (v$COMPOSE_VERSION)"

# Navigate to script directory
cd "$(dirname "$0")"

# Create .env if it doesn't exist
if [ ! -f ".env" ]; then
    print_info "Creating .env file from template..."
    cp ".env.example" ".env"
    print_success ".env file created"
fi

# Build compose command
COMPOSE_CMD="docker compose up"
if [ "$BUILD" = true ]; then
    COMPOSE_CMD="$COMPOSE_CMD --build"
fi
if [ "$DETACH" = true ]; then
    COMPOSE_CMD="$COMPOSE_CMD -d"
fi

# Start containers
print_header "Starting Demo Environment"
print_info "This may take a few minutes on first run..."
echo ""

eval $COMPOSE_CMD

if [ "$DETACH" = true ]; then
    echo ""
    print_header "Services Started"
    echo ""
    echo "Endpoints:"
    echo "  - Azure Functions API:     http://localhost:7071/api/"
    echo "  - Azurite Blob:            http://localhost:10000"
    echo "  - Azurite Queue:           http://localhost:10001"
    echo "  - Azurite Table:           http://localhost:10002"
    echo "  - Service Bus (AMQP):      localhost:5672"
    echo "  - Cosmos DB Gateway:       https://localhost:8081"
    echo "  - Cosmos DB Data Explorer: http://localhost:1234"
    echo ""
    echo "Connection Strings (for local development):"
    echo "  - Storage:     UseDevelopmentStorage=true"
    echo "  - Service Bus: Endpoint=sb://localhost;SharedAccessKeyName=RootManageSharedAccessKey;SharedAccessKey=SAS_KEY_VALUE;UseDevelopmentEmulator=true;"
    echo ""
    echo "Commands:"
    echo "  - View logs:     docker compose logs -f"
    echo "  - Stop:          docker compose down"
    echo "  - Stop & clean:  docker compose down -v"
    echo ""

    if [ "$LOGS" = true ]; then
        print_header "Following Logs (Ctrl+C to exit)"
        docker compose logs -f
    fi
fi

print_success "Done!"
