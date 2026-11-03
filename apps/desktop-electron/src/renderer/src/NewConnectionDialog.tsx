import { createSignal, For, Show } from "solid-js"
import { ChevronDown, Loader2, Plug, X } from "lucide-solid"
import { databaseTypes, databaseTypeByKey } from "./dbx/databaseTypes"
import { saveConnection, testConnection, type DbxConnectionInput } from "./dbx/client"

export default function NewConnectionDialog(props: {
  onClose: () => void
  onCreated: (connection: DbxConnectionInput) => void
  onToast: (message: string) => void
}) {
  const id = crypto.randomUUID()
  const [name, setName] = createSignal("")
  const [dbType, setDbType] = createSignal("mysql")
  const [host, setHost] = createSignal("")
  const [port, setPort] = createSignal(3306)
  const [username, setUsername] = createSignal("")
  const [password, setPassword] = createSignal("")
  const [database, setDatabase] = createSignal("")
  const [busy, setBusy] = createSignal<"test" | "save" | null>(null)
  const [result, setResult] = createSignal<{ ok: boolean; message: string } | null>(null)

  const info = () => databaseTypeByKey.get(dbType())
  const isFile = () => info()?.localFile === true

  function pickType(next: string) {
    setDbType(next)
    const fallback = databaseTypeByKey.get(next)?.defaultPort
    if (fallback) setPort(fallback)
    setResult(null)
  }

  function config(): DbxConnectionInput {
    const payload: DbxConnectionInput = {
      id,
      name: name().trim(),
      db_type: dbType(),
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
        class="dialog"
        role="dialog"
        aria-modal="true"
        aria-label="新建连接"
        onKeyDown={(event) => {
          if (event.key === "Escape") props.onClose()
        }}
      >
        <header class="dialog-head">
          <h2>新建连接</h2>
          <button class="icon-btn" aria-label="关闭" title="关闭" onClick={props.onClose}>
            <X size={15} />
          </button>
        </header>

        <div class="dialog-body">
          <label class="field">
            <span>名称</span>
            <input
              value={name()}
              onInput={(event) => setName(event.currentTarget.value)}
              placeholder="例如 本地 MySQL"
              autofocus
            />
          </label>

          <label class="field">
            <span>类型</span>
            <div class="select-wrap">
              <select value={dbType()} onChange={(event) => pickType(event.currentTarget.value)}>
                <For each={databaseTypes}>
                  {(type) => <option value={type.dbType}>{type.label}</option>}
                </For>
              </select>
              <ChevronDown size={13} />
            </div>
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
                placeholder="/path/to/database.sqlite"
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
            <button class="ghost-btn" onClick={props.onClose}>
              取消
            </button>
            <button class="primary-btn" onClick={runSave} disabled={busy() !== null}>
              <Show when={busy() === "save"}>
                <Loader2 size={13} class="spin" />
              </Show>
              保存并连接
            </button>
          </div>
        </footer>
      </div>
    </div>
  )
}
