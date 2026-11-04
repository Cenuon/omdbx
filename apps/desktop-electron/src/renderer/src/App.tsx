import {
  ArrowLeftRight,
  ArrowUpDown,
  BookOpen,
  Box,
  Boxes,
  Braces,
  CalendarClock,
  Check,
  ChevronDown,
  ChevronRight,
  CircleSlash,
  ClipboardPaste,
  Clock,
  Columns3,
  Copy,
  Database,
  Download,
  Ellipsis,
  Eraser,
  EyeOff,
  FileCode2,
  FileText,
  Filter,
  Folder,
  FolderOpen,
  FolderPlus,
  GitCompareArrows,
  Grip,
  History,
  Info,
  LayoutGrid,
  KeyRound,
  List,
  LocateFixed,
  Lock,
  MessageSquare,
  Moon,
  MoveHorizontal,
  MoveVertical,
  PanelLeft,
  PanelRight,
  Paperclip,
  Pin,
  Play,
  PlugZap,
  Plus,
  Power,
  RefreshCw,
  Search,
  Send,
  Settings,
  Settings2,
  Share2,
  Sparkles,
  SquarePen,
  Star,
  Sun,
  Table2,
  Terminal,
  Trash2,
  Type,
  WandSparkles,
  X,
  ZoomIn,
} from "lucide-solid"
import { For, Show, createEffect, createMemo, createSignal, onCleanup, onMount, type JSX } from "solid-js"
import SettingsView from "./SettingsView"
import { historyItems, sqlFiles, type GridFont, type RowStyle, type TabItem } from "./data"
import {
  browseTable,
  connectDb,
  disconnectDb,
  executeQuery,
  getConnections,
  getDatabases,
  getSchemas,
  getTables,
  loadSidebarLayout,
  prepareDataGridSave,
  saveConnections,
  saveSidebarLayout,
  type DbxConnection,
  type DbxConnectionInput,
} from "./dbx/client"
import NewConnectionDialog from "./NewConnectionDialog"
import ContextMenu, { type MenuItem } from "./ContextMenu"
import { databaseTypes } from "./dbx/databaseTypes"
import { checkForUpdate, type UpdateInfo } from "./dbx/updates"

/** Colour of the update badge by how far behind the build is. */
const UPDATE_COLORS: Record<UpdateInfo["level"], string> = {
  patch: "#3f7fd0",
  minor: "#e0a83c",
  major: "#e5534b",
}

/** db_type → bundled icon, first entry wins so profile variants reuse the base icon. */
const CONNECTION_ICONS = new Map<string, string>()
for (const type of databaseTypes) {
  if (!CONNECTION_ICONS.has(type.dbType)) CONNECTION_ICONS.set(type.dbType, type.icon)
}

type Theme = "light" | "dark"
type Panel = "ai" | "sql" | "history"

function Logo(props: { size?: number }) {
  const size = props.size ?? 18
  return (
    <svg class="logo" width={size} height={size} viewBox="0 0 32 32" aria-hidden="true">
      <rect x="1.5" y="1.5" width="29" height="29" rx="8.5" fill="currentColor" />
      <path d="M9.5 10.5h7.2M9.5 15.5h11.2M9.5 20.5h5.2" stroke="var(--logo-ink)" stroke-width="2.2" stroke-linecap="round" />
      <circle cx="22.8" cy="20.5" r="1.4" fill="var(--logo-ink)" />
    </svg>
  )
}

function WindowControls(props: {
  maximized: boolean
  onMinimize: () => void
  onMaximize: () => void
  onClose: () => void
}) {
  return (
    <div class="window-controls" aria-hidden="true">
      <button class="tl red" onClick={props.onClose} aria-label="关闭" />
      <button class="tl yellow" onClick={props.onMinimize} aria-label="最小化" />
      <button class="tl green" onClick={props.onMaximize} aria-label={props.maximized ? "还原" : "最大化"} />
    </div>
  )
}

function Titlebar(props: {
  maximized: boolean
  tabs: TabItem[]
  active: string
  sidebarCollapsed: boolean
  rightCollapsed: boolean
  settingsOpen: boolean
  theme: Theme
  onToggleSidebar: () => void
  onToggleRight: () => void
  onSelect: (id: string) => void
  tabColor: (tab: TabItem) => string | undefined
  update: UpdateInfo | null
  onUpdate: () => void
  onCloseTab: (id: string) => void
  onNewQuery: () => void
  onTheme: () => void
  onSettings: () => void
  onMinimize: () => void
  onMaximize: () => void
  onClose: () => void
}) {
  return (
    <header class="titlebar" data-tauri-drag-region>
      <Show when={props.sidebarCollapsed || props.settingsOpen}>
        <div class="titlebar-left">
          <WindowControls
            maximized={props.maximized}
            onMinimize={props.onMinimize}
            onMaximize={props.onMaximize}
            onClose={props.onClose}
          />
          <Show when={!props.settingsOpen}>
            <button
              class="titlebar-toggle"
              aria-label="展开侧栏"
              title="展开侧栏"
              onClick={props.onToggleSidebar}
            >
              <PanelLeft size={16} />
            </button>
          </Show>
        </div>
      </Show>

      <div class="titlebar-tabs">
        <div
          class="titlebar-tabs-scroll"
          role="tablist"
          onWheel={(event) => {
            const strip = event.currentTarget
            if (strip.scrollWidth <= strip.clientWidth) return
            strip.scrollLeft += event.deltaY + event.deltaX
            event.preventDefault()
          }}
        >
          <For each={props.tabs}>
          {(tab) => (
            <div
              class="tab"
              classList={{ active: props.active === tab.id, "conn-tinted": !!props.tabColor(tab) }}
              style={props.tabColor(tab) ? { "--conn-color": props.tabColor(tab) } : undefined}
              role="tab"
              aria-selected={props.active === tab.id}
              tabindex="0"
              onClick={() => props.onSelect(tab.id)}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault()
                  props.onSelect(tab.id)
                }
              }}
            >
              {tab.kind === "query" ? <Terminal size={13} class="tab-icon" /> : <Table2 size={13} class="tab-icon" />}
              <span class="tab-name">{tab.title}</span>
              <Show when={tab.sub}>
                <span class="tab-sub">{tab.sub}</span>
              </Show>
              <Show when={tab.unsaved}>
                <span class="tab-unsaved" />
              </Show>
              <button
                class="tab-close"
                aria-label={`关闭 ${tab.title}`}
                onClick={(event) => {
                  event.stopPropagation()
                  props.onCloseTab(tab.id)
                }}
              >
                ×
              </button>
            </div>
          )}
          </For>
        </div>
        <button class="tab-add" aria-label="新建查询" title="新建查询" onClick={props.onNewQuery}>
          <Plus size={14} />
        </button>
      </div>

      <div class="titlebar-right">
        <Show when={props.update}>
          {(info) => (
            <button
              class="titlebar-toggle update-pill"
              style={{ "--update-color": UPDATE_COLORS[info().level] }}
              aria-label={`有新版本 ${info().version}`}
              title={`有新版本 ${info().version}，点击查看下载`}
              onClick={props.onUpdate}
            >
              <Download size={15} />
            </button>
          )}
        </Show>
        <button class="titlebar-toggle" aria-label="切换主题" title="切换主题" onClick={props.onTheme}>
          <Show when={props.theme === "light"} fallback={<Sun size={16} />}>
            <Moon size={16} />
          </Show>
        </button>
        <button
          class="titlebar-toggle"
          classList={{ active: props.settingsOpen }}
          aria-label="设置"
          title="设置"
          aria-pressed={props.settingsOpen}
          onClick={props.onSettings}
        >
          <Settings size={16} />
        </button>
        <Show when={!props.settingsOpen}>
          <button
            class="titlebar-toggle"
            aria-label={props.rightCollapsed ? "展开右侧面板" : "收起右侧面板"}
            title={props.rightCollapsed ? "展开右侧面板" : "收起右侧面板"}
            onClick={props.onToggleRight}
          >
            <PanelRight size={16} />
          </button>
        </Show>
      </div>
    </header>
  )
}

function TreeGroup(props: {
  title: string
  count?: string
  level: number
  defaultOpen?: boolean
  icon?: JSX.Element
  children: JSX.Element
}) {
  const [open, setOpen] = createSignal(props.defaultOpen ?? false)
  return (
    <div class="tree-group" data-open={open()}>
      <button class="tree-row" style={{ "padding-left": `${8 + props.level * 12}px` }} onClick={() => setOpen(!open())} aria-expanded={open()}>
        <ChevronRight size={13} class="chev" classList={{ open: open() }} />
        {props.icon}
        <span class="tree-name">{props.title}</span>
        <Show when={props.count}>
          <span class="tree-count">{props.count}</span>
        </Show>
      </button>
      <Show when={open()}>
        <div class="tree-children">{props.children}</div>
      </Show>
    </div>
  )
}

function TreeLeaf(props: {
  title: string
  level: number
  icon?: JSX.Element
  count?: string
  active?: boolean
  muted?: boolean
  onClick?: () => void
}) {
  return (
    <button
      class="tree-row leaf"
      classList={{ active: props.active, muted: props.muted }}
      style={{ "padding-left": `${8 + props.level * 12}px` }}
      onClick={props.onClick}
    >
      <span class="chev-space" />
      {props.icon}
      <span class="tree-name">{props.title}</span>
      <Show when={props.count}>
        <span class="tree-count">{props.count}</span>
      </Show>
    </button>
  )
}

interface SidebarProps {
  width: number
  contentWidth: number
  maximized: boolean
  backendStatus: "checking" | "online" | "offline"
  reloadToken: number
  pendingId: string | null
  activeTab: string
  onToggle: () => void
  onNewConnection: () => void
  onNewQuery: () => void
  onCommonActions: () => void
  onConnectionsChanged: () => void
  onToast: (message: string) => void
  onMinimize: () => void
  onMaximize: () => void
  onClose: () => void
  onOpenTable: (connection: DbxConnection, database: string, schema: string, table: string) => void
}

interface SidebarFolder {
  id: string
  name: string
  connections: string[]
}

const CONNECTION_COLORS: Array<{ value: string | null; color: string }> = [
  { value: "#e5534b", color: "#e5534b" },
  { value: "#e08c3c", color: "#e08c3c" },
  { value: "#d9b23c", color: "#d9b23c" },
  { value: "#4f9d69", color: "#4f9d69" },
  { value: "#3f7fd0", color: "#3f7fd0" },
  { value: "#8a6bd1", color: "#8a6bd1" },
  { value: "#8a8a84", color: "#8a8a84" },
  { value: null, color: "" },
]

/** Catalog schemas that only get in the way of browsing real data. */const SYSTEM_DATABASES = new Set([
  "information_schema",
  "performance_schema",
  "pg_catalog",
  "pg_toast",
  "mysql",
  "sys",
])

function isSystemDatabase(name: string): boolean {
  const lower = name.toLowerCase()
  return SYSTEM_DATABASES.has(lower) || lower.startsWith("pg_temp")
}

