/// <reference types="vite/client" />

interface Window {
  api?: {
    minimize: () => void
    maximize: () => void
    close: () => void
    onMaximized: (callback: (maximized: boolean) => void) => void
    openExternal: (url: string) => void
  }
}
