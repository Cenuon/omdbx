# Upstream / fork maintenance

This app is additive. The DBX backend stays in the fork's `crates/` and `src-tauri/`; we do not replace or rewrite it.

## Remotes

```bash
git remote -v
# origin   https://github.com/Cenuon/omdbx.git
# upstream https://github.com/t8y2/dbx.git
```

If `upstream` is missing:

```bash
git remote add upstream https://github.com/t8y2/dbx.git
```

## Pull DBX updates

```bash
git fetch upstream
git merge upstream/main
```

Conflict policy:

- **Upstream files** (`apps/desktop`, `crates/*`, `src-tauri/*`, root package files): prefer upstream.
- **`apps/desktop-electron/*`**: this is ours; upstream has no file here, so merges should stay clean.
- Do not patch DBX backend for frontend-only needs. CORS/auth/static serving are already handled by `dbx-web` (`DBX_STATIC_DIR`, `DBX_BIND_ADDR`, `DBX_DISABLE_PASSWORD`).

## Backend binary update

After an upstream merge, rebuild or replace the sidecar:

```bash
cargo build --release -p dbx-web
cp target/release/dbx-web apps/desktop-electron/backend/dbx-web
```

Or download the new `browser-static` release for the target OS/arch and replace the binary. The frontend talks to the HTTP API only, so backend upgrades do not require a frontend rewrite. If an endpoint changes, update `src/renderer/src/dbx/client.ts`.

## Frontend release

```bash
cd apps/desktop-electron
pnpm install
pnpm build
```

Electron Builder packaging is the next step; the main process already knows how to start the sidecar and load the frontend from it.
