import { resolve } from "node:path"
import { defineConfig, externalizeDepsPlugin } from "electron-vite"
import solid from "vite-plugin-solid"

export default defineConfig({
  main: {
    plugins: [externalizeDepsPlugin()],
  },
  preload: {
    plugins: [externalizeDepsPlugin()],
  },
  renderer: {
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
