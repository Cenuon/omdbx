import { createMemo, createSignal, For, Show } from "solid-js"
import {
  ArrowLeft,
  ChevronDown,
  ChevronRight,
  Database,
  Eye,
  EyeOff,
  Link2,
  Loader2,
  Pencil,
  Plug,
  Search,
  X,
} from "lucide-solid"
import {
  databaseCategories,
  databaseTypes,
  type DatabaseCategory,
  type DatabaseTypeInfo,
} from "./dbx/databaseTypes"
import {
  connectionDefaults,
  connectionFields,
  fieldRows,
  parseConnectionUrl,
  type FieldSpec,
  type Values,
} from "./dbx/connectionFields"
import { loadSidebarLayout, saveConnection, saveSidebarLayout, testConnection, type DbxConnectionInput } from "./dbx/client"

const COLORS = ["", "#22c55e", "#eab308", "#f97316", "#ef4444", "#3b82f6", "#a855f7"]

const TABS = [
  { id: "connection", label: "连接信息" },
  { id: "tls", label: "TLS/SSL" },
  { id: "transport", label: "隧道/代理" },
  { id: "advanced", label: "高级" },
] as const

interface SidebarFolder {
  id: string
  name: string
  connections: string[]
}

export default function NewConnectionDialog(props: {
  onClose: () => void
  onCreated: (connection: DbxConnectionInput) => void
  onToast: (message: string) => void
}) {
  const id = crypto.randomUUID()
  const [step, setStep] = createSignal<"pick" | "form">("pick")
  const [tab, setTab] = createSignal<(typeof TABS)[number]["id"]>("connection")
  const [category, setCategory] = createSignal<DatabaseCategory>("sql")
  const [keyword, setKeyword] = createSignal("")
  const [selected, setSelected] = createSignal<DatabaseTypeInfo>(databaseTypes[0])

  const [name, setName] = createSignal("")
  const [url, setUrl] = createSignal("")
  const [values, setValues] = createSignal<Values>({})
  const [color, setColor] = createSignal("")
  const [note, setNote] = createSignal("")
  const [savePassword, setSavePassword] = createSignal(true)
  const [showPassword, setShowPassword] = createSignal(false)
  const [paramsOpen, setParamsOpen] = createSignal(false)
  const [folders, setFolders] = createSignal<SidebarFolder[]>([])
  const [folderId, setFolderId] = createSignal("")
  const [busy, setBusy] = createSignal<"test" | "save" | null>(null)
  const [result, setResult] = createSignal<{ ok: boolean; message: string } | null>(null)

  const visible = createMemo(() => {
    const needle = keyword().trim().toLowerCase()
    if (needle) {
      return databaseTypes.filter(
        (type) => type.label.toLowerCase().includes(needle) || type.key.toLowerCase().includes(needle),
      )
    }
    return databaseTypes.filter((type) => type.category === category())
  })

  const shown = createMemo(() => {
    const all = connectionFields(selected())
    return all.filter((field) => !field.visibleWhen || field.visibleWhen(values()))
  })
  const rows = createMemo(() => fieldRows(shown()))
  const hasPassword = createMemo(() => shown().some((field) => field.key === "password"))

  function openForm(type: DatabaseTypeInfo) {
    setSelected(type)
    setValues(connectionDefaults(type))
    setTab("connection")
    setUrl("")
    setParamsOpen(false)
    setResult(null)
    setStep("form")
    void loadFolders()
  }

  async function loadFolders() {
    try {
      const raw = (await loadSidebarLayout()) as { omdbx?: { folders?: SidebarFolder[] } } | null
      setFolders(raw?.omdbx?.folders ?? [])
    } catch {
      setFolders([])
    }
  }

  function setValue(key: string, value: string | number | boolean) {
    setValues((state) => ({ ...state, [key]: value }))
  }

  function applyUrl() {
    const parsed = parseConnectionUrl(url())
    if (!parsed) {
      props.onToast("无法解析这个 URL")
      return
    }
    setValues((state) => ({ ...state, ...parsed }))
  }

  /** Values that are UI-only and never belong in the saved config. */
  const UI_KEYS = new Set([
    "ssh_enabled",
    "ssh_host",
    "ssh_port",
    "ssh_user",
    "ssh_password",
    "ssh_key_path",
  ])

  function config(): DbxConnectionInput {
    const current = values()
    const payload: DbxConnectionInput = {
      id,
      name: name().trim() || `${selected().label} 连接`,
      db_type: selected().dbType,
      host: String(current.host ?? ""),
      port: Number(current.port) || 0,
      username: String(current.username ?? ""),
      password: String(current.password ?? ""),
      save_password: savePassword(),
    }
    // every other control on any tab writes its ConnectionConfig key straight through
    for (const [key, value] of Object.entries(current)) {
      if (["host", "port", "username", "password"].includes(key) || UI_KEYS.has(key)) continue
      if (value === undefined || value === "" || value === false) continue
      payload[key] = value
    }
    if (color()) payload.color = color()
    if (note().trim()) payload.note = note().trim()
    if (current.ssh_enabled) {
      payload.transport_layers = [
        {
          type: "ssh",
          id: crypto.randomUUID(),
          name: "SSH",
          enabled: true,
          host: String(current.ssh_host ?? ""),
          port: Number(current.ssh_port) || 22,
          user: String(current.ssh_user ?? ""),
          password: String(current.ssh_password ?? ""),
          key_path: String(current.ssh_key_path ?? ""),
        },
      ]
    }
    return payload
  }

  function validate(): string | null {
    const host = shown().find((field) => field.key === "host")
    if (host && !String(values().host ?? "").trim()) {
      return selected().localFile ? "请填写数据库文件路径" : "请填写主机地址"
    }
    if (values().ssh_enabled && !String(values().ssh_host ?? "").trim()) return "请填写 SSH 主机地址"
    return null
  }

  async function assignFolder(connectionId: string) {
    if (!folderId()) return
    try {
      const raw = (await loadSidebarLayout()) as { omdbx?: { folders?: SidebarFolder[]; stars?: string[] } } | null
      const list = (raw?.omdbx?.folders ?? []).map((folder) => ({
        ...folder,
        connections: folder.connections.filter((item) => item !== connectionId),
      }))
      const target = list.findIndex((folder) => folder.id === folderId())
      if (target >= 0) list[target] = { ...list[target], connections: [...list[target].connections, connectionId] }
      await saveSidebarLayout({ omdbx: { folders: list, stars: raw?.omdbx?.stars ?? [] } })
    } catch {
      // the connection is saved either way; the folder is a sidebar preference
    }
  }

  async function runTest() {
    const invalid = validate()
    if (invalid) return props.onToast(invalid)
    setBusy("test")
    setResult(null)
    try {
      const message = await testConnection(config())
      setResult({ ok: true, message: message || "连接成功" })
    } catch (error) {
      setResult({ ok: false, message: error instanceof Error ? error.message : String(error) })
    } finally {
      setBusy(null)
    }
  }

  async function runSave() {
    const invalid = validate()
    if (invalid) return props.onToast(invalid)
    setBusy("save")
    try {
      const payload = config()
      await saveConnection(payload)
      await assignFolder(id)
      props.onCreated(payload)
      props.onClose()
    } catch (error) {
      setResult({ ok: false, message: error instanceof Error ? error.message : String(error) })
      setBusy(null)
    }
  }

  function renderControl(field: FieldSpec) {
    const value = () => values()[field.key]
    if (field.kind === "select") {
      return (
        <span class="select-wrap">
          <select value={String(value() ?? "")} onChange={(event) => setValue(field.key, event.currentTarget.value)}>
            <For each={field.options ?? []}>{(option) => <option value={option.value}>{option.label}</option>}</For>
          </select>
          <ChevronDown size={13} />
        </span>
      )
    }
    if (field.kind === "textarea") {
      return (
        <textarea
          rows="3"
          value={String(value() ?? "")}
          placeholder={field.placeholder}
          onInput={(event) => setValue(field.key, event.currentTarget.value)}
        />
      )
    }
    if (field.kind === "checkbox") {
      return (
        <label class="check-row">
          <input
            type="checkbox"
            class="field-check"
            checked={!!value()}
            onChange={(event) => setValue(field.key, event.currentTarget.checked)}
          />
          {field.hint ?? field.label}
        </label>
      )
    }
    if (field.kind === "number") {
      return (
        <input
          type="number"
          class="conn-narrow"
          value={Number(value() ?? 0)}
          onInput={(event) => setValue(field.key, Number(event.currentTarget.value) || 0)}
        />
      )
    }
    return (
      <span class="field-input">
        <input
          type={field.kind === "password" && !showPassword() ? "password" : "text"}
          value={String(value() ?? "")}
          placeholder={field.placeholder}
          onInput={(event) => setValue(field.key, event.currentTarget.value)}
        />
        <Show when={field.kind === "password"}>
          <button
            type="button"
            class="field-eye"
            aria-label={showPassword() ? "隐藏密码" : "显示密码"}
            onClick={() => setShowPassword((state) => !state)}
          >
            <Show when={showPassword()} fallback={<Eye size={14} />}>
              <EyeOff size={14} />
            </Show>
          </button>
        </Show>
      </span>
    )
  }

  function renderRow(fields: FieldSpec[]) {
    const first = fields[0]
    return (
      <div class="conn-row">
        <span class="conn-label">{first.label}</span>
        <div class="conn-controls">
          <For each={fields}>{(field) => renderControl(field)}</For>
        </div>
      </div>
    )
  }

  return (
    <div class="palette-backdrop" onClick={(event) => event.target === event.currentTarget && props.onClose()}>
      <div
        class="dialog"
        classList={{ "dialog-wide": step() === "pick", "dialog-form": step() === "form" }}
        role="dialog"
        aria-modal="true"
        aria-label="新建连接"
        onKeyDown={(event) => {
          if (event.key === "Escape") props.onClose()
        }}
      >
        <header class="dialog-head">
          <h2>{step() === "pick" ? "新建连接" : "连接配置"}</h2>
          <button class="icon-btn" aria-label="关闭" title="关闭" onClick={props.onClose}>
            <X size={15} />
          </button>
        </header>

        <Show when={step() === "pick"}>
          <div class="picker-toolbar">
            <Search size={14} />
            <input
              value={keyword()}
              onInput={(event) => setKeyword(event.currentTarget.value)}
              placeholder="搜索数据库类型"
              aria-label="搜索数据库类型"
              autofocus
            />
          </div>
          <div class="picker-body">
            <nav class="picker-rail">
              <For each={databaseCategories}>
                {(item) => (
                  <button
                    class="picker-cat"
                    classList={{ active: !keyword().trim() && category() === item.key }}
                    onClick={() => {
                      setKeyword("")
                      setCategory(item.key)
                    }}
                  >
                    {item.label}
                  </button>
                )}
              </For>
            </nav>
            <div class="picker-grid">
              <For each={visible()}>
                {(type) => (
                  <button
                    class="picker-card"
                    classList={{ active: selected().key === type.key }}
                    onClick={() => setSelected(type)}
                    onDblClick={() => openForm(type)}
                  >
                    <span class="picker-icon">
                      <Show when={type.icon} fallback={<Database size={22} />}>
                        <img src={type.icon} alt="" loading="lazy" />
                      </Show>
                    </span>
                    <span class="picker-label">{type.label}</span>
                  </button>
                )}
              </For>
              <Show when={visible().length === 0}>
                <p class="picker-empty">没有匹配的数据库类型</p>
              </Show>
            </div>
          </div>
          <footer class="dialog-foot">
            <span class="picker-chosen">
              已选择：<strong>{selected().label}</strong>
            </span>
            <div class="dialog-foot-right">
              <button class="primary-btn" onClick={() => openForm(selected())}>
                下一步
                <ChevronRight size={13} />
              </button>
            </div>
          </footer>
        </Show>

        <Show when={step() === "form"}>
          <div class="dialog-tabs" role="tablist">
            <For each={TABS}>
              {(item) => (
                <button
                  class="dialog-tab"
                  classList={{ active: tab() === item.id }}
                  role="tab"
                  aria-selected={tab() === item.id}
                  onClick={() => setTab(item.id)}
                >
                  {item.label}
                </button>
              )}
            </For>
          </div>

          <div class="dialog-body conn-form">
            <Show when={tab() === "connection"}>
              <div class="conn-row">
                <span class="conn-label">URL（可选）</span>
                <div class="conn-controls">
                  <span class="field-input">
                    <input
                      value={url()}
                      onInput={(event) => setUrl(event.currentTarget.value)}
                      placeholder="http://user:password@host:port"
                      autofocus
                    />
                    <button type="button" class="field-eye" aria-label="解析 URL" title="解析 URL" onClick={applyUrl}>
                      <Link2 size={14} />
                    </button>
                  </span>
                </div>
              </div>

              <div class="conn-row">
                <span class="conn-label">名称</span>
                <div class="conn-controls">
                  <input
                    value={name()}
                    onInput={(event) => setName(event.currentTarget.value)}
                    placeholder="连接名称，留空则自动生成"
                  />
                </div>
              </div>

              <div class="conn-row">
                <span class="conn-label">类型</span>
                <div class="conn-controls">
                  <button class="type-row" onClick={() => setStep("pick")}>
                    <Show when={selected().icon} fallback={<Database size={16} />}>
                      <img src={selected().icon} alt="" />
                    </Show>
                    <span>{selected().label}</span>
                    <Pencil size={13} />
                  </button>
                </div>
              </div>

              <div class="conn-row">
                <span class="conn-label">颜色</span>
                <div class="conn-controls color-row">
                  <For each={COLORS}>
                    {(value) => (
                      <button
                        class="color-dot"
                        classList={{ none: value === "", active: color() === value }}
                        style={value ? { background: value } : undefined}
                        aria-label={value || "无颜色"}
                        title={value || "无颜色"}
                        onClick={() => setColor(value)}
                      />
                    )}
                  </For>
                  <Show when={folders().length > 0}>
                    <span class="group-pick">
                      分组
                      <span class="select-wrap">
                        <select value={folderId()} onChange={(event) => setFolderId(event.currentTarget.value)}>
                          <option value="">不分组</option>
                          <For each={folders()}>{(folder) => <option value={folder.id}>{folder.name}</option>}</For>
                        </select>
                        <ChevronDown size={13} />
                      </span>
                    </span>
                  </Show>
                </div>
              </div>

              <For each={rows()}>
                {(row) => (
                  <>
                    {renderRow(row)}
                    <Show when={hasPassword() && row.some((field) => field.key === "password")}>
                      <div class="conn-row">
                        <span class="conn-label" />
                        <div class="conn-controls">
                          <label class="check-row">
                            <input
                              type="checkbox"
                              class="field-check"
                              checked={savePassword()}
                              onChange={(event) => setSavePassword(event.currentTarget.checked)}
                            />
                            保存密码
                          </label>
                        </div>
                      </div>
                    </Show>
                  </>
                )}
              </For>

              <div class="conn-row">
                <span class="conn-label">连接参数</span>
                <div class="conn-controls">
                  <button class="params-row" onClick={() => setParamsOpen((state) => !state)}>
                    <span>{String(values().url_params ?? "").trim() || "默认"}</span>
                    <span class="params-open">配置…</span>
                  </button>
                </div>
              </div>
              <Show when={paramsOpen()}>
                <div class="conn-row">
                  <span class="conn-label" />
                  <div class="conn-controls">
                    <textarea
                      rows="2"
                      value={String(values().url_params ?? "")}
                      placeholder="k=v&k2=v2"
                      onInput={(event) => setValue("url_params", event.currentTarget.value)}
                    />
                  </div>
                </div>
              </Show>

              <div class="conn-row">
                <span class="conn-label">数据库信息</span>
                <div class="conn-controls">
                  <div class="info-row">
                    <Database size={15} />
                    当前配置
                    <strong>{selected().label}</strong>
                  </div>
                </div>
              </div>

              <div class="conn-row">
                <span class="conn-label">备注</span>
                <div class="conn-controls">
                  <input
                    value={note()}
                    onInput={(event) => setNote(event.currentTarget.value)}
                    placeholder="请勿在备注中明文保存密码"
                  />
                </div>
              </div>
            </Show>

            <Show when={tab() === "tls"}>
              <div class="conn-row">
                <span class="conn-label">TLS/SSL</span>
                <div class="conn-controls">
                  <label class="check-row">
                    <input
                      type="checkbox"
                      class="field-check"
                      checked={!!values().ssl}
                      onChange={(event) => setValue("ssl", event.currentTarget.checked)}
                    />
                    启用 TLS/SSL
                  </label>
                </div>
              </div>
              <div class="conn-row">
                <span class="conn-label">CA 证书</span>
                <div class="conn-controls">
                  <input
                    value={String(values().ca_cert_path ?? "")}
                    placeholder="/path/to/ca.pem"
                    onInput={(event) => setValue("ca_cert_path", event.currentTarget.value)}
                  />
                </div>
              </div>
              <div class="conn-row">
                <span class="conn-label">客户端证书</span>
                <div class="conn-controls">
                  <input
                    value={String(values().client_cert_path ?? "")}
                    placeholder="/path/to/client.crt"
                    onInput={(event) => setValue("client_cert_path", event.currentTarget.value)}
                  />
                </div>
              </div>
              <div class="conn-row">
                <span class="conn-label">客户端私钥</span>
                <div class="conn-controls">
                  <input
                    value={String(values().client_key_path ?? "")}
                    placeholder="/path/to/client.key"
                    onInput={(event) => setValue("client_key_path", event.currentTarget.value)}
                  />
                </div>
              </div>
            </Show>

            <Show when={tab() === "transport"}>
              <div class="conn-row">
                <span class="conn-label">SSH 隧道</span>
                <div class="conn-controls">
                  <label class="check-row">
                    <input
                      type="checkbox"
                      class="field-check"
                      checked={!!values().ssh_enabled}
                      onChange={(event) => setValue("ssh_enabled", event.currentTarget.checked)}
                    />
                    通过跳板机连接
                  </label>
                </div>
              </div>
              <div class="conn-row">
                <span class="conn-label">SSH 主机</span>
                <div class="conn-controls">
                  <input
                    value={String(values().ssh_host ?? "")}
                    placeholder="127.0.0.1"
                    onInput={(event) => setValue("ssh_host", event.currentTarget.value)}
                  />
                  <input
                    type="number"
                    class="conn-narrow"
                    value={Number(values().ssh_port ?? 22)}
                    onInput={(event) => setValue("ssh_port", Number(event.currentTarget.value) || 22)}
                  />
                </div>
              </div>
              <div class="conn-row">
                <span class="conn-label">SSH 用户名</span>
                <div class="conn-controls">
                  <input
                    value={String(values().ssh_user ?? "")}
                    onInput={(event) => setValue("ssh_user", event.currentTarget.value)}
                  />
                </div>
              </div>
              <div class="conn-row">
                <span class="conn-label">SSH 密码</span>
                <div class="conn-controls">
                  <input
                    type="password"
                    value={String(values().ssh_password ?? "")}
                    onInput={(event) => setValue("ssh_password", event.currentTarget.value)}
                  />
                </div>
              </div>
              <div class="conn-row">
                <span class="conn-label">私钥路径</span>
                <div class="conn-controls">
                  <input
                    value={String(values().ssh_key_path ?? "")}
                    placeholder="~/.ssh/id_rsa，可选"
                    onInput={(event) => setValue("ssh_key_path", event.currentTarget.value)}
                  />
                </div>
              </div>
            </Show>

            <Show when={tab() === "advanced"}>
              <div class="conn-row">
                <span class="conn-label">连接超时（秒）</span>
                <div class="conn-controls">
                  <input
                    type="number"
                    class="conn-narrow"
                    value={Number(values().connect_timeout_secs ?? 10)}
                    onInput={(event) => setValue("connect_timeout_secs", Number(event.currentTarget.value) || 0)}
                  />
                </div>
              </div>
              <div class="conn-row">
                <span class="conn-label">查询超时（秒）</span>
                <div class="conn-controls">
                  <input
                    type="number"
                    class="conn-narrow"
                    value={Number(values().query_timeout_secs ?? 30)}
                    onInput={(event) => setValue("query_timeout_secs", Number(event.currentTarget.value) || 0)}
                  />
                </div>
              </div>
              <div class="conn-row">
                <span class="conn-label">空闲超时（秒）</span>
                <div class="conn-controls">
                  <input
                    type="number"
                    class="conn-narrow"
                    value={Number(values().idle_timeout_secs ?? 0)}
                    onInput={(event) => setValue("idle_timeout_secs", Number(event.currentTarget.value) || 0)}
                  />
                  <small class="field-hint">0 表示不回收空闲连接</small>
                </div>
              </div>
              <div class="conn-row">
                <span class="conn-label">只读连接</span>
                <div class="conn-controls">
                  <label class="check-row">
                    <input
                      type="checkbox"
                      class="field-check"
                      checked={!!values().read_only}
                      onChange={(event) => setValue("read_only", event.currentTarget.checked)}
                    />
                    阻止写入操作
                  </label>
                </div>
              </div>
              <div class="conn-row">
                <span class="conn-label">系统 schema</span>
                <div class="conn-controls">
                  <label class="check-row">
                    <input
                      type="checkbox"
                      class="field-check"
                      checked={!!values().show_system_schemas}
                      onChange={(event) => setValue("show_system_schemas", event.currentTarget.checked)}
                    />
                    在侧栏列出系统 schema
                  </label>
                </div>
              </div>
              <div class="conn-row">
                <span class="conn-label">表加载</span>
                <div class="conn-controls">
                  <label class="check-row">
                    <input
                      type="checkbox"
                      class="field-check"
                      checked={!!values().sidebar_auto_load_all_tables}
                      onChange={(event) => setValue("sidebar_auto_load_all_tables", event.currentTarget.checked)}
                    />
                    展开时自动加载全部表
                  </label>
                </div>
              </div>
            </Show>

            <Show when={result()}>
              {(state) => (
                <div class="dialog-result" classList={{ ok: state().ok }}>
                  {state().message}
                </div>
              )}
            </Show>
          </div>

          <footer class="dialog-foot">
            <button class="ghost-btn" onClick={() => setStep("pick")}>
              <ArrowLeft size={13} />
              上一步
            </button>
            <div class="dialog-foot-right">
              <button class="ghost-btn" onClick={runTest} disabled={busy() !== null}>
                <Show when={busy() === "test"} fallback={<Plug size={13} />}>
                  <Loader2 size={13} class="spin" />
                </Show>
                测试
              </button>
              <button class="primary-btn" onClick={runSave} disabled={busy() !== null}>
                <Show when={busy() === "save"}>
                  <Loader2 size={13} class="spin" />
                </Show>
                保存并连接
              </button>
            </div>
          </footer>
        </Show>
      </div>
    </div>
  )
}
