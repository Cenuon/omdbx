#!/bin/sh
# DBX.app keeps 4224, so the GPUI build's sidecar uses its own port.
cd "$(dirname "$0")/../.." || exit 1

if [ ! -x ./target/release/dbx-web ]; then
  echo "dbx-web backend not found at ./target/release/dbx-web" >&2
  echo "Build it first:" >&2
  echo "  make build-backend" >&2
  echo "or:" >&2
  echo "  cargo build --release -p dbx-web --no-default-features --features duckdb-sidecar,dynamodb,mq-admin,dbx-core/sqlite-bundled" >&2
  exit 1
fi

export DBX_PORT=4225
export DBX_BIND_ADDR=127.0.0.1
export DBX_DISABLE_PASSWORD=1
export DBX_DATA_DIR="$PWD/apps/desktop-gpui/.dbx-web-data"

# exec so callers that track this script's PID (see dev.sh) actually signal dbx-web.
exec ./target/release/dbx-web