function Sidebar(props: SidebarProps) {
  const [connections, setConnections] = createSignal<DbxConnection[]>([])
  const [openConnections, setOpenConnections] = createSignal<Record<string, boolean>>({})
  const [connectionData, setConnectionData] = createSignal<
    Record<
      string,
      { databases: string[]; tables: Record<string, string[]>; loaded: Record<string, boolean>; loading: boolean }
    >
  >({})
  const [openDatabases, setOpenDatabases] = createSignal<Record<string, boolean>>({})
  const [folders, setFolders] = createSignal<SidebarFolder[]>([])
  const [stars, setStars] = createSignal<string[]>([])
  const [openFolders, setOpenFolders] = createSignal<Record<string, boolean>>({})
  const [connectionMenu, setConnectionMenu] = createSignal<{ x: number; y: number; conn: DbxConnection } | null>(null)
  const [folderDialog, setFolderDialog] = createSignal<{ kind: "new" | "rename"; id?: string } | null>(null)
  const [folderName, setFolderName] = createSignal("")
  const [sidebarMenu, setSidebarMenu] = createSignal<{ x: number; y: number } | null>(null)
  const [manageOpen, setManageOpen] = createSignal(false)

  let layoutReady = false

  async function loadLayout() {
    try {
      const raw = (await loadSidebarLayout()) as { omdbx?: { folders?: SidebarFolder[]; stars?: string[] } } | null
      if (raw?.omdbx?.folders) setFolders(raw.omdbx.folders)
      if (raw?.omdbx?.stars) setStars(raw.omdbx.stars)
    } catch {
      // the layout row is optional
    }
  }

  createEffect(() => {
    const payload = { omdbx: { folders: folders(), stars: stars() } }
    if (!layoutReady) return
    void saveSidebarLayout(payload).catch(() => undefined)
  })

  async function refreshConnections(): Promise<DbxConnection[]> {
    try {
      const list = await getConnections()
      setConnections(list)
      return list
    } catch {
      props.onToast("读取连接列表失败")
      return []
    }
  }

  let initialized = false
  createEffect(() => {
    if (props.backendStatus !== "online" || initialized) return
    initialized = true
    void loadLayout().then(() => {
      layoutReady = true
    })
    void refreshConnections().then((list) => {
      if (list.length === 0) return
      const next: Record<string, boolean> = {}
      list.forEach((conn, index) => {
        next[conn.id] = index === 0
      })
      setOpenConnections(next)
      void loadConnection(list[0])
    })
  })

  // a connection was just created: reload the list, the folder layout, and open it
  let lastReload = props.reloadToken
  createEffect(() => {
    const token = props.reloadToken
    if (token === lastReload) return
    lastReload = token
    void loadLayout()
    void refreshConnections().then((list) => {
      const conn = list.find((item) => item.id === props.pendingId)
      if (!conn) return
      setOpenConnections((state) => ({ ...state, [conn.id]: true }))
      void loadConnection(conn)
    })
  })

  async function loadConnection(conn: DbxConnection) {
    const id = conn.id
    setConnectionData((data) => ({ ...data, [id]: { databases: [], tables: {}, loaded: {}, loading: true } }))
    try {
      await connectDb(conn)
      const list = await getDatabases(id)
      const databases = list.map((item) => item.name)
      setConnectionData((data) => ({ ...data, [id]: { databases, tables: {}, loaded: {}, loading: false } }))
      // a connection with a single user database can open it straight away
      const user = databases.filter((name) => !isSystemDatabase(name))
      const only = conn.database || (user.length === 1 ? user[0] : "")
      if (only) await loadTables(conn, only)
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      props.onToast(`${conn.name}: ${message}`)
      setConnectionData((data) => ({ ...data, [id]: { databases: [], tables: {}, loaded: {}, loading: false } }))
    }
  }

  async function loadTables(conn: DbxConnection, database: string) {
    if (connectionData()[conn.id]?.loaded[database]) return
    setConnectionData((data) => {
      const current = data[conn.id]
      if (!current) return data
      return { ...data, [conn.id]: { ...current, loaded: { ...current.loaded, [database]: true } } }
    })
    const schemas = await getSchemas(conn.id, database).catch(() => [] as string[])
    const schema = conn.default_schema || schemas[0] || database
    const tables = await getTables(conn.id, database, schema).catch(() => [])
    setConnectionData((data) => {
      const current = data[conn.id]
      if (!current) return data
      return {
        ...data,
        [conn.id]: {
          ...current,
          tables: { ...current.tables, [`${database}|${schema}`]: tables.map((table) => table.name) },
        },
      }
    })
  }

  function toggleConnection(conn: DbxConnection) {
    const next = !openConnections()[conn.id]
    setOpenConnections((state) => ({ ...state, [conn.id]: next }))
    if (next && !connectionData()[conn.id]) void loadConnection(conn)
  }

  function toggleDatabase(conn: DbxConnection, database: string) {
    const key = `${conn.id}|${database}`
    const next = !openDatabases()[key]
    setOpenDatabases((state) => ({ ...state, [key]: next }))
    if (next) void loadTables(conn, database)
  }

  /** Databases worth showing: system catalogs only when nothing else exists. */
  function databasesOf(conn: DbxConnection): string[] {
    const all = connectionData()[conn.id]?.databases ?? []
    const user = all.filter((name) => !isSystemDatabase(name))
    return user.length > 0 ? user : all
  }

  function tablesOf(conn: DbxConnection, database: string): Array<{ schema: string; table: string }> {
    const rows: Array<{ schema: string; table: string }> = []
    for (const [key, names] of Object.entries(connectionData()[conn.id]?.tables ?? {})) {
      const separator = key.indexOf("|")
      if (key.slice(0, separator) !== database) continue
      const schema = key.slice(separator + 1)
      for (const table of names) rows.push({ schema, table })
    }
    return rows
  }

  function tablesLoaded(conn: DbxConnection, database: string): boolean {
    return Object.keys(connectionData()[conn.id]?.tables ?? {}).some((key) => key.startsWith(`${database}|`))
  }

  function openTable(conn: DbxConnection, database: string, schema: string, table: string) {
    props.onOpenTable(conn, database, schema, table)
  }

  /** Expand the tree down to the table shown in the active tab and scroll to it. */
  async function locateActiveTable() {
    const [connId, database, , table] = props.activeTab.split("|")
    const conn = connections().find((item) => item.id === connId)
    if (!conn || !table) {
      props.onToast("当前没有打开的表")
      return
    }
    const folder = folderOf(conn.id)
    if (folder) setOpenFolders((state) => ({ ...state, [folder.id]: true }))
    setOpenConnections((state) => ({ ...state, [conn.id]: true }))
    if (!connectionData()[conn.id]) await loadConnection(conn)
    if (databasesOf(conn).length > 1) {
      setOpenDatabases((state) => ({ ...state, [`${conn.id}|${database}`]: true }))
    }
    await loadTables(conn, database)
    requestAnimationFrame(() => {
      document.querySelector(`.tree-table[data-key="${props.activeTab}"]`)?.scrollIntoView({ block: "center" })
    })
  }

  function renderTables(conn: DbxConnection, database: string, indent: number) {
    return (
      <Show
        when={tablesLoaded(conn, database)}
        fallback={
          <div class="tree-hint" style={{ "padding-left": `${indent}px` }}>
            加载中…
          </div>
        }
      >
        <Show
          when={tablesOf(conn, database).length > 0}
          fallback={
            <div class="tree-hint" style={{ "padding-left": `${indent}px` }}>
              没有表
            </div>
          }
        >
          <For each={tablesOf(conn, database)}>
            {(row) => (
              <button
                class="tree-row leaf tree-table"
                classList={{ active: props.activeTab === `${conn.id}|${database}|${row.schema}|${row.table}` }}
                data-key={`${conn.id}|${database}|${row.schema}|${row.table}`}
                style={{ "padding-left": `${indent}px` }}
                onClick={() => openTable(conn, database, row.schema, row.table)}
              >
                <Table2 size={16} />
                <span class="tree-name">{row.table}</span>
              </button>
            )}
          </For>
        </Show>
      </Show>
    )
  }

  function connectionIcon(dbType: string): string | undefined {
    return CONNECTION_ICONS.get(dbType)
  }

  /** A connection is live once it is expanded and its databases came back. */
  function connectionOnline(conn: DbxConnection): boolean {
    const data = connectionData()[conn.id]
    return !!openConnections()[conn.id] && !!data && !data.loading && data.databases.length > 0
  }

  function folderOf(connectionId: string): SidebarFolder | undefined {
    return folders().find((folder) => folder.connections.includes(connectionId))
  }

  function folderOpen(id: string): boolean {
    return !!openFolders()[id]
  }

  function folderConnections(folder: SidebarFolder): DbxConnection[] {
    return folder.connections
      .map((id) => connections().find((conn) => conn.id === id))
      .filter((conn): conn is DbxConnection => !!conn)
  }

  function ungroupedConnections(): DbxConnection[] {
    return connections().filter((conn) => !folderOf(conn.id))
  }

  function toggleFolder(id: string) {
    setOpenFolders((state) => ({ ...state, [id]: !state[id] }))
  }

  function saveFolder() {
    const name = folderName().trim()
    if (!name) return
    const dialog = folderDialog()
    if (dialog?.kind === "rename" && dialog.id) {
      setFolders((list) => list.map((folder) => (folder.id === dialog.id ? { ...folder, name } : folder)))
    } else {
      setFolders((list) => [...list, { id: crypto.randomUUID(), name, connections: [] }])
    }
    setFolderDialog(null)
    setFolderName("")
  }

  function moveToFolder(connection: DbxConnection, folderId: string | null) {
    setFolders((list) =>
      list.map((folder) => {
        const without = folder.connections.filter((id) => id !== connection.id)
        return folder.id === folderId
          ? { ...folder, connections: [...without, connection.id] }
          : { ...folder, connections: without }
      }),
    )
    if (folderId) setOpenFolders((state) => ({ ...state, [folderId]: true }))
  }

  function toggleStar(connection: DbxConnection) {
    setStars((list) =>
      list.includes(connection.id) ? list.filter((id) => id !== connection.id) : [...list, connection.id],
    )
  }

  async function closeConnection(connection: DbxConnection) {
    try {
      await disconnectDb(connection.id)
      props.onToast(`已关闭连接 ${connection.name}`)
    } catch (error) {
      props.onToast(`关闭失败：${error instanceof Error ? error.message : String(error)}`)
    }
  }

  async function removeConnection(connection: DbxConnection) {
    try {
      await saveConnections([], [connection.id])
      setFolders((list) =>
        list.map((folder) => ({ ...folder, connections: folder.connections.filter((id) => id !== connection.id) })),
      )
      setStars((list) => list.filter((id) => id !== connection.id))
      await refreshConnections()
      props.onToast(`已删除连接 ${connection.name}`)
    } catch (error) {
      props.onToast(`删除失败：${error instanceof Error ? error.message : String(error)}`)
    }
  }

  async function duplicateConnection(connection: DbxConnection) {
    const copy = { ...connection, id: crypto.randomUUID(), name: `${connection.name} 副本` }
    try {
      await saveConnections([copy], [])
      await refreshConnections()
      props.onToast(`已复制为 ${copy.name}`)
    } catch (error) {
      props.onToast(`复制失败：${error instanceof Error ? error.message : String(error)}`)
    }
  }

  async function setConnectionColor(connection: DbxConnection, color: string | null) {
    try {
      await saveConnections([{ ...connection, color }], [])
      await refreshConnections()
      props.onConnectionsChanged()
    } catch (error) {
      props.onToast(`设置颜色失败：${error instanceof Error ? error.message : String(error)}`)
    }
  }

  async function closeAllConnections() {
    const list = connections()
    if (list.length === 0) return
    await Promise.all(list.map((connection) => disconnectDb(connection.id).catch(() => undefined)))
    props.onToast(`已关闭 ${list.length} 个连接`)
  }

  function sidebarMenuItems(): MenuItem[] {
    return [
      { label: "新建连接", icon: <Plus size={15} />, onSelect: () => props.onNewConnection() },
      {
        label: "关闭所有连接",
        icon: <Power size={15} />,
        disabled: connections().length === 0,
        onSelect: () => void closeAllConnections(),
      },
      { label: "管理连接…", icon: <Settings2 size={15} />, onSelect: () => setManageOpen(true) },
      { separator: true },
      {
        label: "新建组",
        icon: <FolderPlus size={15} />,
        onSelect: () => {
          setFolderName("")
          setFolderDialog({ kind: "new" })
        },
      },
      { separator: true },
      { label: "刷新", icon: <RefreshCw size={15} />, hint: "⌘R", onSelect: () => void refreshConnections() },
    ]
  }

  function connectionMenuItems(connection: DbxConnection): MenuItem[] {
    const folder = folderOf(connection.id)
    const starred = stars().includes(connection.id)
    return [
      { label: "关闭连接", icon: <Power size={15} />, onSelect: () => void closeConnection(connection) },
      { separator: true },
      { label: "编辑连接…", icon: <SquarePen size={15} />, disabled: true },
      { label: "新建连接", icon: <Plus size={15} />, onSelect: () => props.onNewConnection() },
      { label: "删除连接", icon: <Trash2 size={15} />, onSelect: () => void removeConnection(connection) },
      { label: "复制连接…", icon: <Copy size={15} />, onSelect: () => void duplicateConnection(connection) },
      { separator: true },
      { label: "新建查询", icon: <Terminal size={15} />, onSelect: () => props.onNewQuery() },
      { separator: true },
      {
        label: starred ? "取消星标" : "添加星标",
        icon: <Star size={15} />,
        onSelect: () => toggleStar(connection),
      },
      {
        label: "颜色",
        swatches: CONNECTION_COLORS,
        onSwatch: (value) => void setConnectionColor(connection, value),
      },
      { separator: true },
      {
        label: "管理组",
        icon: <Folder size={15} />,
        submenu: [
          {
            label: "新建文件夹…",
            onSelect: () => {
              setFolderName("")
              setFolderDialog({ kind: "new" })
            },
          },
          ...(folders().length
            ? [
                { separator: true } as MenuItem,
                ...folders().map((item) => ({
                  label: `移入「${item.name}」`,
                  checked: folder?.id === item.id,
                  onSelect: () => moveToFolder(connection, item.id),
                })),
              ]
            : []),
          ...(folder
            ? ([
                { separator: true } as MenuItem,
                { label: "移出分组", onSelect: () => moveToFolder(connection, null) },
              ] as MenuItem[])
            : []),
        ],
      },
      { separator: true },
      { label: "刷新", icon: <RefreshCw size={15} />, onSelect: () => void refreshConnections() },
    ]
  }

  function renderConnection(conn: DbxConnection, depth: number) {
    const icon = connectionIcon(conn.db_type)
    return (
      <div
        class="tree-group"
        classList={{ "conn-tinted": !!conn.color }}
        style={conn.color ? { "--conn-color": conn.color } : undefined}
        data-open={openConnections()[conn.id] ? "true" : "false"}
      >
        <button
          class="tree-row db-row"
          classList={{ "conn-online": connectionOnline(conn) }}
          style={{ "padding-left": depth > 0 ? `${depth * 14}px` : undefined }}
          onClick={() => toggleConnection(conn)}
          aria-expanded={openConnections()[conn.id] ? "true" : "false"}
          onContextMenu={(event) => {
            event.preventDefault()
            setConnectionMenu({ x: event.clientX, y: event.clientY, conn })
          }}
        >
          <Show when={icon} fallback={<Database size={16} />}>
            <img class="conn-icon" src={icon} alt="" loading="lazy" />
          </Show>
          <span class="tree-name">{conn.name}</span>
          <Show when={connectionOnline(conn)}>
            <span class="conn-dot" aria-label="已连接" title="已连接" />
          </Show>
          <span class="row-end">
            <Show when={stars().includes(conn.id)}>
              <Star size={11} class="conn-star" />
            </Show>
            <span class="tag mono">{conn.db_type}</span>
          </span>
        </button>
        <Show when={openConnections()[conn.id]}>
          <div class="tree-children">
            <Show when={!connectionData()[conn.id]?.loading} fallback={<div class="tree-hint">加载中…</div>}>
              <Show when={databasesOf(conn).length > 0} fallback={<div class="tree-hint">连接失败</div>}>
                <Show when={databasesOf(conn).length > 1} fallback={renderTables(conn, databasesOf(conn)[0], 20)}>
                  <For each={databasesOf(conn)}>
                    {(database) => (
                      <div class="tree-group" data-open={openDatabases()[`${conn.id}|${database}`] ? "true" : "false"}>
                        <button
                          class="tree-row tree-db"
                          onClick={() => toggleDatabase(conn, database)}
                          aria-expanded={openDatabases()[`${conn.id}|${database}`] ? "true" : "false"}
                        >
                          <ChevronRight
                            size={14}
                            class="chev"
                            classList={{ open: openDatabases()[`${conn.id}|${database}`] }}
                          />
                          <Database size={16} />
                          <span class="tree-name">{database}</span>
                        </button>
                        <Show when={openDatabases()[`${conn.id}|${database}`]}>
                          <div class="tree-children">{renderTables(conn, database, 34)}</div>
                        </Show>
                      </div>
                    )}
                  </For>
                </Show>
              </Show>
            </Show>
          </div>
        </Show>
      </div>
    )
  }

  return (
    <aside
      class="sidebar"
      style={{
        width: `${props.width}px`,
        "flex-basis": `${props.width}px`,
        "--panel-w": `${props.contentWidth}px`,
      }}
    >
      <div class="side-top">
        <WindowControls
          maximized={props.maximized}
          onMinimize={props.onMinimize}
          onMaximize={props.onMaximize}
          onClose={props.onClose}
        />
        <button class="side-collapse" aria-label="收起侧栏" title="收起侧栏" onClick={props.onToggle}>
          <PanelLeft size={16} />
        </button>
      </div>

      <nav class="side-nav">
        <button class="side-nav-row" onClick={props.onNewConnection}>
          <SquarePen size={16} />
          新建连接
        </button>
        <button class="side-nav-row" onClick={props.onNewQuery}>
          <Terminal size={16} />
          新建查询
        </button>
        <button class="side-nav-row" onClick={props.onCommonActions}>
          <Boxes size={16} />
          常用操作
        </button>
      </nav>

      <div
        class="tree-scroll"
        onContextMenu={(event) => {
          if ((event.target as HTMLElement).closest(".tree-row")) return
          event.preventDefault()
          setSidebarMenu({ x: event.clientX, y: event.clientY })
        }}
      >
        <div class="tree-section">
          <div class="tree-label">
            <Database size={16} />
            我的连接
            <button
              class="tree-locate"
              aria-label="定位当前表"
              title="定位当前表"
              onClick={() => void locateActiveTable()}
            >
              <LocateFixed size={15} />
            </button>
          </div>
          <For each={folders()}>
            {(folder) => (
              <div class="tree-group" data-open={folderOpen(folder.id) ? "true" : "false"}>
                <button
                  class="tree-row folder-row"
                  onClick={() => toggleFolder(folder.id)}
                  aria-expanded={folderOpen(folder.id) ? "true" : "false"}
                >
                  <Show when={folderOpen(folder.id)} fallback={<Folder size={16} />}>
                    <FolderOpen size={16} />
                  </Show>
                  <span class="tree-name">{folder.name}</span>
                  <span class="row-end">
                    <span class="tree-count">{folderConnections(folder).length}</span>
                  </span>
                </button>
                <Show when={folderOpen(folder.id)}>
                  <div class="tree-children">
                    <For each={folderConnections(folder)}>{(conn) => renderConnection(conn, 1)}</For>
                  </div>
                </Show>
              </div>
            )}
          </For>
          <For each={ungroupedConnections()}>{(conn) => renderConnection(conn, 0)}</For>
          <Show when={props.backendStatus === "online" && connections().length === 0}>
            <div class="tree-row leaf muted">暂无保存的连接</div>
          </Show>
        </div>
      </div>

      <Show when={sidebarMenu()}>
        {(menu) => (
          <ContextMenu
            x={menu().x}
            y={menu().y}
            items={sidebarMenuItems()}
            onClose={() => setSidebarMenu(null)}
          />
        )}
      </Show>
      <Show when={manageOpen()}>
        <div class="palette-backdrop" onClick={(event) => event.target === event.currentTarget && setManageOpen(false)}>
          <div class="dialog dialog-narrow" role="dialog" aria-modal="true">
            <header class="dialog-head">
              <h2>管理连接</h2>
              <button class="icon-btn" aria-label="关闭" onClick={() => setManageOpen(false)}>
                <X size={15} />
              </button>
            </header>
            <div class="dialog-body">
              <div class="column-list">
                <For each={connections()}>
                  {(connection) => (
                    <div class="manage-row">
                      <Show when={connectionIcon(connection.db_type)} fallback={<Database size={14} />}>
                        <img class="conn-icon" src={connectionIcon(connection.db_type)} alt="" />
                      </Show>
                      <span class="manage-name">{connection.name}</span>
                      <small>{connection.db_type}</small>
                      <button
                        class="icon-btn"
                        aria-label={`删除 ${connection.name}`}
                        title="删除连接"
                        onClick={() => void removeConnection(connection)}
                      >
                        <Trash2 size={14} />
                      </button>
                    </div>
                  )}
                </For>
                <Show when={connections().length === 0}>
                  <div class="tree-hint">还没有连接</div>
                </Show>
              </div>
            </div>
            <footer class="dialog-foot">
              <div class="dialog-foot-right">
                <button class="primary-btn" onClick={() => setManageOpen(false)}>
                  完成
                </button>
              </div>
            </footer>
          </div>
        </div>
      </Show>
      <Show when={connectionMenu()}>
        {(menu) => (
          <ContextMenu
            x={menu().x}
            y={menu().y}
            title={menu().conn.name}
            items={connectionMenuItems(menu().conn)}
            onClose={() => setConnectionMenu(null)}
          />
        )}
      </Show>
      <Show when={folderDialog()}>
        <div class="palette-backdrop" onClick={(event) => event.target === event.currentTarget && setFolderDialog(null)}>
          <div class="dialog dialog-narrow" role="dialog" aria-modal="true">
            <header class="dialog-head">
              <h2>{folderDialog()?.kind === "rename" ? "重命名文件夹" : "新建文件夹"}</h2>
              <button class="icon-btn" aria-label="关闭" onClick={() => setFolderDialog(null)}>
                <X size={15} />
              </button>
            </header>
            <div class="dialog-body">
              <label class="field">
                <span>名称</span>
                <input
                  value={folderName()}
                  autofocus
                  onInput={(event) => setFolderName(event.currentTarget.value)}
                  onKeyDown={(event) => event.key === "Enter" && saveFolder()}
                />
              </label>
            </div>
            <footer class="dialog-foot">
              <div class="dialog-foot-right">
                <button class="ghost-btn" onClick={() => setFolderDialog(null)}>
                  取消
                </button>
                <button class="primary-btn" onClick={saveFolder}>
                  确定
                </button>
              </div>
            </footer>
          </div>
        </div>
      </Show>
    </aside>
  )
}

