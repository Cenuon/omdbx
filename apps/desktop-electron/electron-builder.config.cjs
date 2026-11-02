const isWindows = process.platform === "win32"

module.exports = {
  appId: "io.dbx.desktop",
  productName: "DBX",
  directories: {
    output: "release",
    buildResources: "build",
  },
  files: ["out/main/**/*", "out/preload/**/*", "package.json"],
  extraResources: [
    { from: "out/renderer", to: "renderer" },
    {
      from: isWindows ? "backend/dbx-web.exe" : "backend/dbx-web",
      to: isWindows ? "dbx-web.exe" : "dbx-web",
    },
  ],
  mac: {
    target: ["dmg", "zip"],
    category: "public.app-category.developer-tools",
    icon: "build/icon.icns",
    hardenedRuntime: false,
    gatekeeperAssess: false,
    identity: null,
  },
  win: {
    target: ["nsis", "portable"],
    icon: "build/icon.ico",
  },
  linux: {
    target: ["AppImage", "deb"],
    category: "Development",
    icon: "build/icon.png",
    // scoped package name (@dbx-app/...) is illegal as an executable name
    executableName: "dbx",
  },
  nsis: {
    oneClick: false,
    perMachine: false,
    allowToChangeInstallationDirectory: true,
    deleteAppDataOnUninstall: false,
  },
  publish: [
    {
      provider: "github",
      owner: "Cenuon",
      repo: "dbx",
      releaseType: "draft",
    },
  ],
}
