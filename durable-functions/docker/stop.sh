#!/bin/bash
# =============================================================================
# Bash script to stop the Docker demo environment
# Usage: ./stop.sh [--clean] [--help]
# =============================================================================

set -e

CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

CLEAN=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --clean|-c)
            CLEAN=true
            shift
            ;;
        --help|-h)
            echo ""
            echo "Stop Azure Durable Functions Demo Environment"
            echo "============================================="
            echo ""
            echo "Usage: ./stop.sh [options]"
            echo ""
            echo "Options:"
            echo "  --clean, -c   Remove volumes (clears all persistent data)"
            echo "  --help, -h    Show this help message"
            echo ""
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

cd "$(dirname "$0")"

echo -e "\n${CYAN}=== Stopping Demo Environment ===${NC}"

COMPOSE_CMD="docker compose down"
if [ "$CLEAN" = true ]; then
    COMPOSE_CMD="$COMPOSE_CMD -v"
    echo -e "${YELLOW}[INFO]${NC} Will remove volumes (persistent data)"
fi

eval $COMPOSE_CMD

echo -e "${GREEN}[OK]${NC} Environment stopped"