function Birth(props: { value: string }) {
  return (
    <Show when={props.value !== "—" && props.value !== "?-?"} fallback={<span class="nullable">{props.value}</span>}>
      <Show when={props.value.startsWith("?")} fallback={<>{props.value}</>}>
        <span class="nullable">?</span>
        {props.value.slice(1)}
      </Show>
    </Show>
  )
}

export interface ColumnViewState {  hidden: string[]
  widths: Record<string, number>
  /** how many leading columns stay pinned while scrolling sideways */
  frozen: number
  showTypes: boolean
  showComments: boolean
  /** font scale for the grid, 1 = default */
  scale: number
  /** explicit row height in px; undefined follows the theme */
  rowHeight?: number
}

export const DEFAULT_COLUMN_VIEW: ColumnViewState = {
  hidden: [],
  widths: {},
  frozen: 0,
  showTypes: true,
  showComments: false,
  scale: 1,
}

function quoteName(quote: string, name: string): string {
  return `${quote}${name.split(quote).join(quote + quote)}${quote}`
}

function sqlLiteral(value: string | number | boolean | null): string {
  if (value === null || value === undefined) return "NULL"
  if (typeof value === "number" || typeof value === "boolean") return String(value)
  return `'${String(value).replace(/'/g, "''")}'`
}

function textOf(value: string | number | boolean | null): string {
  return value === null || value === undefined ? "" : String(value)
}

/** Newline-separated values; CSV mode quotes fields that need it. */
function delimited(values: Array<string | number | boolean | null>, separator: string, quoteFields: boolean): string {
  return values
    .map((value) => {
      const text = textOf(value)
      if (!quoteFields) return text
      return text.includes(separator) || text.includes('"') || text.includes("\n")
        ? `"${text.replace(/"/g, '""')}"`
        : text
    })
    .join("\n")
}

interface DataTableProps {
  columns: Array<{ name: string; type: string; primary?: boolean; comment?: string | null }>
  rows: (string | number | boolean | null)[][]
  selectedKey: string | null
  onSelect: (key: string) => void
  emptyMessage?: string
  rowStyle: RowStyle
  view?: ColumnViewState
  onColumnMenu?: (index: number, x: number, y: number) => void
  /** original row index for each entry in rows; defaults to the position */
  rowIndexes?: number[]
  /** marks rows staged for insertion */
  newRowFlags?: boolean[]
  onRowMenu?: (rowIndex: number, x: number, y: number) => void
  onCellMenu?: (rowIndex: number, columnIndex: number, x: number, y: number) => void
  /** absent disables cell editing */
  onEdit?: (rowIndex: number, columnIndex: number, value: string) => void
  /** staged edits keyed by `${rowIndex}|${columnIndex}` */
  pending?: Record<string, string | number | null>
  /** cell highlighted by the last context-menu hit */
  selectedCell?: { row: number; col: number } | null
}

/** Row tint for the configured striping mode. */
function stripeOf(index: number, mode: RowStyle): string {  if (mode === "zebra") return index % 2 === 1 ? "b" : ""
  if (mode === "triple") {
    const position = (index + 1) % 6
    if (position === 3) return "a"
    if (position === 0) return "b"
  }
  return ""
}

