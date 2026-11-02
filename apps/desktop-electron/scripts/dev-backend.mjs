#!/usr/bin/env node
import { spawn } from "node:child_process"
import { existsSync, mkdirSync } from "node:fs"
import { dirname, join, resolve } from "node:path"
import { fileURLToPath } from "node:url"

const appDir = resolve(dirname(fileURLToPath(import.meta.url)), "..")
const repoRoot = resolve(appDir, "../..")
const dataDir = join(appDir, ".dbx-web-data")
mkdirSync(dataDir, { recursive: true })

const env = {
  ...process.env,
  DBX_PORT: process.env.DBX_PORT || "4224",
  DBX_BIND_ADDR: process.env.DBX_BIND_ADDR || "127.0.0.1",
  DBX_DISABLE_PASSWORD: process.env.DBX_DISABLE_PASSWORD || "1",
  DBX_DATA_DIR: process.env.DBX_DATA_DIR || dataDir,
}

const explicitBinary = process.env.DBX_WEB_BIN ? resolve(process.env.DBX_WEB_BIN) : null
const bundledBinary = join(appDir, "backend", "dbx-web")
const binary = explicitBinary || (existsSync(bundledBinary) ? bundledBinary : null)
const cargoToml = join(repoRoot, "Cargo.toml")

let command
let args
if (binary && existsSync(binary)) {
  command = binary
  args = []
} else if (existsSync(cargoToml)) {
  command = "cargo"
  args = ["run", "--release", "-p", "dbx-web"]
} else {
  console.error(`
dbx-web binary not found.

Do one of these:
  1. Build it from the fork:
       cargo build --release -p dbx-web
       cp target/release/dbx-web ${bundledBinary}

  2. Download a DBX browser-static release for your OS/arch and put dbx-web at:
       ${bundledBinary}

  3. Point to an existing binary:
       DBX_WEB_BIN=/absolute/path/to/dbx-web pnpm dev:backend
`)
  process.exit(1)
}

console.log(`[dbx-web] ${command} ${args.join(" ")}`)
console.log(`[dbx-web] data: ${env.DBX_DATA_DIR}`)
console.log(`[dbx-web] http://${env.DBX_BIND_ADDR}:${env.DBX_PORT}`)

const child = spawn(command, args, { cwd: repoRoot, env, stdio: "inherit" })
child.on("exit", (code) => process.exit(code ?? 0))
for (const signal of ["SIGINT", "SIGTERM"]) {
  process.on(signal, () => child.kill(signal))
}
