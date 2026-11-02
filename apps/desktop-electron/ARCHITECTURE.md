# Architecture

```text
Cenuon/dbx (fork)
├── upstream: t8y2/dbx
├── crates/dbx-web              # DBX backend, unchanged
├── crates/dbx-*                # DBX database engine, unchanged
├── src-tauri/                  # DBX desktop backend, unchanged
└── apps/desktop-electron/      # our frontend, additive
    ├── src/main/               # Electron main + dbx-web sidecar
    ├── src/renderer/           # SolidJS UI
    └── backend/                # local dbx-web binary (ignored)
```

## Runtime

```text
Electron main
  ├── spawn dbx-web on 127.0.0.1:<free port>
  │     DBX_STATIC_DIR = Electron renderer dist
  │     DBX_DATA_DIR   = <userData>/dbx-web
  │     DBX_DISABLE_PASSWORD = 1
  └── BrowserWindow.loadURL(http://127.0.0.1:<port>/)
```

The UI and `/api/*` share an origin, so DBX's cookie sessions, SSE, and downloads work without CORS changes.

## Dev

```bash
# terminal 1
pnpm dev:backend      # dbx-web on :4224

# terminal 2
pnpm dev              # Vite + Electron; /api proxied to :4224
```

`src/renderer/src/dbx/client.ts` is the small HTTP bridge. It currently covers connections, databases, schemas, tables, and query execution. The sidebar shows whether the backend is reachable; the first vertical slice to wire is connection list → schema tree → query → results.

## Why not replace DBX's Tauri/Vue app?

That would make every upstream frontend update a conflict. Keeping the backend untouched and adding an Electron client means:

- upstream `git merge upstream/main` stays mostly clean;
- DBX database drivers and migrations keep improving;
- our UI can evolve independently;
- the cost is a second desktop shell and an HTTP bridge.

The `dbx-web` backend already exists in DBX and is used by its browser/Docker deployment, so this is not a new server we have to maintain.