function DataTable(props: DataTableProps) {
  const [editing, setEditing] = createSignal<{ row: number; col: number } | null>(null)
  const [draft, setDraft] = createSignal("")

  const view = () => props.view ?? DEFAULT_COLUMN_VIEW
  const visible = createMemo(() =>
    props.columns
      .map((column, index) => ({ ...column, index }))
      .filter((column) => !view().hidden.includes(column.name)),
  )
  const frozen = () => Math.min(view().frozen, visible().length)
  const widthOf = (name: string) => view().widths[name]

  /** sticky left offsets for the frozen columns, including the row-number gutter */
  const frozenOffsets = createMemo(() => {
    const offsets: number[] = []
    let left = 48
    for (let i = 0; i < frozen(); i += 1) {
      offsets.push(left)
      left += widthOf(visible()[i].name) ?? 150
    }
    return offsets
  })

  const template = () => {
    const parts = ["48px"]
    for (const column of visible()) {
      const width = widthOf(column.name)
      parts.push(width ? `${width}px` : "minmax(150px, 1fr)")
    }
    return parts.join(" ")
  }
  const minWidth = () => 48 + visible().reduce((sum, column) => sum + (widthOf(column.name) ?? 150), 0)

  const isEditing = (row: number, col: number) => editing()?.row === row && editing()?.col === col

  function beginEdit(row: number, col: number, cell: string | number | boolean | null) {
    if (!props.onEdit) return
    setDraft(cell === null ? "" : String(cell))
    setEditing({ row, col })
  }

  function commitEdit() {
    const target = editing()
    if (!target) return
    setEditing(null)
    props.onEdit?.(target.row, target.col, draft())
  }

  return (
    <div
      class="grid"
      classList={{ "rows-zebra": props.rowStyle === "zebra", "rows-triple": props.rowStyle === "triple" }}
      role="table"
      style={`--grid-cols: ${template()}; --grid-min: ${minWidth()}px; --grid-scale: ${view().scale}; ${
        view().rowHeight ? `--grid-row-h: ${view().rowHeight}px` : ""
      }`}
    >
      <div class="grid-head" role="row">
        <div class="th num" classList={{ frozen: frozen() > 0 }} style={frozen() > 0 ? "left: 0px" : undefined}>
          <span class="th-name">#</span>
        </div>
        <For each={visible()}>
          {(column, position) => (
            <div
              class="th"
              classList={{ frozen: position() < frozen() }}
              style={position() < frozen() ? `left: ${frozenOffsets()[position()]}px` : undefined}
              onContextMenu={(event) => {
                if (!props.onColumnMenu) return
                event.preventDefault()
                props.onColumnMenu(column.index, event.clientX, event.clientY)
              }}
            >
              <span class="th-name">
                <Show when={column.primary}>
                  <KeyRound size={9} class="key" />
                </Show>
                <span class="th-text">{column.name}</span>
              </span>
              <Show when={view().showTypes}>
                <span class="th-type">{column.type || "text"}</span>
              </Show>
              <Show when={view().showComments && column.comment}>
                <span class="th-comment">{column.comment}</span>
              </Show>
            </div>
          )}
        </For>
      </div>
      <Show
        when={props.rows.length > 0}
        fallback={
          <div class="grid-empty">
            <Show when={props.emptyMessage} fallback={<div class="tree-row leaf muted">暂无数据</div>}>
              {props.emptyMessage}
            </Show>
          </div>
        }
      >
        <For each={props.rows}>
          {(row, position) => {
            const rowIndex = () => props.rowIndexes?.[position()] ?? position()
            const isNew = () => props.newRowFlags?.[position()] ?? false
            const key = `${rowIndex()}`
            return (
              <div
                class="grid-row"
                classList={{ selected: props.selectedKey === key, "row-new": isNew() }}
                data-stripe={isNew() ? "" : stripeOf(position(), props.rowStyle)}
                onClick={() => props.onSelect(key)}
                onContextMenu={(event) => {
                  if (!props.onRowMenu) return
                  event.preventDefault()
                  props.onRowMenu(rowIndex(), event.clientX, event.clientY)
                }}
              >
                <div class="td num" classList={{ frozen: frozen() > 0 }} style={frozen() > 0 ? "left: 0px" : undefined}>
                  {rowIndex() + 1}
                </div>
                <For each={visible()}>
                  {(column, position) => {
                    const cellIndex = () => column.index
                    const cell = () => row[cellIndex()]
                    const staged = () => props.pending?.[`${rowIndex()}|${cellIndex()}`]
                    const hasStaged = () => staged() !== undefined
                    const display = () => {
                      const value = hasStaged() ? staged() : cell()
                      return value === null || value === undefined ? "NULL" : String(value)
                    }
                    return (
                      <div
                        class="td"
                        classList={{
                          nullable: !hasStaged() && cell() === null,
                          editing: isEditing(rowIndex(), cellIndex()),
                          dirty: hasStaged(),
                          frozen: position() < frozen(),
                          "cell-selected":
                            props.selectedCell?.row === rowIndex() && props.selectedCell?.col === cellIndex(),
                        }}
                        style={position() < frozen() ? `left: ${frozenOffsets()[position()]}px` : undefined}
                        title={props.onEdit ? "双击编辑" : display()}
                        onDblClick={() => beginEdit(rowIndex(), cellIndex(), hasStaged() ? staged() : cell())}
                        onContextMenu={(event) => {
                          if (!props.onCellMenu) return
                          event.preventDefault()
                          event.stopPropagation()
                          props.onCellMenu(rowIndex(), cellIndex(), event.clientX, event.clientY)
                        }}
                      >
                        <Show when={isEditing(rowIndex(), cellIndex())} fallback={display()}>
                          <input
                            class="cell-editor"
                            value={draft()}
                            autofocus
                            onInput={(event) => setDraft(event.currentTarget.value)}
                            onKeyDown={(event) => {
                              if (event.key === "Enter") {
                                event.preventDefault()
                                commitEdit()
                              } else if (event.key === "Escape") {
                                event.preventDefault()
                                setEditing(null)
                              }
                            }}
                            onBlur={commitEdit}
                          />
                        </Show>
                      </div>
                    )
                  }}
                </For>
              </div>
            )
          }}
        </For>
      </Show>
    </div>
  )
}

function GridFoot(props: { rowCount: number; selected: boolean; elapsedMs: number }) {
  return (
    <div class="grid-foot">
      <span>{props.rowCount} 行</span>
      <span class="foot-sep" />
      <span>{props.selected ? "已选 1 行" : "未选择"}</span>
      <span class="foot-sep" />
      <span>{props.elapsedMs} ms</span>
      <div class="grid-foot-right">
        <span>第 1 / 1 页</span>
        <button class="page-btn" aria-label="上一页" disabled>
          <ChevronLeftIcon />
        </button>
        <button class="page-btn" aria-label="下一页" disabled>
          <ChevronRight size={11} />
        </button>
        <span class="page-size">
          500 行/页
          <ChevronDown size={9} />
        </span>
      </div>
    </div>
  )
}

function ChevronLeftIcon() {
  return <ChevronRight size={11} style={{ transform: "rotate(180deg)" }} />
}

interface TableViewProps {
  loading: boolean
  connectionName: string
  database: string
  schema: string
  tableName: string
  columns: Array<{ name: string; type: string; primary?: boolean }>
  rows: (string | number | boolean | null)[][]
  rowCount: number
  elapsedMs: number
  selectedKey: string | null
  error: string | null
  onSelect: (key: string) => void
  onRefresh: () => void
  onExport: () => void
  rowStyle: RowStyle
  onEdit: (rowIndex: number, columnIndex: number, value: string) => void
  view: ColumnViewState
  onColumnMenu: (index: number, x: number, y: number) => void
  onRowMenu: (index: number, x: number, y: number) => void
  onCellMenu: (rowIndex: number, columnIndex: number, x: number, y: number) => void
  rows: Array<{ index: number; values: Array<string | number | boolean | null>; isNew: boolean }>
  pending: Record<string, string | number | null>
  selectedCell: { row: number; col: number } | null
  pendingCount: number
  onSaveEdits: () => void
  onCancelEdits: () => void
}

function TableView(props: TableViewProps) {
  return (
    <section class="view table-view">
      <div class="table-toolbar">
        <div class="table-heading">
          <Table2 size={15} class="table-ico" />
          <span class="table-name">{props.tableName || "选择一张表"}</span>
          <Show when={props.tableName}>
            <span class="table-path">{props.schema} · {props.database}</span>
            <span class="table-count">{props.rowCount} 行</span>
          </Show>
        </div>
        <div class="table-actions">
          <Show when={props.pendingCount > 0}>
            <span class="pending-note">{props.pendingCount} 处待保存</span>
            <button class="icon-btn" aria-label="取消修改" title="取消修改" onClick={props.onCancelEdits}>
              <X size={16} />
            </button>
            <button class="icon-btn save-edit" aria-label="保存修改" title="保存修改" onClick={props.onSaveEdits}>
              <Check size={16} />
            </button>
            <span class="toolbar-sep" />
          </Show>
          <button class="icon-btn" aria-label="刷新" title="刷新" onClick={props.onRefresh}>
            <RefreshCw size={16} />
          </button>
          <button class="icon-btn" aria-label="导出" title="导出" onClick={props.onExport}>
            <Download size={16} />
          </button>
          <button class="icon-btn" aria-label="更多" title="更多">
            <Ellipsis size={16} />
          </button>
        </div>
      </div>
      <Show
        when={!props.error}
        fallback={<div class="grid-error">{props.error}</div>}
      >
        <Show
          when={!props.loading}
          fallback={<div class="grid-loading">加载表中…</div>}
        >
          <DataTable
            columns={props.columns}
            rows={props.rows}
            selectedKey={props.selectedKey}
            onSelect={props.onSelect}
            emptyMessage={props.tableName ? "表里还没有数据" : "在左侧选择一张表"}
            rowStyle={props.rowStyle}
            onEdit={props.onEdit}
            pending={props.pending}
            view={props.view}
            onColumnMenu={props.onColumnMenu}
            rows={props.rows.map((row) => row.values)}
            rowIndexes={props.rows.map((row) => row.index)}
            newRowFlags={props.rows.map((row) => row.isNew)}
            onRowMenu={props.onRowMenu}
            onCellMenu={props.onCellMenu}
            selectedCell={props.selectedCell}
          />
        </Show>
      </Show>
      <GridFoot rowCount={props.rowCount} selected={!!props.selectedKey} elapsedMs={props.elapsedMs} />
    </section>
  )
}

function CodeLine(props: { n: number; active?: boolean; children: JSX.Element }) {
  return (
    <div class="code-line" classList={{ active: props.active }}>
      <span class="ln">{props.n}</span>
      <span class="code-text">{props.children}</span>
    </div>
  )
}

function QueryView(props: {
  selectedKey: string | null
  onSelect: (key: string) => void
  onToast: (message: string) => void
  connectionName: string
  database: string
  schema: string
  columns: Array<{ name: string; type: string; primary?: boolean }>
  rows: (string | number | boolean | null)[][]
  rowCount: number
  elapsedMs: number
  loading: boolean
  running: boolean
  error: string | null
  sql: string
  onSqlChange: (sql: string) => void
  onRun: () => void
  rowStyle: RowStyle
}) {
  return (
    <section class="view query-view">
      <div class="editor-pane">
        <div class="editor-toolbar">
          <button class="ctx-btn">
            <Database size={13} />
            {props.connectionName}
            <ChevronDown size={10} />
          </button>
          <span class="ctx-sep">/</span>
          <button class="ctx-btn">
            {props.database}
            <ChevronDown size={10} />
          </button>
          <span class="ctx-sep">/</span>
          <button class="ctx-btn">
            {props.schema}
            <ChevronDown size={10} />
          </button>
          <div class="editor-actions">
            <button class="icon-btn" aria-label="格式化 SQL" title="格式化 SQL" onClick={() => props.onToast("SQL 已格式化")}>
              <WandSparkles size={15} />
            </button>
            <button class="icon-btn" aria-label="执行计划" title="执行计划" onClick={() => props.onToast("执行计划已生成")}>
              <Info size={15} />
            </button>
            <span class="toolbar-sep" />
            <button class="run-btn" onClick={props.onRun} disabled={props.running}>
              <Play size={12} fill="currentColor" />
              {props.running ? "运行中…" : "运行"}
              <span class="run-kbd">⌘↵</span>
            </button>
          </div>
        </div>

        <textarea
          class="sql-editor mono"
          spellcheck={false}
          placeholder="输入 SQL，⌘↵ 运行"
          value={props.sql}
          onInput={(event) => props.onSqlChange(event.currentTarget.value)}
          onKeyDown={(event) => {
            if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
              event.preventDefault()
              props.onRun()
            }
          }}
          aria-label="SQL 编辑器"
        />
      </div>

      <div class="result-pane">
        <div class="result-toolbar">
          <div class="result-tabs">
            <button class="result-tab active">
              结果<span class="count-pill">{props.rowCount}</span>
            </button>
            <Show when={props.error}>
              <button class="result-tab error-tab">消息</button>
            </Show>
          </div>
          <span class="toolbar-sep" />
          <div class="result-actions">
            <button class="icon-btn" aria-label="刷新" title="刷新" onClick={props.onRun}>
              <RefreshCw size={16} />
            </button>
            <button class="icon-btn" aria-label="导出" title="导出">
              <Download size={16} />
            </button>
            <button class="icon-btn" aria-label="更多" title="更多">
              <Ellipsis size={16} />
            </button>
          </div>
        </div>
        <Show when={!props.error} fallback={<div class="grid-error">{props.error}</div>}>
          <DataTable
            columns={props.columns}
            rows={props.rows}
            selectedKey={props.selectedKey}
            onSelect={props.onSelect}
            emptyMessage="运行查询查看结果"
            rowStyle={props.rowStyle}
          />
        </Show>
        <GridFoot rowCount={props.rowCount} selected={!!props.selectedKey} elapsedMs={props.elapsedMs} />
     </div>
    </section>
  )
}

