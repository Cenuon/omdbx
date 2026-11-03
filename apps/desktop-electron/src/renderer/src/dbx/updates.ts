declare const __APP_VERSION__: string

export const APP_VERSION = typeof __APP_VERSION__ === "string" ? __APP_VERSION__ : "0.0.0"

export type UpdateLevel = "patch" | "minor" | "major"

export interface UpdateInfo {
  version: string
  url: string
  level: UpdateLevel
}

const RELEASES_API = "https://api.github.com/repos/Cenuon/omdbx/releases/latest"

function parseVersion(text: string): [number, number, number] {
  const match = text.match(/(\d+)\.(\d+)\.(\d+)/)
  return match ? [Number(match[1]), Number(match[2]), Number(match[3])] : [0, 0, 0]
}

/** How far behind the released version is: patch < minor < major. */
export function compareVersions(current: string, latest: string): UpdateLevel | null {
  const [cMajor, cMinor, cPatch] = parseVersion(current)
  const [lMajor, lMinor, lPatch] = parseVersion(latest)
  if (lMajor > cMajor) return "major"
  if (lMajor === cMajor && lMinor > cMinor) return "minor"
  if (lMajor === cMajor && lMinor === cMinor && lPatch > cPatch) return "patch"
  return null
}

/** Asks GitHub for the newest published release; null when offline or up to date. */
export async function checkForUpdate(): Promise<UpdateInfo | null> {
  // an uninjected version would make every release look newer
  if (APP_VERSION === "0.0.0") return null
  try {
    const response = await fetch(RELEASES_API, { headers: { Accept: "application/vnd.github+json" } })
    if (!response.ok) return null
    const data = (await response.json()) as { tag_name?: string; html_url?: string }
    const version = String(data.tag_name ?? "").replace(/^desktop-v/, "")
    const level = compareVersions(APP_VERSION, version)
    if (!level || !data.html_url) return null
    return { version, url: data.html_url, level }
  } catch {
    return null
  }
}
