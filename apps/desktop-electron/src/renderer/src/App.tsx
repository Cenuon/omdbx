import {
  ArrowUpDown,
  BookOpen,
  Braces,
  Check,
  ChevronDown,
  ChevronRight,
  Clock,
  Copy,
  Database,
  Download,
  Ellipsis,
  FileText,
  Filter,
  Folder,
  Grip,
  History,
  Info,
  LayoutGrid,
  KeyRound,
  List,
  Lock,
  Moon,
  PanelLeft,
  PanelRight,
  Paperclip,
  Play,
  Plus,
  RefreshCw,
  Search,
  Send,
  Settings2,
  Sparkles,
  SquarePen,
  Sun,
  Table2,
  Terminal,
  WandSparkles,
  X,
} from "lucide-solid"
import { For, Show, createEffect, createMemo, createSignal, onCleanup, onMount, type JSX } from "solid-js"
import SettingsView from "./SettingsView"
import { historyItems, sqlFiles, type TabItem } from "./data"
import {
  browseTable,
  connectDb,
  executeQuery,
  getConnections,
  getDatabases,
  getSchemas,
  getTables,
  type DbxConnection,
  type DbxConnectionInput,
} from "./dbx/client"
import NewConnectionDialog from "./NewConnectionDialog"

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
  onCloseTab: (id: string) => void
  onNewQuery: () => void
  onTheme: () => void
  onSettings: () => void
  onCommand: () => void
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

      <div class="titlebar-tabs" role="tablist">
        <For each={props.tabs}>
          {(tab) => (
            <div
              class="tab"
              classList={{ active: props.active === tab.id }}
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
        <button class="tab-add" aria-label="新建查询" title="新建查询" onClick={props.onNewQuery}>
          <Plus size={14} />
        </button>
      </div>

      <div class="titlebar-right">
        <button class="titlebar-toggle" aria-label="命令面板" title="命令面板 ⌘K" onClick={props.onCommand}>
          <Search size={16} />
        </button>
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
          <Settings2 size={16} />
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
  onToggle: () => void
  onNewConnection: () => void
  onToast: (message: string) => void
  onMinimize: () => void
  onMaximize: () => void
  onClose: () => void
  onOpenTable: (connection: DbxConnection, database: string, schema: string, table: string) => void
}

/** Catalog schemas that only get in the way of browsing real data. */
const SYSTEM_DATABASES = new Set([
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
  const [activeKey, setActiveKey] = createSignal("")

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
    void refreshConnections().then((list) => {
      if (list.length === 0) return
      const next: Record<string, boolean> = {}
      list.forEach((conn, index) => {
        next[conn.id] = index === 0
      })
      setOpenConnections(next)
      void loadConnection(list[0])
      setActiveKey(`${list[0].id}|${list[0].database || "main"}|${list[0].default_schema || "main"}|`)
    })
  })

  // a connection was just created: reload the list and open it
  let lastReload = props.reloadToken
  createEffect(() => {
    const token = props.reloadToken
    if (token === lastReload) return
    lastReload = token
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
    setActiveKey(`${conn.id}|${database}|${schema}|${table}`)
    props.onOpenTable(conn, database, schema, table)
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
                classList={{ active: activeKey() === `${conn.id}|${database}|${row.schema}|${row.table}` }}
                style={{ "padding-left": `${indent}px` }}
                onClick={() => openTable(conn, database, row.schema, row.table)}
              >
                <Table2 size={12} />
                <span class="tree-name">{row.table}</span>
              </button>
            )}
          </For>
        </Show>
      </Show>
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
        <button class="side-nav-row" onClick={() => props.onToast("主页")}>
          <LayoutGrid size={16} />
          主页
        </button>
        <label class="side-nav-row side-search-row">
          <Search size={16} />
          <input placeholder="搜索连接和表" aria-label="搜索连接和表" />
        </label>
        <button class="side-nav-row" onClick={props.onNewConnection}>
          <SquarePen size={16} />
          新建连接
        </button>
      </nav>

      <div class="tree-scroll">
        <div class="tree-section">
          <div class="tree-label">
            <ChevronDown size={12} />
            已连接
            <span>{connections().length}</span>
          </div>
          <For each={connections()}>
            {(conn) => (
              <div class="tree-group" data-open={openConnections()[conn.id] ? "true" : "false"}>
                <button
                  class="tree-row db-row"
                  onClick={() => toggleConnection(conn)}
                  aria-expanded={openConnections()[conn.id] ? "true" : "false"}
                >
                  <ChevronRight size={13} class="chev" classList={{ open: openConnections()[conn.id] }} />
                  <Database size={15} />
                  <span class="tree-name">{conn.name}</span>
                  <span class="row-end">
                    <span class="tag mono">{conn.db_type}</span>
                  </span>
                </button>
                <Show when={openConnections()[conn.id]}>
                  <div class="tree-children">
                    <Show when={!connectionData()[conn.id]?.loading} fallback={<div class="tree-hint">加载中…</div>}>
                      <Show when={databasesOf(conn).length > 0} fallback={<div class="tree-hint">连接失败</div>}>
                        <Show
                          when={databasesOf(conn).length > 1}
                          fallback={renderTables(conn, databasesOf(conn)[0], 20)}
                        >
                          <For each={databasesOf(conn)}>
                            {(database) => (
                              <div
                                class="tree-group"
                                data-open={openDatabases()[`${conn.id}|${database}`] ? "true" : "false"}
                              >
                                <button
                                  class="tree-row tree-db"
                                  onClick={() => toggleDatabase(conn, database)}
                                  aria-expanded={openDatabases()[`${conn.id}|${database}`] ? "true" : "false"}
                                >
                                  <ChevronRight
                                    size={12}
                                    class="chev"
                                    classList={{ open: openDatabases()[`${conn.id}|${database}`] }}
                                  />
                                  <Database size={13} />
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
            )}
          </For>
          <Show when={props.backendStatus === "online" && connections().length === 0}>
            <div class="tree-row leaf muted">暂无保存的连接</div>
          </Show>
        </div>
      </div>
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

interface DataTableProps {
  columns: Array<{ name: string; type: string; primary?: boolean }>
  rows: (string | number | boolean | null)[][]
  selectedKey: string | null
  onSelect: (key: string) => void
  emptyMessage?: string
}

function DataTable(props: DataTableProps) {
  return (
    <div class="grid" role="table">
      <div class="grid-head" role="row">
        <div class="th num">
          <span class="th-name">#</span>
        </div>
        <For each={props.columns}>
          {(column) => (
            <div class="th">
              <span class="th-name">
                <Show when={column.primary}>
                  <KeyRound size={9} class="key" />
                </Show>
                {column.name}
              </span>
              <span class="th-type">{column.type || "text"}</span>
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
          {(row, rowIndex) => {
            const key = `${rowIndex()}`
            return (
              <div class="grid-row" classList={{ selected: props.selectedKey === key }} onClick={() => props.onSelect(key)}>
                <div class="td num">{rowIndex() + 1}</div>
                <For each={row}>
                  {(cell) => (
                    <div class="td" classList={{ nullable: cell === null }} title={cell === null ? "NULL" : String(cell)}>
                      {cell === null ? "NULL" : cell}
                    </div>
                  )}
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
          <span class="toolbar-sep" />
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

function CommandPalette(props: {
  onClose: () => void
  onCommand: (id: string) => void
  onToast: (message: string) => void
}) {
  const commands = [
    { id: "new-query", label: "新建查询", hint: "⌘N", icon: <Plus size={14} /> },
    { id: "new-connection", label: "新建连接", hint: "⌘⇧N", icon: <Database size={14} /> },
    { id: "format", label: "格式化当前 SQL", hint: "⌘⇧F", icon: <WandSparkles size={14} /> },
    { id: "toggle-theme", label: "切换主题", hint: "⌘⇧L", icon: <Moon size={14} /> },
    { id: "settings", label: "打开设置", hint: "⌘,", icon: <Settings2 size={14} /> },
  ]
  const [query, setQuery] = createSignal("")
  const [index, setIndex] = createSignal(0)
  let input!: HTMLInputElement

  const filtered = createMemo(() => {
    const q = query().toLowerCase().trim()
    if (!q) return commands
    return commands.filter((item) => item.label.toLowerCase().includes(q))
  })

  createEffect(() => {
    query()
    setIndex(0)
  })

  onMount(() => input?.focus())

  function run(id: string) {
    props.onCommand(id)
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
      <div class="palette" role="dialog" aria-modal="true" aria-label="命令面板">
        <div class="palette-input">
          <Search size={16} />
          <input
            ref={input}
            value={query()}
            onInput={(event) => setQuery(event.currentTarget.value)}
            onKeyDown={onKey}
            placeholder="搜索表、连接或命令"
            autocomplete="off"
          />
          <kbd>Esc</kbd>
        </div>
        <div class="palette-list">
          <div class="palette-group">命令</div>
          <For each={filtered()}>
            {(item, i) => (
              <button class="palette-item" classList={{ active: i() === index() }} onMouseEnter={() => setIndex(i())} onClick={() => run(item.id)}>
                {item.icon}
                {item.label}
                <kbd>{item.hint}</kbd>
              </button>
            )}
          </For>
          <Show when={filtered().length === 0}>
            <div class="palette-empty">没有找到匹配命令</div>
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

export default function App() {
  const [theme, setTheme] = createSignal<Theme>((localStorage.getItem("dbx-theme") as Theme) || "light")
  const [maximized, setMaximized] = createSignal(false)
  const [sidebarCollapsed, setSidebarCollapsed] = createSignal(false)
  const [rightCollapsed, setRightCollapsed] = createSignal(false)
  const [activePanel, setActivePanel] = createSignal<Panel>("ai")
  const [settingsOpen, setSettingsOpen] = createSignal(false)
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
    columns: Array<{ name: string; type: string; primary?: boolean }>
    rows: (string | number | boolean | null)[][]
    rowCount: number
    elapsedMs: number
    error: string | null
  }>({ loading: false, columns: [], rows: [], rowCount: 0, elapsedMs: 0, error: null })
  const [selectedKey, setSelectedKey] = createSignal<string | null>(null)
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
  const [commandOpen, setCommandOpen] = createSignal(false)
  const [newConnectionOpen, setNewConnectionOpen] = createSignal(false)
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

  onCleanup(() => {
    if (toastTimer) window.clearTimeout(toastTimer)
  })

  createEffect(() => {
    document.documentElement.dataset.theme = theme()
    localStorage.setItem("dbx-theme", theme())
    const meta = document.querySelector('meta[name="theme-color"]')
    meta?.setAttribute("content", theme() === "dark" ? "#141414" : "#ffffff")
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
    setBrowse((state) => ({ ...state, loading: true, error: null }))
    try {
      const { result, columns, sql } = await browseTable(connection, database, schema, table)
      setQuerySql(`${sql};`)
      setBrowse({
        loading: false,
        columns: columns.map((column) => ({
          name: column.name,
          type: column.data_type,
          primary: column.is_primary_key,
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
    window.api?.onMaximized(setMaximized)
    const onKey = (event: KeyboardEvent) => {
      const mod = event.metaKey || event.ctrlKey
      if (mod && event.key.toLowerCase() === "k") {
        event.preventDefault()
        setCommandOpen((open) => !open)
        return
      }
      if (event.key === "Escape") {
        setCommandOpen(false)
        return
      }
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

  function handleCommand(id: string) {
    if (id === "new-query") openQuery()
    else if (id === "new-connection") setNewConnectionOpen(true)
    else if (id === "format") showToast("SQL 已格式化")
    else if (id === "toggle-theme") toggleTheme()
    else if (id === "settings") setSettingsOpen(true)
  }

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
          onToggle={() => setSidebarCollapsed(true)}
          onNewConnection={() => setNewConnectionOpen(true)}
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
          onToggleRight={() => setRightCollapsed((value) => !value)}
          onSelect={(id) => {
            setSettingsOpen(false)
            setActiveTab(id)
          }}
          onCloseTab={closeTab}
          onNewQuery={openQuery}
          onTheme={toggleTheme}
          onSettings={() => setSettingsOpen((value) => !value)}
          onCommand={() => setCommandOpen(true)}
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
            <SettingsView onClose={() => setSettingsOpen(false)} />
          </Show>
        </div>
      </div>
      <Show when={commandOpen()}>
        <CommandPalette onClose={() => setCommandOpen(false)} onCommand={handleCommand} onToast={showToast} />
      </Show>
      <Show when={newConnectionOpen()}>
        <NewConnectionDialog
          onClose={() => setNewConnectionOpen(false)}
          onCreated={handleConnectionCreated}
          onToast={showToast}
        />
      </Show>
      <Toast message={toast()} />
    </div>
  )
}