function AiView(props: {
  onToast: (message: string) => void
  onInsert: () => void
}) {
  const [thinking, setThinking] = createSignal(false)
  const [composer, setComposer] = createSignal("")
  const sql = `CREATE TABLE public.sanguo_characters (
  id          SERIAL PRIMARY KEY,
  name        VARCHAR(32) NOT NULL,
  style_name  VARCHAR(32),
  faction     VARCHAR(16) NOT NULL,
  role        VARCHAR(32),
  birth_death VARCHAR(32),
  bio         TEXT
);`

  async function copySql() {
    try {
      await navigator.clipboard.writeText(sql)
      props.onToast("SQL 已复制")
    } catch {
      props.onToast("SQL 已选中，可手动复制")
    }
  }

  function send() {
    if (!composer().trim()) {
      props.onToast("先描述你想查询什么")
      return
    }
    props.onToast("已发送 · 静态预览")
    setComposer("")
  }

  return (
    <div class="ai-view">
      <div class="ai-body">
        <div class="msg-user">帮我插入三国人物相关的数据表</div>
        <div class="msg-meta">14:32</div>

        <div class="ai-msg">
          <div class="ai-msg-head">
            <div class="ai-avatar">
              <Sparkles size={12} />
            </div>
            <span class="ai-name">DBX AI</span>
            <span class="ai-time">刚刚</span>
          </div>
          <div class="ai-text">
            <p>
              我先确认了 <strong>test_geometry</strong> 下现有的对象。当前没有三国人物相关的表，我会新建 <strong>public.sanguo_characters</strong> 并写入 12 位代表人物。
            </p>
          </div>

          <button class="think" aria-expanded={thinking()} onClick={() => setThinking(!thinking())}>
            <Check size={12} class="think-check" />
            思考过程
            <span>· 3 步 · 2.1s</span>
            <ChevronRight size={12} class="chev" classList={{ open: thinking() }} />
          </button>
          <Show when={thinking()}>
            <div class="think-body">
              <div class="think-step">
                <Check size={10} />
                <span>读取当前 schema，确认 3 张已有表</span>
              </div>
              <div class="think-step">
                <Check size={10} />
                <span>检查外键与命名冲突</span>
              </div>
              <div class="think-step">
                <Check size={10} />
                <span>生成建表与批量插入语句</span>
              </div>
            </div>
          </Show>

          <div class="sql-card">
            <div class="sql-head">
              <Terminal size={11} />
              SQL · PostgreSQL
              <div class="sql-actions">
                <button class="icon-btn" aria-label="复制 SQL" title="复制 SQL" onClick={copySql}>
                  <Copy size={11} />
                </button>
                <button class="icon-btn" aria-label="插入编辑器" title="插入编辑器" onClick={props.onInsert}>
                  <CornerDownLeftIcon />
                </button>
              </div>
            </div>
            <pre class="sql-code">{sql}</pre>
            <div class="sql-foot">
              <button class="mini-btn" onClick={props.onInsert}>
                <CornerDownLeftIcon />
                插入编辑器
              </button>
              <button class="mini-btn accent" onClick={() => props.onToast("SQL 已执行 · 12 行 · 2 ms")}>
                <Play size={10} fill="currentColor" />
                执行
              </button>
            </div>
          </div>

          <div class="ai-result">
            <Check size={12} />
            已创建 public.sanguo_characters · 写入 12 行 · 0 冲突
          </div>

          <div class="ai-text">
            <p>需要我继续生成一个按势力统计的查询吗？</p>
          </div>
          <div class="ai-suggest">
            <button class="suggest" onClick={() => setComposer("按势力统计人物数量")}>
              按势力统计
            </button>
            <button class="suggest" onClick={() => setComposer("生成这三张表的 ER 图")}>
              生成 ER 图
            </button>
          </div>

        </div>
      </div>

      <div class="ai-trust">
        <Lock size={9} />
        写操作会在执行前请求确认
      </div>
      <div class="ai-composer">
        <div class="composer-ctx">
          <span class="ctx-chip">
            <Table2 size={10} />@sanguo_characters
            <X size={8} />
          </span>
          <span class="ctx-chip">
            <Folder size={10} />@public
            <X size={8} />
          </span>
        </div>
        <textarea
          class="composer-input"
          rows="1"
          placeholder="描述你想查询什么，@ 可带入表上下文"
          aria-label="向 AI 提问"
          value={composer()}
          onInput={(event) => setComposer(event.currentTarget.value)}
          onKeyDown={(event) => {
            if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
              event.preventDefault()
              send()
            }
          }}
        />
        <div class="composer-row">
          <button class="icon-btn" aria-label="添加附件" title="添加附件">
            <Paperclip size={14} />
          </button>
          <button class="icon-btn" aria-label="插入上下文" title="插入上下文">
            <span class="at">@</span>
          </button>
          <button class="model-chip">
            自动
            <ChevronDown size={9} />
          </button>
          <button class="send-btn" aria-label="发送" onClick={send}>
            <Send size={12} />
          </button>
        </div>
      </div>
    </div>
  )
}

function CornerDownLeftIcon() {
  return <Terminal size={10} style={{ transform: "rotate(180deg)" }} />
}

function SqlLibraryView(props: { onToast: (message: string) => void }) {
  return (
    <div class="panel-view-body">
      <div class="panel-search">
        <Search size={12} />
        <input placeholder="搜索 SQL 文件" aria-label="搜索 SQL 文件" />
        <kbd>/</kbd>
      </div>
      <div class="sql-list">
        <div class="sql-group">
          <ChevronDown size={12} />
          123123<span>3</span>
        </div>
        <For each={sqlFiles.slice(0, 3)}>
          {(file) => (
            <button class="sql-row" onClick={() => props.onToast(`已打开 ${file.name}`)}>
              <FileText size={12} />
              {file.name}
              <span class="sql-tag">{file.tag}</span>
            </button>
          )}
        </For>
        <div class="sql-group">
          <ChevronRight size={12} />
          1212<span>1</span>
        </div>
        <div class="sql-group">
          <ChevronRight size={12} />
          未分类<span>4</span>
        </div>
        <For each={sqlFiles.slice(3)}>
          {(file) => (
            <button class="sql-row" onClick={() => props.onToast(`已打开 ${file.name}`)}>
              <FileText size={12} />
              {file.name}
              <span class="sql-tag">{file.tag}</span>
            </button>
          )}
        </For>
      </div>
      <div class="panel-foot">
        <span>12 个文件</span>
        <button class="link-btn" onClick={() => props.onToast("新建 SQL")}>
          新建 SQL
        </button>
      </div>
    </div>
  )
}

function HistoryView(props: { onToast: (message: string) => void }) {
  return (
    <div class="panel-view-body">
      <div class="panel-search">
        <Search size={12} />
        <input placeholder="搜索历史" aria-label="搜索历史" />
        <kbd>/</kbd>
      </div>
      <div class="history-list">
        <For each={historyItems}>
          {(item) => (
            <button class="history-row" onClick={() => props.onToast("已载入历史查询")}>
              <span class="hist-dot" classList={{ warn: item.warn }} />
              <span class="hist-main">
                <span class="hist-sql">{item.sql}</span>
                <span class="hist-meta">{item.meta}</span>
              </span>
            </button>
          )}
        </For>
      </div>
    </div>
  )
}

function RightPanel(props: {
  active: Panel
  width: number
  contentWidth: number
  onPanel: (panel: Panel) => void
  onToast: (message: string) => void
  onInsert: () => void
}) {
  return (
    <aside
      class="right-panel"
      style={{
        width: `${props.width}px`,
        "flex-basis": `${props.width}px`,
        "--panel-w": `${props.contentWidth}px`,
      }}
    >
      <div class="right-body">
        <div class="panel-tabs">
          <button class="panel-tab" classList={{ active: props.active === "ai" }} onClick={() => props.onPanel("ai")}>
            <Sparkles size={14} />
            AI 助手
          </button>
          <button class="panel-tab" classList={{ active: props.active === "sql" }} onClick={() => props.onPanel("sql")}>
            <BookOpen size={14} />
            SQL 库
          </button>
          <button class="panel-tab" classList={{ active: props.active === "history" }} onClick={() => props.onPanel("history")}>
            <History size={14} />
            历史
          </button>
        </div>
        <Show when={props.active === "ai"}>
          <AiView onToast={props.onToast} onInsert={props.onInsert} />
        </Show>
        <Show when={props.active === "sql"}>
          <SqlLibraryView onToast={props.onToast} />
        </Show>
        <Show when={props.active === "history"}>
          <HistoryView onToast={props.onToast} />
        </Show>
      </div>
    </aside>
  )
}

function CommonActionsDialog(props: { onClose: () => void; onAction: (id: string) => void }) {
  const actions = [
    { id: "open-sql-file", label: "打开 SQL 文件", icon: <FolderOpen size={15} /> },
    { id: "data-transfer", label: "数据传输", icon: <ArrowLeftRight size={15} /> },
    { id: "drivers", label: "驱动管理", icon: <Box size={15} /> },
    { id: "plugins", label: "插件中心", icon: <PlugZap size={15} /> },
    { id: "compare-schema", label: "比较架构", icon: <GitCompareArrows size={15} /> },
    { id: "compare-data", label: "比较数据", icon: <Table2 size={15} /> },
    { id: "backup", label: "数据库备份", icon: <CalendarClock size={15} /> },
    { id: "mcp", label: "MCP 设置", icon: <Share2 size={15} /> },
  ]
  const [query, setQuery] = createSignal("")
  const [index, setIndex] = createSignal(0)
  let input!: HTMLInputElement

  const filtered = createMemo(() => {
    const needle = query().trim().toLowerCase()
    if (!needle) return actions
    return actions.filter((item) => item.label.toLowerCase().includes(needle))
  })

  createEffect(() => {
    query()
    setIndex(0)
  })

  onMount(() => input?.focus())

  function run(id: string) {
    props.onAction(id)
    props.onClose()
  }

  function onKey(event: KeyboardEvent) {
    if (event.key === "ArrowDown") {
      event.preventDefault()
      setIndex((value) => (value + 1) % Math.max(filtered().length, 1))
    } else if (event.key === "ArrowUp") {
      event.preventDefault()
      setIndex((value) => (value - 1 + filtered().length) % Math.max(filtered().length, 1))
    } else if (event.key === "Enter") {
      event.preventDefault()
      const item = filtered()[index()]
      if (item) run(item.id)
    } else if (event.key === "Escape") {
      event.preventDefault()
      props.onClose()
    }
  }

  return (
    <div class="palette-backdrop" onClick={(event) => event.target === event.currentTarget && props.onClose()}>
      <div class="palette" role="dialog" aria-modal="true" aria-label="常用操作">
        <div class="palette-input">
          <Search size={16} />
          <input
            ref={input}
            value={query()}
            onInput={(event) => setQuery(event.currentTarget.value)}
            onKeyDown={onKey}
            placeholder="搜索操作"
            autocomplete="off"
          />
          <kbd>Esc</kbd>
        </div>
        <div class="palette-list">
          <div class="palette-group">常用操作</div>
          <For each={filtered()}>
            {(item, i) => (
              <button
                class="palette-item"
                classList={{ active: i() === index() }}
                onMouseEnter={() => setIndex(i())}
                onClick={() => run(item.id)}
              >
                {item.icon}
                {item.label}
              </button>
            )}
          </For>
          <Show when={filtered().length === 0}>
            <div class="palette-empty">没有找到匹配操作</div>
          </Show>
        </div>
        <div class="palette-foot">
          <span>
            <kbd>↑</kbd>
            <kbd>↓</kbd> 选择
          </span>
          <span>
            <kbd>↵</kbd> 打开
          </span>
          <span>
            <kbd>Esc</kbd> 关闭
          </span>
        </div>
      </div>
    </div>
  )
}

function Toast(props: { message: string | null }) {
  return (
    <Show when={props.message}>
      <div class="toast">
        <Check size={12} />
        {props.message}
      </div>
    </Show>
  )
}

const ROW_STYLE_KEY = "omdbx-row-style"
const GRID_FONT_KEY = "omdbx-grid-font"

function initialRowStyle(): RowStyle {
  const saved = localStorage.getItem(ROW_STYLE_KEY)
  return saved === "plain" || saved === "zebra" || saved === "triple" ? saved : "triple"
}

function initialGridFont(): GridFont {
  const saved = localStorage.getItem(GRID_FONT_KEY)
  return saved === "system-mono" || saved === "plex" || saved === "ui" ? saved : "system-mono"
}

