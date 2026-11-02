#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
APP_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"
cargo build --release -p dbx-web \
  --no-default-features \
  --features duckdb-sidecar,dynamodb,mq-admin,dbx-core/sqlite-bundled
mkdir -p "$APP_DIR/backend"
cp target/release/dbx-web "$APP_DIR/backend/dbx-web"
echo "dbx-web -> $APP_DIR/backend/dbx-web"
