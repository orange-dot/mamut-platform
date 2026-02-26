#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

cd "$ROOT_DIR/services"
cargo build -p mamut-proto

echo "Proto bindings generated via mamut-proto build."
