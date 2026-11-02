import { app } from "electron"
import { spawn, type ChildProcess } from "node:child_process"
import { existsSync, mkdirSync } from "node:fs"
import { createServer } from "node:net"
import { join } from "node:path"

export interface DbxWebProcess {
  url: string
  stop: () => void
}

function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = createServer()
    server.once("error", reject)
    server.listen(0, "127.0.0.1", () => {
      const address = server.address()
      const port = typeof address === "object" && address ? address.port : 0
      server.close(() => resolve(port))
    })
  })
}

async function waitForServer(url: string, timeoutMs = 20_000): Promise<void> {
  const startedAt = Date.now()
  while (Date.now() - startedAt < timeoutMs) {
    try {
      const response = await fetch(url, { method: "GET" })
      if (response.ok || response.status === 401 || response.status === 302) return
    } catch {
      // The sidecar is still binding its port.
    }
    await new Promise((resolve) => setTimeout(resolve, 200))
  }
  throw new Error(`dbx-web did not become ready at ${url}`)
}

function resolveBinary(): string {
  if (process.env.DBX_WEB_BIN) return process.env.DBX_WEB_BIN
  return join(process.resourcesPath, "dbx-web")
}

export async function startDbxWeb(): Promise<DbxWebProcess> {
  const binary = resolveBinary()
  if (!existsSync(binary)) {
    throw new Error(
      `dbx-web binary not found at ${binary}. Put the release binary there or set DBX_WEB_BIN for development.`,
    )
  }

  const port = await freePort()
  const dataDir = join(app.getPath("userData"), "dbx-web")
  mkdirSync(dataDir, { recursive: true })

  const child: ChildProcess = spawn(binary, [], {
    env: {
      ...process.env,
      DBX_PORT: String(port),
      DBX_BIND_ADDR: "127.0.0.1",
      DBX_DISABLE_PASSWORD: "1",
      DBX_DATA_DIR: dataDir,
      DBX_STATIC_DIR: join(__dirname, "../renderer"),
    },
    stdio: ["ignore", "pipe", "pipe"],
  })

  child.stdout?.on("data", (chunk) => console.log("[dbx-web]", String(chunk).trim()))
  child.stderr?.on("data", (chunk) => console.error("[dbx-web]", String(chunk).trim()))

  const url = `http://127.0.0.1:${port}/`
  await waitForServer(url)
  return {
    url,
    stop: () => {
      if (!child.killed) child.kill()
    },
  }
}
