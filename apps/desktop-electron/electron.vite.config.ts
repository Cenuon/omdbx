import { defineConfig, externalizeDepsPlugin } from "electron-vite"
import { readFileSync } from "node:fs"
import { resolve } from "node:path"
import solid from "vite-plugin-solid"

function appVersion(): string {
  try {
    const pkg = JSON.parse(readFileSync(resolve(process.cwd(), "package.json"), "utf-8")) as { version?: string }
    return pkg.version ?? "0.0.0"
  } catch {
    return "0.0.0"
  }
}

export default defineConfig({
  main: {
    plugins: [externalizeDepsPlugin()],
  },
  preload: {
    plugins: [externalizeDepsPlugin()],
  },
  renderer: {
    define: {
      __APP_VERSION__: JSON.stringify(appVersion()),
    },
    resolve: {
      alias: {
        "@": resolve("src/renderer/src"),
      },
    },
    plugins: [solid()],
    server: {
      proxy: {
        "/api": {
          target: "http://127.0.0.1:4224",
          changeOrigin: true,
          ws: true,
        },
      },
    },
  },
})