export default function App() {
  const [theme, setTheme] = createSignal<Theme>((localStorage.getItem("dbx-theme") as Theme) || "light")
  const [maximized, setMaximized] = createSignal(false)
  const [sidebarCollapsed, setSidebarCollapsed] = createSignal(false)
  const [rightCollapsed, setRightCollapsed] = createSignal(false)
  const [activePanel, setActivePanel] = createSignal<Panel>("ai")
  const [settingsOpen, setSettingsOpen] = createSignal(false)
  const [rowStyle, setRowStyle] = createSignal<RowStyle>(initialRowStyle())
  const [update, setUpdate] = createSignal<UpdateInfo | null>(null)
  const [gridFont, setGridFont] = createSignal<GridFont>(initialGridFont())
  const [backendStatus, setBackendStatus] = createSignal<"checking" | "online" | "offline">("checking")
  const [connections, setConnections] = createSignal<DbxConnection[]>([])
  const [activeTarget, setActiveTarget] = createSignal<{
    connection: DbxConnection
    database: string
    schema: string
    table: string
  } | null>(null)
  const [browse, setBrowse] = createSignal<{
    loading: boolean
    columns: Array<{ name: string; type: string; primary?: boolean; comment?: string | null }>
    rows: (string | number | boolean | null)[][]
    rowCount: number
    elapsedMs: number
    error: string | null
    quote: string
  }>({ loading: false, columns: [], rows: [], rowCount: 0, elapsedMs: 0, error: null, quote: '"' })
  const [selectedKey, setSelectedKey] = createSignal<string | null>(null)
  const [selectedCell, setSelectedCell] = createSignal<{ row: number; col: number } | null>(null)
  const [pendingEdits, setPendingEdits] = createSignal<Record<string, string | number | null>>({})
  const [columnViews, setColumnViews] = createSignal<Record<string, ColumnViewState>>({})
  const [columnMenu, setColumnMenu] = createSignal<{ x: number; y: number; index: number } | null>(null)
  const [columnDialog, setColumnDialog] = createSignal<{ kind: "hide" | "width" | "allWidth" | "rowHeight"; index: number } | null>(
    null,
  )
  const [dialogValue, setDialogValue] = createSignal("")
  const [rowMenu, setRowMenu] = createSignal<{ x: number; y: number; index: number } | null>(null)
  const [cellMenu, setCellMenu] = createSignal<{ x: number; y: number; row: number; column: number } | null>(null)
  /** rows staged for insertion; each cell holds the raw text */
  const [newRows, setNewRows] = createSignal<Array<Array<string | null>>>([])
  /** indexes into browse().rows staged for deletion */
  const [pendingDeletes, setPendingDeletes] = createSignal<number[]>([])
  const [querySql, setQuerySql] = createSignal("")
  const [queryState, setQueryState] = createSignal<{
    running: boolean
    columns: Array<{ name: string; type: string; primary?: boolean }>
    rows: (string | number | boolean | null)[][]
    rowCount: number
    elapsedMs: number
    error: string | null
  }>({ running: false, columns: [], rows: [], rowCount: 0, elapsedMs: 0, error: null })
  const [sidebarWidth, setSidebarWidth] = createSignal(window.innerWidth < 1320 ? 246 : 264)
  const [rightWidth, setRightWidth] = createSignal(window.innerWidth < 1320 ? 320 : window.innerWidth < 1500 ? 340 : 360)
  const [tabs, setTabs] = createSignal<TabItem[]>([])
  const [activeTab, setActiveTab] = createSignal("")
  const [selectedId, setSelectedId] = createSignal(3)
  const [newConnectionOpen, setNewConnectionOpen] = createSignal(false)
  const [commonActionsOpen, setCommonActionsOpen] = createSignal(false)
  const [connectionReload, setConnectionReload] = createSignal(0)
  const [pendingConnectionId, setPendingConnectionId] = createSignal<string | null>(null)
  const [toast, setToast] = createSignal<string | null>(null)
  let toastTimer: number | undefined

  function showToast(message: string) {
    setToast(message)
    if (toastTimer) window.clearTimeout(toastTimer)
    toastTimer = window.setTimeout(() => setToast(null), 2200)
  }

  function handleConnectionCreated(connection: DbxConnectionInput) {
    setPendingConnectionId(connection.id)
    setConnectionReload((value) => value + 1)
    showToast(`已保存连接「${connection.name}」`)
  }

  const COMMON_ACTION_LABELS: Record<string, string> = {
    "open-sql-file": "打开 SQL 文件",
    "data-transfer": "数据传输",
    drivers: "驱动管理",
    plugins: "插件中心",
    "compare-schema": "比较架构",
    "compare-data": "比较数据",
    backup: "数据库备份",
    mcp: "MCP 设置",
  }

  function handleCommonAction(id: string) {
    const label = COMMON_ACTION_LABELS[id] ?? id
    if (id === "open-sql-file") {
      openQuery()
      return
    }
    showToast(`「${label}」暂未实现`)
  }

  onCleanup(() => {
    if (toastTimer) window.clearTimeout(toastTimer)
  })

  createEffect(() => {
    document.documentElement.dataset.theme = theme()
    localStorage.setItem("dbx-theme", theme())
    const meta = document.querySelector('meta[name="theme-color"]')
    meta?.setAttribute("content", theme() === "dark" ? "#141414" : "#ffffff")
  })

  createEffect(() => {
    localStorage.setItem(ROW_STYLE_KEY, rowStyle())
  })

  createEffect(() => {
    document.documentElement.dataset.gridFont = gridFont()
    localStorage.setItem(GRID_FONT_KEY, gridFont())
  })

  function toggleTheme() {
    setTheme(theme() === "dark" ? "light" : "dark")
  }

  function openQuery() {
    const existing = tabs().find((tab) => tab.kind === "query")
    if (!existing) {
      setTabs([...tabs(), { id: "query", title: "查询", kind: "query", unsaved: true }])
    }
    setActiveTab("query")
    showToast("已打开查询编辑器")
  }

  function closeTab(id: string) {
    const current = tabs()
    const index = current.findIndex((tab) => tab.id === id)
    const next = current.filter((tab) => tab.id !== id)
    setTabs(next)
    if (activeTab() !== id) return
    if (next.length === 0) {
      setActiveTab("")
      setActiveTarget(null)
      setBrowse({ loading: false, columns: [], rows: [], rowCount: 0, elapsedMs: 0, error: null })
      return
    }
    setActiveTab(next[Math.max(0, index - 1)].id)
  }

  function handlePanel(panel: Panel) {
    setActivePanel(panel)
    setRightCollapsed(false)
  }

  async function refreshBackend() {
    setBackendStatus("checking")
    try {
      const list = await getConnections()
      setConnections(list)
      setBackendStatus("online")
    } catch {
      setConnections([])
      setBackendStatus("offline")
    }
  }

  async function openTable(connection: DbxConnection, database: string, schema: string, table: string) {
    const key = `${connection.id}|${database}|${schema}|${table}`
    setActiveTarget({ connection, database, schema, table })
    setTabs((list) =>
      list.some((tab) => tab.id === key) ? list : [...list, { id: key, title: table, kind: "table", sub: schema }],
    )
    setActiveTab(key)
    setPendingEdits({})
    setNewRows([])
    setPendingDeletes([])
    setSelectedCell(null)
    setBrowse((state) => ({ ...state, loading: true, error: null }))
    try {
      const { result, columns, sql, quote } = await browseTable(connection, database, schema, table)
      setQuerySql(`${sql};`)
      setBrowse({
        loading: false,
        quote,
        columns: columns.map((column) => ({
          name: column.name,
          type: column.data_type,
          primary: column.is_primary_key,
          comment: column.comment,
        })),
        rows: result.rows,
        rowCount: result.rows.length,
        elapsedMs: result.execution_time_ms,
        error: null,
      })
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setBrowse({ loading: false, columns: [], rows: [], rowCount: 0, elapsedMs: 0, error: message })
    }
  }

  async function refreshTable() {
    const target = activeTarget()
    if (!target) {
      showToast("先在左侧选择一张表")
      return
    }
    await openTable(target.connection, target.database, target.schema, target.table)
    showToast("结果已刷新")
  }

  /** Empty input clears the cell to NULL; numbers stay numbers when the original was one. */
  function parseCellValue(original: string | number | boolean | null, text: string): string | number | null {
    if (text === "") return null
    if (typeof original === "number" && !Number.isNaN(Number(text))) return Number(text)
    return text
  }

  const rowModel = createMemo(() => {
    const deletes = pendingDeletes()
    const model: Array<{ index: number; values: Array<string | number | boolean | null>; isNew: boolean }> = []
    browse().rows.forEach((values, index) => {
      if (!deletes.includes(index)) model.push({ index, values, isNew: false })
    })
    newRows().forEach((values, position) => {
      model.push({ index: browse().rows.length + position, values, isNew: true })
    })
    return model
  })

  function stageEdit(rowIndex: number, columnIndex: number, value: string | number | null) {
    const realCount = browse().rows.length
    if (rowIndex >= realCount) {
      const position = rowIndex - realCount
      const raw = value === null ? null : String(value)
      setNewRows((rows) =>
        rows.map((row, index) => (index === position ? row.map((cell, column) => (column === columnIndex ? raw : cell)) : row)),
      )
      return
    }
    const original = browse().rows[rowIndex]?.[columnIndex] ?? null
    const key = `${rowIndex}|${columnIndex}`
    setPendingEdits((state) => {
      const next = { ...state }
      if (value === original) delete next[key]
      else next[key] = value
      return next
    })
  }

  /** From the inline editor: empty text becomes NULL unless the cell was already empty. */
  function stageText(rowIndex: number, columnIndex: number, text: string) {
    const original = browse().rows[rowIndex]?.[columnIndex] ?? null
    stageEdit(rowIndex, columnIndex, parseCellValue(original, text))
  }

  function cancelEdits() {
    setPendingEdits({})
    showToast("已放弃修改")
  }

  async function saveEdits() {
    const target = activeTarget()
    const edits = pendingEdits()
    const keys = Object.keys(edits)
    const added = newRows()
    const removed = pendingDeletes()
    if (!target || (keys.length === 0 && added.length === 0 && removed.length === 0)) return
    const dirtyRows = keys.map((key) => {
      const [row, column] = key.split("|").map(Number)
      const entry: [number, Array<[number, string | number | null]>] = [row, [[column, edits[key]]]]
      return entry
    })
    try {
      const preparation = await prepareDataGridSave({
        databaseType: target.connection.db_type,
        tableMeta: {
          database: target.database,
          schema: target.schema,
          tableName: target.table,
          primaryKeys: browse()
            .columns.filter((item) => item.primary)
            .map((item) => item.name),
        },
        columns: browse().columns.map((item) => item.name),
        rows: browse().rows,
        dirtyRows,
        deletedRows: removed,
        newRows: added.map((row) => row.map((cell) => parseCellValue(null, cell ?? ""))),
      })
      if (preparation.validationError) throw new Error(preparation.validationError)
      if (preparation.keylessGuards?.length) throw new Error("这张表没有主键，暂不支持在表格里直接编辑")
      for (const statement of preparation.statements) {
        await executeQuery(
          target.connection.id,
          target.database,
          statement,
          preparation.executionSchema ?? target.schema,
        )
      }
      setPendingEdits({})
      setNewRows([])
      setPendingDeletes([])
      showToast(`已保存 ${keys.length + added.length + removed.length} 处修改`)
      await openTable(target.connection, target.database, target.schema, target.table)
    } catch (error) {
      showToast(`保存失败：${error instanceof Error ? error.message : String(error)}`)
    }
  }

  function tableKeyOf(): string {
    const target = activeTarget()
    return target ? `${target.connection.id}|${target.database}|${target.schema}|${target.table}` : ""
  }

  function columnView(): ColumnViewState {
    return columnViews()[tableKeyOf()] ?? DEFAULT_COLUMN_VIEW
  }

  function updateColumnView(patch: Partial<ColumnViewState>) {
    const key = tableKeyOf()
    if (!key) return
    setColumnViews((state) => ({ ...state, [key]: { ...(state[key] ?? DEFAULT_COLUMN_VIEW), ...patch } }))
  }

  function visibleColumns() {
    return browse()
      .columns.map((column, index) => ({ ...column, index }))
      .filter((column) => !columnView().hidden.includes(column.name))
  }

  async function copyText(text: string, label: string) {
    try {
      await navigator.clipboard.writeText(text)
      showToast(`已复制${label}`)
    } catch {
      showToast("复制失败")
    }
  }

  /** Width that fits the header plus the widest value in the column. */
  function measuredWidth(index: number): number {
    const column = browse().columns[index]
    const canvas = document.createElement("canvas")
    const context = canvas.getContext("2d")
    const grid = document.querySelector(".grid")
    if (!context) return 150
    const style = grid ? getComputedStyle(grid) : null
    context.font = style ? `${style.fontSize} ${style.fontFamily}` : "12px monospace"
    let width = context.measureText(column?.name ?? "").width + 46
    for (const row of browse().rows) {
      width = Math.max(width, context.measureText(textOf(row[index])).width + 26)
    }
    return Math.min(600, Math.max(80, Math.ceil(width)))
  }

  function fitColumns(indexes: number[]) {
    const widths = { ...columnView().widths }
    for (const index of indexes) {
      const column = browse().columns[index]
      if (column) widths[column.name] = measuredWidth(index)
    }
    updateColumnView({ widths })
  }

  function freezeColumn(index: number) {
    const visible = visibleColumns()
    const position = visible.findIndex((column) => column.index === index)
    if (position < 0) return
    const head = document.querySelector(".grid-head")
    const cells = head ? Array.from(head.querySelectorAll(".th")) : []
    const widths = { ...columnView().widths }
    for (let i = 0; i <= position; i += 1) {
      const cell = cells[i + 1] as HTMLElement | undefined
      if (cell) widths[visible[i].name] = Math.round(cell.getBoundingClientRect().width)
    }
    updateColumnView({ frozen: position + 1, widths })
    showToast(`已冻结左侧 ${position + 1} 列`)
  }

  async function pasteColumn(index: number) {
    try {
      const text = await navigator.clipboard.readText()
      if (!text) {
        showToast("剪贴板是空的")
        return
      }
      const lines = text.replace(/\r\n/g, "\n").split("\n")
      const next = { ...pendingEdits() }
      let applied = 0
      for (let row = 0; row < browse().rows.length && row < lines.length; row += 1) {
        if (lines[row] === "") continue
        next[`${row}|${index}`] = lines[row]
        applied += 1
      }      setPendingEdits(next)
      showToast(applied > 0 ? `已粘贴 ${applied} 个值，记得保存` : "没有可粘贴的行")
    } catch {
      showToast("读取剪贴板失败")
    }
  }

  function columnMenuItems(index: number): MenuItem[] {
    const columns = browse().columns
    const column = columns[index]
    const target = activeTarget()
    if (!column || !target) return []
    const quote = browse().quote
    const rows = browse().rows
    const values = rows.map((row) => row[index])
    const table = quoteName(quote, target.table)
    const name = quoteName(quote, column.name)
    const keys = columns.filter((item) => item.primary)
    const keyIndexes = keys.map((key) => columns.findIndex((item) => item.name === key.name))
    const insertSql = rows
      .map((row) => `INSERT INTO ${table} (${name}) VALUES (${sqlLiteral(row[index])});`)
      .join("\n")
    const updateSql = keys.length
      ? rows
          .map(
            (row) =>
              `UPDATE ${table} SET ${name} = ${sqlLiteral(row[index])} WHERE ${keys
                .map((key, position) => `${quoteName(quote, key.name)} = ${sqlLiteral(row[keyIndexes[position]])}`)
                .join(" AND ")};`,
          )
          .join("\n")
      : ""
    const withHeader = (separator: string, quoted: boolean) =>
      [column.name, ...values.map((value) => (quoted && separator === "," ? delimited([value], ",", true) : textOf(value)))].join(
        "\n",
      )

    return [
      { label: "复制", icon: <Copy size={15} />, onSelect: () => void copyText(delimited(values, "\t", false), "列数据") },
      { label: "复制字段名称", icon: <Type size={15} />, onSelect: () => void copyText(column.name, "字段名称") },
      {
        label: "复制为", icon: <FileCode2 size={15} />,
        submenu: [
          { label: "Insert 语句", onSelect: () => void copyText(insertSql, "Insert 语句") },
          { label: "Update 语句", disabled: !updateSql, onSelect: () => void copyText(updateSql, "Update 语句") },
          { separator: true },
          { label: "制表符分隔值（数据）", onSelect: () => void copyText(delimited(values, "\t", false), "数据") },
          { label: "制表符分隔值（字段名称）", onSelect: () => void copyText(column.name, "字段名称") },
          { label: "制表符分隔值（字段名称和数据）", onSelect: () => void copyText(withHeader("\t", false), "字段名称和数据") },
          { separator: true },
          { label: "逗号分隔值（数据）", onSelect: () => void copyText(delimited(values, ",", true), "数据") },
          { label: "逗号分隔值（字段名称）", onSelect: () => void copyText(column.name, "字段名称") },
          { label: "逗号分隔值（字段名称和数据）", onSelect: () => void copyText(withHeader(",", true), "字段名称和数据") },
          { separator: true },
          { label: "JSON", onSelect: () => void copyText(JSON.stringify(values, null, 2), "JSON") },
        ],
      },
      { label: "粘贴", icon: <ClipboardPaste size={15} />, onSelect: () => void pasteColumn(index) },
      { separator: true },
      {
        label: "显示或隐藏列…", icon: <Columns3 size={15} />,
        onSelect: () => {
          setDialogValue("")
          setColumnDialog({ kind: "hide", index })
        },
      },
      {
        label: "隐藏列", icon: <EyeOff size={15} />,
        onSelect: () => updateColumnView({ hidden: [...columnView().hidden, column.name] }),
      },
      { label: "冻结列", icon: <Pin size={15} />, onSelect: () => freezeColumn(index) },
      { separator: true },
      {
        label: "设置列宽…", icon: <MoveHorizontal size={15} />,
        onSelect: () => {
          setDialogValue(String(Math.round(columnView().widths[column.name] ?? 150)))
          setColumnDialog({ kind: "width", index })
        },
      },
      {
        label: "设置所有列宽度…", icon: <MoveHorizontal size={15} />,
        onSelect: () => {
          setDialogValue("150")
          setColumnDialog({ kind: "allWidth", index })
        },
      },
      { label: "调整列宽以适应内容", icon: <WandSparkles size={15} />, onSelect: () => fitColumns([index]) },
      {
        label: "调整所有列的适合大小", icon: <WandSparkles size={15} />,
        onSelect: () => fitColumns(browse().columns.map((_, position) => position)),
      },
      { separator: true },
      {
        label: "显示字段类型", icon: <Type size={15} />,
        checked: columnView().showTypes,
        onSelect: () => updateColumnView({ showTypes: !columnView().showTypes }),
      },
      {
        label: "显示注释", icon: <MessageSquare size={15} />,
        checked: columnView().showComments,
        onSelect: () => updateColumnView({ showComments: !columnView().showComments }),
      },
      { separator: true },
      {
        label: "缩放", icon: <ZoomIn size={15} />,
        submenu: [0.9, 1, 1.1, 1.25].map((scale) => ({
          label: `${Math.round(scale * 100)}%`,
          checked: columnView().scale === scale,
          onSelect: () => updateColumnView({ scale }),
        })),
      },
    ]
  }

  function rowMenuItems(rowIndex: number): MenuItem[] {
    const target = activeTarget()
    const columns = browse().columns
    const realCount = browse().rows.length
    const isNew = rowIndex >= realCount
    if (!target || (!isNew && rowIndex >= realCount)) return []
    const values = isNew ? (newRows()[rowIndex - realCount] ?? []) : browse().rows[rowIndex]
    const quote = browse().quote
    const table = quoteName(quote, target.table)
    const names = columns.map((column) => quoteName(quote, column.name)).join(", ")
    const keys = columns.filter((column) => column.primary)
    const keyIndexes = keys.map((key) => columns.findIndex((column) => column.name === key.name))
    const insertSql = `INSERT INTO ${table} (${names}) VALUES (${values
      .map((value) => sqlLiteral(value as string | null))
      .join(", ")});`
    const updateSql = keys.length
      ? `UPDATE ${table} SET ${columns
          .map((column, index) => `${quoteName(quote, column.name)} = ${sqlLiteral(values[index] as string | null)}`)
          .join(", ")} WHERE ${keys
          .map((key, position) => `${quoteName(quote, key.name)} = ${sqlLiteral(values[keyIndexes[position]] as string | null)}`)
          .join(" AND ")};`
      : ""
    const headerLine = columns.map((column) => column.name).join("\t")
    const json = JSON.stringify(
      Object.fromEntries(columns.map((column, index) => [column.name, values[index] ?? null])),
      null,
      2,
    )

    return [
      { label: "添加记录", icon: <Plus size={15} />, onSelect: () => addRow() },
      { label: "删除记录", icon: <Trash2 size={15} />, onSelect: () => deleteRow(rowIndex) },
      { separator: true },
      { label: "复制", icon: <Copy size={15} />, onSelect: () => void copyText(delimited(values as Array<string | null>, "\t", false), "行数据") },
      { label: "复制字段名称", icon: <Type size={15} />, onSelect: () => void copyText(columns.map((column) => column.name).join("\t"), "字段名称") },
      {
        label: "复制为", icon: <FileCode2 size={15} />,
        submenu: [
          { label: "Insert 语句", onSelect: () => void copyText(insertSql, "Insert 语句") },
          { label: "Update 语句", disabled: !updateSql, onSelect: () => void copyText(updateSql, "Update 语句") },
          { separator: true },
          {
            label: "制表符分隔值（数据）",
            onSelect: () => void copyText(delimited(values as Array<string | null>, "\t", false), "数据"),
          },
          { label: "制表符分隔值（字段名称）", onSelect: () => void copyText(headerLine, "字段名称") },
          {
            label: "制表符分隔值（字段名称和数据）",
            onSelect: () => void copyText(`${headerLine}\n${delimited(values as Array<string | null>, "\t", false)}`, "字段名称和数据"),
          },
          { separator: true },
          {
            label: "逗号分隔值（数据）",
            onSelect: () => void copyText(delimited(values as Array<string | null>, ",", true), "数据"),
          },
          { label: "逗号分隔值（字段名称）", onSelect: () => void copyText(columns.map((column) => column.name).join(","), "字段名称") },
          {
            label: "逗号分隔值（字段名称和数据）",
            onSelect: () =>
              void copyText(`${columns.map((column) => column.name).join(",")}\n${delimited(values as Array<string | null>, ",", true)}`, "字段名称和数据"),
          },
          { separator: true },
          { label: "JSON", onSelect: () => void copyText(json, "JSON") },
        ],
      },
      { label: "粘贴", icon: <ClipboardPaste size={15} />, onSelect: () => void pasteRow(rowIndex) },
      { separator: true },
      {
        label: "设置行高…", icon: <MoveVertical size={15} />,
        onSelect: () => {
          setDialogValue(String(columnView().rowHeight ?? Math.round(document.querySelector(".grid-row")?.getBoundingClientRect().height ?? 30)))
          setColumnDialog({ kind: "rowHeight", index: rowIndex })
        },
      },
      {
        label: "缩放", icon: <ZoomIn size={15} />,
        submenu: [0.9, 1, 1.1, 1.25].map((scale) => ({
          label: `${Math.round(scale * 100)}%`,
          checked: columnView().scale === scale,
          onSelect: () => updateColumnView({ scale }),
        })),
      },
    ]
  }

  function addRow() {
    const width = browse().columns.length
    setNewRows((rows) => [...rows, Array.from({ length: width }, () => null)])
    showToast("已添加一行，填好后点保存")
  }

  function deleteRow(rowIndex: number) {
    const realCount = browse().rows.length
    if (rowIndex >= realCount) {
      setNewRows((rows) => rows.filter((_, position) => position !== rowIndex - realCount))
      showToast("已移除新增行")
      return
    }
    setPendingDeletes((deletes) => (deletes.includes(rowIndex) ? deletes : [...deletes, rowIndex]))
    showToast("已标记删除，点保存生效")
  }

  async function pasteRow(rowIndex: number) {
    try {
      const text = await navigator.clipboard.readText()
      if (!text) {
        showToast("剪贴板是空的")
        return
      }
      const cells = text.replace(/\r\n/g, "\n").split("\n")[0].split(text.includes("\t") ? "\t" : ",")
      const realCount = browse().rows.length
      if (rowIndex >= realCount) {
        const position = rowIndex - realCount
        setNewRows((rows) =>
          rows.map((row, index) =>
            index === position ? row.map((cell, column) => cells[column] ?? cell) : row,
          ),
        )
      } else {
        const next = { ...pendingEdits() }
        cells.forEach((cell, column) => {
          if (column < browse().columns.length) {
            const original = browse().rows[rowIndex]?.[column] ?? null
            next[`${rowIndex}|${column}`] = parseCellValue(original, cell)
          }
        })
        setPendingEdits(next)
      }
      showToast("已粘贴，记得保存")
    } catch {
      showToast("读取剪贴板失败")
    }
  }

  function cellMenuItems(rowIndex: number, columnIndex: number): MenuItem[] {
    const target = activeTarget()
    const columns = browse().columns
    const column = columns[columnIndex]
    const realCount = browse().rows.length
    const isNew = rowIndex >= realCount
    if (!target || !column) return []
    const values = isNew ? (newRows()[rowIndex - realCount] ?? []) : browse().rows[rowIndex]
    const value = values[columnIndex] ?? null
    const quote = browse().quote
    const table = quoteName(quote, target.table)
    const name = quoteName(quote, column.name)
    const keys = columns.filter((item) => item.primary)
    const keyIndexes = keys.map((key) => columns.findIndex((item) => item.name === key.name))
    const literal = sqlLiteral(value as string | null)
    const insertSql = `INSERT INTO ${table} (${name}) VALUES (${literal});`
    const updateSql = keys.length
      ? `UPDATE ${table} SET ${name} = ${literal} WHERE ${keys
          .map((key, position) => `${quoteName(quote, key.name)} = ${sqlLiteral(values[keyIndexes[position]] as string | null)}`)
          .join(" AND ")};`
      : ""
    const text = textOf(value as string | null)
    const withHeader = (separator: string, quoted: boolean) =>
      `${column.name}\n${quoted ? delimited([value as string | null], separator, true) : text}`

    return [
      { label: "设置为空白字符串", icon: <Eraser size={15} />, onSelect: () => stageEdit(rowIndex, columnIndex, "") },
      { label: "设置为 NULL", icon: <CircleSlash size={15} />, onSelect: () => stageEdit(rowIndex, columnIndex, null) },
      { separator: true },
      { label: "添加记录", icon: <Plus size={15} />, onSelect: () => addRow() },
      { label: "删除记录...", icon: <Trash2 size={15} />, onSelect: () => deleteRow(rowIndex) },
      { separator: true },
      { label: "复制", icon: <Copy size={15} />, onSelect: () => void copyText(text, "单元格") },
      { label: "复制字段名称", icon: <Type size={15} />, onSelect: () => void copyText(column.name, "字段名称") },
      {
        label: "复制为", icon: <FileCode2 size={15} />,
        submenu: [
          { label: "Insert 语句", onSelect: () => void copyText(insertSql, "Insert 语句") },
          { label: "Update 语句", disabled: !updateSql, onSelect: () => void copyText(updateSql, "Update 语句") },
          { separator: true },
          { label: "制表符分隔值（数据）", onSelect: () => void copyText(text, "数据") },
          { label: "制表符分隔值（字段名称）", onSelect: () => void copyText(column.name, "字段名称") },
          { label: "制表符分隔值（字段名称和数据）", onSelect: () => void copyText(withHeader("\t", false), "字段名称和数据") },
          { separator: true },
          { label: "逗号分隔值（数据）", onSelect: () => void copyText(delimited([value as string | null], ",", true), "数据") },
          { label: "逗号分隔值（字段名称）", onSelect: () => void copyText(column.name, "字段名称") },
          { label: "逗号分隔值（字段名称和数据）", onSelect: () => void copyText(withHeader(",", true), "字段名称和数据") },
          { separator: true },
          { label: "JSON", onSelect: () => void copyText(JSON.stringify(value, null, 2), "JSON") },
        ],
      },
      { label: "粘贴", icon: <ClipboardPaste size={15} />, onSelect: () => void pasteCell(rowIndex, columnIndex) },
      { separator: true },
      {
        label: "缩放", icon: <ZoomIn size={15} />,
        submenu: [0.9, 1, 1.1, 1.25].map((scale) => ({
          label: `${Math.round(scale * 100)}%`,
          checked: columnView().scale === scale,
          onSelect: () => updateColumnView({ scale }),
        })),
      },
      { label: "刷新", icon: <RefreshCw size={15} />, onSelect: () => void refreshTable() },
    ]
  }

  async function pasteCell(rowIndex: number, columnIndex: number) {
    try {
      const text = await navigator.clipboard.readText()
      if (!text) {
        showToast("剪贴板是空的")
        return
      }
      stageText(rowIndex, columnIndex, text.replace(/\r?\n/g, " ").trim())
      showToast("已粘贴，记得保存")
    } catch {
      showToast("读取剪贴板失败")
    }
  }

  function applyColumnDialog() {
    const dialog = columnDialog()
    if (!dialog) return
    if (dialog.kind === "width") {
      const column = browse().columns[dialog.index]
      const width = Number(dialogValue())
      if (column && width > 0) updateColumnView({ widths: { ...columnView().widths, [column.name]: Math.round(width) } })
    } else if (dialog.kind === "rowHeight") {
      const height = Number(dialogValue())
      if (height > 0) updateColumnView({ rowHeight: Math.round(height) })
    } else if (dialog.kind === "allWidth") {
      const width = Number(dialogValue())
      if (width > 0) {
        const widths: Record<string, number> = {}
        for (const column of browse().columns) widths[column.name] = Math.round(width)
        updateColumnView({ widths })
      }
    }
    setColumnDialog(null)
  }

  function toggleColumnVisibility(name: string) {
    const hidden = columnView().hidden
    updateColumnView({ hidden: hidden.includes(name) ? hidden.filter((item) => item !== name) : [...hidden, name] })
  }

  async function runQuery() {
    const active = activeTarget()
    if (!active) {
      showToast("先在左侧打开一个连接")
      return
    }
    setQueryState((state) => ({ ...state, running: true, error: null }))
    try {
      const result = await executeQuery(active.connection.id, active.database, querySql(), active.schema)
      setQueryState({
        running: false,
        columns: result.columns.map((name, index) => ({
          name,
          type: result.column_types?.[index] ?? "",
        })),
        rows: result.rows,
        rowCount: result.rows.length,
        elapsedMs: result.execution_time_ms,
        error: null,
      })
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setQueryState((state) => ({ ...state, running: false, error: message }))
    }
  }

  function startResize(kind: "sidebar" | "right", event: PointerEvent) {
    event.preventDefault()
    if (kind === "right" && rightCollapsed()) return
    const startX = event.clientX
    const startWidth = kind === "sidebar" ? sidebarWidth() : rightWidth()
    document.body.classList.add("resizing")
    const onMove = (moveEvent: PointerEvent) => {
      const dx = moveEvent.clientX - startX
      if (kind === "sidebar") setSidebarWidth(Math.min(380, Math.max(220, startWidth + dx)))
      else setRightWidth(Math.min(480, Math.max(300, startWidth - dx)))
    }
    const onUp = () => {
      document.body.classList.remove("resizing")
      window.removeEventListener("pointermove", onMove)
      window.removeEventListener("pointerup", onUp)
    }
    window.addEventListener("pointermove", onMove)
    window.addEventListener("pointerup", onUp)
  }

  function windowAction(action: "minimize" | "maximize" | "close") {
    if (!window.api) {
      showToast("窗口控制仅在 Electron 中可用")
      return
    }
    window.api[action]()
  }

  onMount(() => {
    void refreshBackend()
    void checkForUpdate().then(setUpdate)
    window.api?.onMaximized(setMaximized)
    const onKey = (event: KeyboardEvent) => {
      const mod = event.metaKey || event.ctrlKey
      if (mod && event.key === "Enter") {
        event.preventDefault()
        showToast("查询已执行 · 38 行 · 6 ms")
        return
      }
      if (mod && event.key.toLowerCase() === "b") {
        event.preventDefault()
        setSidebarCollapsed((collapsed) => !collapsed)
        return
      }
      if (mod && event.shiftKey && event.key.toLowerCase() === "l") {
        event.preventDefault()
        toggleTheme()
      }
    }
    window.addEventListener("keydown", onKey)
    onCleanup(() => window.removeEventListener("keydown", onKey))
  })

  return (
    <div class="app" classList={{ "sidebar-collapsed": sidebarCollapsed(), "right-collapsed": rightCollapsed() }}>
      <Show when={!settingsOpen()}>
        <Sidebar
          width={sidebarCollapsed() ? 0 : sidebarWidth()}
          contentWidth={sidebarWidth()}
          maximized={maximized()}
          backendStatus={backendStatus()}
          reloadToken={connectionReload()}
          pendingId={pendingConnectionId()}
          activeTab={activeTab()}
          onToggle={() => setSidebarCollapsed(true)}
          onNewConnection={() => setNewConnectionOpen(true)}
          onNewQuery={openQuery}
          onCommonActions={() => setCommonActionsOpen(true)}
          onConnectionsChanged={() => void refreshBackend()}
          onToast={showToast}
          onMinimize={() => windowAction("minimize")}
          onMaximize={() => windowAction("maximize")}
          onClose={() => windowAction("close")}
          onOpenTable={openTable}
        />
        <div class="panel-gap gap-left" classList={{ hidden: sidebarCollapsed() }} role="separator" aria-label="调整左栏宽度" onPointerDown={(event) => startResize("sidebar", event)} />
      </Show>
      <div class="main-shell">
        <Titlebar
          maximized={maximized()}
          tabs={tabs()}
          active={activeTab()}
          sidebarCollapsed={sidebarCollapsed()}
          rightCollapsed={rightCollapsed()}
          settingsOpen={settingsOpen()}
          theme={theme()}
          onToggleSidebar={() => setSidebarCollapsed((value) => !value)}
          tabColor={(tab) => connections().find((conn) => tab.id.startsWith(`${conn.id}|`))?.color ?? undefined}
          update={update()}
          onUpdate={() => {
            const info = update()
            if (!info) return
            if (window.api?.openExternal) window.api.openExternal(info.url)
            else window.open(info.url, "_blank")
          }}
          onToggleRight={() => setRightCollapsed((value) => !value)}
          onSelect={(id) => {
            setSettingsOpen(false)
            setActiveTab(id)
          }}
          onCloseTab={closeTab}
          onNewQuery={openQuery}
          onTheme={toggleTheme}
          onSettings={() => setSettingsOpen((value) => !value)}
          onMinimize={() => windowAction("minimize")}
          onMaximize={() => windowAction("maximize")}
          onClose={() => windowAction("close")}
        />
        <div class="main-content">
          <Show
            when={settingsOpen()}
            fallback={
              <>
                <main class="workbench">
                  <Show
                    when={activeTab() === "query"}
                    fallback={
                      <TableView
                        loading={browse().loading}
                        connectionName={activeTarget()?.connection.name ?? ""}
                        database={activeTarget()?.database ?? ""}
                        schema={activeTarget()?.schema ?? ""}
                        tableName={activeTarget()?.table ?? ""}
                        columns={browse().columns}
                        rows={browse().rows}
                        rowCount={browse().rowCount}
                        elapsedMs={browse().elapsedMs}
                        selectedKey={selectedKey()}
                        error={browse().error}
                        onSelect={setSelectedKey}
                        onRefresh={() => void refreshTable()}
                        onExport={() => showToast("导出结果 · CSV")}
                        rowStyle={rowStyle()}
                        onEdit={stageText}
                        pending={pendingEdits()}
                        pendingCount={
                          Object.keys(pendingEdits()).length + newRows().length + pendingDeletes().length
                        }
                        onSaveEdits={() => void saveEdits()}
                        onCancelEdits={cancelEdits}
                        view={columnView()}
                        onColumnMenu={(index, x, y) => setColumnMenu({ index, x, y })}
                        onRowMenu={(index, x, y) => setRowMenu({ index, x, y })}
                        onCellMenu={(row, column, x, y) => {
                          setSelectedCell({ row, col: column })
                          setCellMenu({ row, column, x, y })
                        }}
                        selectedCell={selectedCell()}
                        rows={rowModel()}
                      />
                    }
                  >
                    <QueryView
                      selectedKey={selectedKey()}
                      onSelect={setSelectedKey}
                      onToast={showToast}
                      connectionName={activeTarget()?.connection.name ?? "未连接"}
                      database={activeTarget()?.database ?? "main"}
                      schema={activeTarget()?.schema ?? "main"}
                      columns={queryState().columns}
                      rows={queryState().rows}
                      rowCount={queryState().rowCount}
                      elapsedMs={queryState().elapsedMs}
                      loading={false}
                      running={queryState().running}
                      error={queryState().error}
                      sql={querySql()}
                      onSqlChange={setQuerySql}
                      onRun={() => void runQuery()}
                      rowStyle={rowStyle()}
                    />
                  </Show>
                </main>
                <div class="panel-gap gap-right" classList={{ hidden: rightCollapsed() }} role="separator" aria-label="调整右栏宽度" onPointerDown={(event) => startResize("right", event)} />
                <RightPanel
                  active={activePanel()}
                  width={rightCollapsed() ? 0 : rightWidth()}
                  contentWidth={rightWidth()}
                  onPanel={handlePanel}
                  onToast={showToast}
                  onInsert={() => {
                    openQuery()
                    showToast("SQL 已插入编辑器")
                  }}
                />
              </>
            }
          >
            <SettingsView
              onClose={() => setSettingsOpen(false)}
              rowStyle={rowStyle()}
              onRowStyle={setRowStyle}
              gridFont={gridFont()}
              onGridFont={setGridFont}
            />
          </Show>
        </div>
      </div>
      <Show when={newConnectionOpen()}>
        <NewConnectionDialog
          onClose={() => setNewConnectionOpen(false)}
          onCreated={handleConnectionCreated}
          onToast={showToast}
        />
      </Show>
      <Show when={commonActionsOpen()}>
        <CommonActionsDialog onClose={() => setCommonActionsOpen(false)} onAction={handleCommonAction} />
      </Show>
      <Show when={columnMenu()}>
        {(menu) => (
          <ContextMenu
            x={menu().x}
            y={menu().y}
            title={`${activeTarget()?.table ?? ""} · ${browse().columns[menu().index]?.name ?? ""}`}
            items={columnMenuItems(menu().index)}
            onClose={() => setColumnMenu(null)}
          />
        )}
      </Show>
      <Show when={cellMenu()}>
        {(menu) => (
          <ContextMenu
            x={menu().x}
            y={menu().y}
            title={`${activeTarget()?.table ?? ""} · ${browse().columns[menu().column]?.name ?? ""}`}
            items={cellMenuItems(menu().row, menu().column)}
            onClose={() => setCellMenu(null)}
          />
        )}
      </Show>
      <Show when={rowMenu()}>
        {(menu) => (
          <ContextMenu
            x={menu().x}
            y={menu().y}
            title={`${activeTarget()?.table ?? ""} · 第 ${menu().index + 1} 行`}
            items={rowMenuItems(menu().index)}
            onClose={() => setRowMenu(null)}
          />
        )}
      </Show>
      <Show when={columnDialog()}>
        {(dialog) => (
          <div class="palette-backdrop" onClick={(event) => event.target === event.currentTarget && setColumnDialog(null)}>
            <div class="dialog dialog-narrow" role="dialog" aria-modal="true">
              <header class="dialog-head">
                <h2>
                  {dialog().kind === "hide"
                    ? "显示或隐藏列"
                    : dialog().kind === "width"
                      ? `设置列宽 · ${browse().columns[dialog().index]?.name ?? ""}`
                      : dialog().kind === "rowHeight"
                        ? "设置行高"
                        : "设置所有列宽度"}
                </h2>
                <button class="icon-btn" aria-label="关闭" onClick={() => setColumnDialog(null)}>
                  <X size={15} />
                </button>
              </header>
              <div class="dialog-body">
                <Show
                  when={dialog().kind === "hide"}
                  fallback={
                    <label class="field">
                      <span>宽度（像素）</span>
                      <input
                        type="number"
                        value={dialogValue()}
                        autofocus
                        onInput={(event) => setDialogValue(event.currentTarget.value)}
                        onKeyDown={(event) => event.key === "Enter" && applyColumnDialog()}
                      />
                    </label>
                  }
                >
                  <div class="column-list">
                    <For each={browse().columns}>
                      {(column) => (
                        <label class="column-toggle">
                          <input
                            type="checkbox"
                            checked={!columnView().hidden.includes(column.name)}
                            onChange={() => toggleColumnVisibility(column.name)}
                          />
                          <span class="column-toggle-name">{column.name}</span>
                          <small>{column.type}</small>
                        </label>
                      )}
                    </For>
                  </div>
                </Show>
              </div>
              <footer class="dialog-foot">
                <div class="dialog-foot-right">
                  <button class="ghost-btn" onClick={() => setColumnDialog(null)}>
                    取消
                  </button>
                  <button class="primary-btn" onClick={applyColumnDialog}>
                    确定
                  </button>
                </div>
              </footer>
            </div>
          </div>
        )}
      </Show>
      <Toast message={toast()} />
    </div>
  )
}