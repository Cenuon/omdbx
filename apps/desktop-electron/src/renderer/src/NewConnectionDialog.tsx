import { createMemo, createSignal, For, Show } from "solid-js"
import { ArrowLeft, ChevronDown, ChevronRight, Database, Loader2, Plug, Search, X } from "lucide-solid"
import {
  databaseCategories,
  databaseTypes,
  type DatabaseCategory,
  type DatabaseTypeInfo,
} from "./dbx/databaseTypes"
import { saveConnection, testConnection, type DbxConnectionInput } from "./dbx/client"

export default function NewConnectionDialog(props: {
  onClose: () => void
  onCreated: (connection: DbxConnectionInput) => void
  onToast: (message: string) => void
}) {
  const id = crypto.randomUUID()
  const [step, setStep] = createSignal<"pick" | "form">("pick")
  const [category, setCategory] = createSignal<DatabaseCategory>("sql")
  const [keyword, setKeyword] = createSignal("")
  const [selected, setSelected] = createSignal<DatabaseTypeInfo>(databaseTypes[0])

  const [name, setName] = createSignal("")
  const [host, setHost] = createSignal("")
  const [port, setPort] = createSignal(0)
  const [username, setUsername] = createSignal("")
  const [password, setPassword] = createSignal("")
  const [database, setDatabase] = createSignal("")
  const [busy, setBusy] = createSignal<"test" | "save" | null>(null)
  const [result, setResult] = createSignal<{ ok: boolean; message: string } | null>(null)

  const isFile = () => selected().localFile

  const visible = createMemo(() => {
    const needle = keyword().trim().toLowerCase()
    if (needle) {
      return databaseTypes.filter(
        (type) => type.label.toLowerCase().includes(needle) || type.key.toLowerCase().includes(needle),
      )
    }
    return databaseTypes.filter((type) => type.category === category())
  })

  function openForm(type: DatabaseTypeInfo) {
    setSelected(type)
    setPort(type.defaultPort)
    setUsername(type.defaultUser)
    setResult(null)
    setStep("form")
  }

  function config(): DbxConnectionInput {
    const payload: DbxConnectionInput = {
      id,
      name: name().trim(),
      db_type: selected().dbType,
      host: host().trim(),
      port: isFile() ? 0 : port(),
      username: isFile() ? "" : username().trim(),
      password: isFile() ? "" : password(),
    }
    if (database().trim()) payload.database = database().trim()
    return payload
  }

  function validate(): string | null {
    if (!name().trim()) return "请填写连接名称"
    if (!host().trim()) return isFile() ? "请填写数据库文件路径" : "请填写主机地址"
    return null
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
      props.onCreated(payload)
      props.onClose()
    } catch (error) {
      setResult({ ok: false, message: error instanceof Error ? error.message : String(error) })
      setBusy(null)
    }
  }

  return (
    <div
      class="palette-backdrop"
      onClick={(event) => event.target === event.currentTarget && props.onClose()}
    >
      <div
        class="dialog dialog-wide"
        role="dialog"
        aria-modal="true"
        aria-label="新建连接"
        onKeyDown={(event) => {
          if (event.key === "Escape") props.onClose()
        }}
      >
        <header class="dialog-head">
          <Show when={step() === "form"}>
            <button class="icon-btn" aria-label="上一步" title="上一步" onClick={() => setStep("pick")}>
              <ArrowLeft size={15} />
            </button>
          </Show>
          <h2>{step() === "pick" ? "新建连接" : selected().label}</h2>
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
          <div class="dialog-body">
            <label class="field">
              <span>名称</span>
              <input
                value={name()}
                onInput={(event) => setName(event.currentTarget.value)}
                placeholder={`例如 本地 ${selected().label}`}
                autofocus
              />
            </label>

            <Show
              when={isFile()}
              fallback={
                <>
                  <div class="field-row">
                    <label class="field">
                      <span>主机</span>
                      <input
                        value={host()}
                        onInput={(event) => setHost(event.currentTarget.value)}
                        placeholder="127.0.0.1"
                      />
                    </label>
                    <label class="field field-narrow">
                      <span>端口</span>
                      <input
                        type="number"
                        value={port()}
                        onInput={(event) => setPort(Number(event.currentTarget.value) || 0)}
                      />
                    </label>
                  </div>
                  <div class="field-row">
                    <label class="field">
                      <span>用户名</span>
                      <input value={username()} onInput={(event) => setUsername(event.currentTarget.value)} />
                    </label>
                    <label class="field">
                      <span>密码</span>
                      <input
                        type="password"
                        value={password()}
                        onInput={(event) => setPassword(event.currentTarget.value)}
                      />
                    </label>
                  </div>
                </>
              }
            >
              <label class="field">
                <span>数据库文件</span>
                <input
                  value={host()}
                  onInput={(event) => setHost(event.currentTarget.value)}
                  placeholder="/path/to/database"
                />
              </label>
            </Show>

            <label class="field">
              <span>数据库</span>
              <input
                value={database()}
                onInput={(event) => setDatabase(event.currentTarget.value)}
                placeholder={isFile() ? "留空即可" : "可选"}
              />
            </label>

            <Show when={result()}>
              {(state) => (
                <div class="dialog-result" classList={{ ok: state().ok }}>
                  {state().message}
                </div>
              )}
            </Show>
          </div>

          <footer class="dialog-foot">
            <button class="ghost-btn" onClick={runTest} disabled={busy() !== null}>
              <Show when={busy() === "test"} fallback={<Plug size={13} />}>
                <Loader2 size={13} class="spin" />
              </Show>
              测试连接
            </button>
            <div class="dialog-foot-right">
              <button class="ghost-btn" onClick={() => setStep("pick")}>
                上一步
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
