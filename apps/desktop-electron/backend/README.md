# dbx-web sidecar

The Electron app does **not** reimplement DBX's database layer. It starts the DBX web backend (`crates/dbx-web` from the fork) on `127.0.0.1` and loads the frontend from it.

## Development

1. Build the backend from the fork root:

```bash
cargo build --release -p dbx-web
```

2. Put the binary here:

```bash
cp target/release/dbx-web apps/desktop-electron/backend/dbx-web
```

Or point at it directly:

```bash
DBX_WEB_BIN=/absolute/path/to/dbx-web pnpm dev:backend
```

3. In another terminal:

```bash
pnpm dev
```

The Vite dev server proxies `/api` to `http://127.0.0.1:4224`.

## Production

Electron's main process starts `resources/dbx-web` with:

- `DBX_PORT` — random free port
- `DBX_BIND_ADDR=127.0.0.1`
- `DBX_DISABLE_PASSWORD=1` — local-only sidecar, no login
- `DBX_DATA_DIR=<app userData>/dbx-web`
- `DBX_STATIC_DIR=<electron renderer dist>`

The app loads `http://127.0.0.1:<port>/`, so the UI and `/api/*` are same-origin and DBX's sessions/cookies work without CORS changes.

## Fetch a release build instead of compiling

DBX publishes `browser-static` bundles per OS/arch on GitHub Releases. Extract the `dbx-web` binary and put it at `backend/dbx-web`. Do not commit the binary; `.gitignore` excludes it.

## Upgrading DBX

The backend is taken from the fork's `crates/dbx-web`; the Electron app only calls its HTTP API. After merging `upstream/main`, rebuild/download a new `dbx-web`. No frontend rewrite is required.
