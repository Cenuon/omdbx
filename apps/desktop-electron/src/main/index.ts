import { app, BrowserWindow, dialog, ipcMain, shell } from "electron"
import { writeFileSync } from "node:fs"
import { join } from "node:path"
import { startDbxWeb, type DbxWebProcess } from "./dbx-web"

let mainWindow: BrowserWindow | null = null
let sidecar: DbxWebProcess | null = null

async function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1440,
    height: 900,
    minWidth: 1080,
    minHeight: 680,
    show: false,
    frame: false,
    titleBarStyle: "hidden",
    backgroundColor: "#f4f4f1",
    trafficLightPosition: { x: -100, y: -100 },
    webPreferences: {
      preload: join(__dirname, "../preload/index.mjs"),
      sandbox: false,
    },
  })

  mainWindow.once("ready-to-show", () => mainWindow?.show())
  mainWindow.on("maximize", () => mainWindow?.webContents.send("window:maximized", true))
  mainWindow.on("unmaximize", () => mainWindow?.webContents.send("window:maximized", false))
  mainWindow.webContents.setWindowOpenHandler(({ url }) => {
    shell.openExternal(url)
    return { action: "deny" }
  })

  mainWindow.webContents.on("did-finish-load", async () => {
    console.log("preload api:", await mainWindow?.webContents.executeJavaScript("typeof window.api"))
    const target = process.env.DBX_CAPTURE
    if (!target) return
    setTimeout(async () => {
      const image = await mainWindow?.webContents.capturePage()
      if (image) writeFileSync(target, image.toPNG())
    }, 1200)
  })

  if (process.env.ELECTRON_RENDERER_URL) {
    await mainWindow.loadURL(process.env.ELECTRON_RENDERER_URL)
  } else {
    try {
      sidecar = await startDbxWeb()
      await mainWindow.loadURL(sidecar.url)
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      dialog.showErrorBox(
        "DBX backend is not ready",
        `${message}\n\nPut the dbx-web binary in resources/dbx-web (or set DBX_WEB_BIN), then restart.`,
      )
      await mainWindow.loadFile(join(__dirname, "../renderer/index.html"))
    }
  }
}

ipcMain.on("window:minimize", () => mainWindow?.minimize())
ipcMain.on("window:maximize", () => {
  if (!mainWindow) return
  if (mainWindow.isMaximized()) mainWindow.unmaximize()
  else mainWindow.maximize()
})
ipcMain.on("window:close", () => mainWindow?.close())

ipcMain.on("shell:open-external", (_event, url: unknown) => {
  if (typeof url === "string" && /^https?:\/\//.test(url)) void shell.openExternal(url)
})

app.whenReady().then(() => {
  createWindow()
  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

app.on("before-quit", () => {
  sidecar?.stop()
  sidecar = null
})

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") app.quit()
})
