import { contextBridge, ipcRenderer } from "electron"

contextBridge.exposeInMainWorld("api", {
  minimize: () => ipcRenderer.send("window:minimize"),
  maximize: () => ipcRenderer.send("window:maximize"),
  close: () => ipcRenderer.send("window:close"),
  onMaximized: (callback: (maximized: boolean) => void) => {
    ipcRenderer.on("window:maximized", (_event, maximized) => callback(maximized))
  },
})
