import { contextBridge, ipcRenderer } from "electron"

contextBridge.exposeInMainWorld("api", {
  minimize: () => ipcRenderer.send("window:minimize"),
  maximize: () => ipcRenderer.send("window:maximize"),
  close: () => ipcRenderer.send("window:close"),
  openExternal: (url: string) => ipcRenderer.send("shell:open-external", url),
  onMaximized: (callback: (maximized: boolean) => void) => {
    ipcRenderer.on("window:maximized", (_event, maximized) => callback(maximized))
  },
})
