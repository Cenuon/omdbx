# Releasing installers

The fork publishes the same kind of download assets as upstream: macOS DMG/ZIP, Windows NSIS/portable EXE, Linux AppImage/DEB.

## Automatic GitHub Actions

1. Enable Actions in `Cenuon/omdbx` (forks start with Actions disabled).
2. Create and push a desktop tag:

```bash
git tag desktop-v0.1.0
git push origin desktop-v0.1.0
```

`desktop-electron-release` builds the DBX `dbx-web` backend on each OS, packages it into the Electron app, and publishes the installers as a GitHub Release (visible immediately on the Releases page and repo home).

Jobs:

- macOS arm64 (`macos-14`)
- macOS x64 (`macos-14`, cross-built)
- Windows x64 (`windows-latest`)
- Linux x64 (`ubuntu-22.04`)

The workflow can also be run manually (`workflow_dispatch`) to produce Actions artifacts without creating a release.

## Signing

The initial installers are **not signed/notarized**:

- macOS: users may need Right-click → Open the first time, or remove quarantine.
- Windows: SmartScreen may warn.

For public distribution, add Apple Developer ID + notarization and a Windows code-signing certificate. `electron-builder` already reads the standard `CSC_*` / `APPLE_*` environment variables.

## Local packaging

```bash
cd apps/desktop-electron
./scripts/build-backend.sh
pnpm install --frozen-lockfile
pnpm dist:mac      # or dist:win / dist:linux
```

Output: `apps/desktop-electron/release/`.

The backend sidecar is intentionally not committed (100 MB+ per platform); CI builds it from the fork's `crates/dbx-web`.
