#!/usr/bin/env bash
set -euo pipefail
echo "Running all tests..."
cd "$(dirname "$0")/.."
make test
