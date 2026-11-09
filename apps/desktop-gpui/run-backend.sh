#!/bin/sh
# DBX.app keeps 4224, so the GPUI build's sidecar uses its own port.
cd "$(dirname "$0")/../.." || exit 1
DBX_PORT=4225 \
DBX_DISABLE_PASSWORD=1 \
DBX_DATA_DIR="$PWD/apps/desktop-electron/.dbx-web-data" \
./target/release/dbx-web
