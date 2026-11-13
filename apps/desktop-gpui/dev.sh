#!/bin/sh
# Start the dbx-web sidecar for local development. The packaged app embeds and
# manages its own sidecar, so this script is only needed for `cargo run`.
set -eu
cd "$(dirname "$0")/../.." || exit 1

sh apps/desktop-gpui/run-backend.sh &
backend_pid=$!
trap 'kill "$backend_pid" 2>/dev/null || true' EXIT INT TERM

i=0
until nc -z 127.0.0.1 4225 2>/dev/null || [ "$i" -ge 25 ]; do
  i=$((i + 1))
  sleep 0.2
done

cd apps/desktop-gpui
cargo run --release
