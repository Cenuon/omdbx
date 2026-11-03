import { createMemo, createSignal, For, Show } from "solid-js"
import { ArrowLeft, ChevronDown, ChevronRight, Database, Loader2, Plug, Search, X } from "lucide-solid"
import {
  databaseCategories,
  databaseTypes,
  type DatabaseCategory,
  type DatabaseTypeInfo,
} from "./dbx/databaseTypes"
import { connectionDefaults, connectionFields, fieldRows, type FieldSpec, type Values } from "./dbx/connectionFields"
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
  const [values, setValues] = createSignal<Values>({})
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

  function openForm(type: DatabaseTypeInfo) {
    setSelected(type)
    setValues(connectionDefaults(type))
    setResult(null)
    setStep("form")
  }

  function setValue(key: string, value: string | number | boolean) {
    setValues((state) => ({ ...state, [key]: value }))
  }

  function config(): DbxConnectionInput {
    const payload: DbxConnectionInput = {
      id,
      name: name().trim(),
      db_type: selected().dbType,
      host: String(values().host ?? ""),
      port: Number(values().port) || 0,
      username: String(values().username ?? ""),
      password: String(values().password ?? ""),
    }
    for (const field of shown()) {
      if (["host", "port", "username", "password"].includes(field.key)) continue
      const value = values()[field.key]
      if (value === undefined || value === "" || value === false) continue
      payload[field.key] = value
    }
    return payload
  }

  function validate(): string | null {
    if (!name().trim()) return "请填写连接名称"
    const host = shown().find((field) => field.key === "host")
    if (host && !String(values().host ?? "").trim()) {
      return selected().localFile ? "请填写数据库文件路径" : "请填写主机地址"
    }
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

  function renderField(field: FieldSpec) {
    const value = () => values()[field.key]
    const narrow = field.kind === "number"
    return (
      <label class="field" classList={{ "field-narrow": narrow }}>
        <span>{field.label}</span>
        <Show when={field.kind === "select"}>
          <span class="select-wrap">
            <select value={String(value() ?? "")} onChange={(event) => setValue(field.key, event.currentTarget.value)}>
              <For each={field.options ?? []}>{(option) => <option value={option.value}>{option.label}</option>}</For>
            </select>
            <ChevronDown size={13} />
          </span>
        </Show>
        <Show when={field.kind === "textarea"}>
          <textarea
            rows="3"
            value={String(value() ?? "")}
            placeholder={field.placeholder}
            onInput={(event) => setValue(field.key, event.currentTarget.value)}
          />
        </Show>
        <Show when={field.kind === "checkbox"}>
          <input
            type="checkbox"
            class="field-check"
            checked={!!value()}
            onChange={(event) => setValue(field.key, event.currentTarget.checked)}
          />
        </Show>
        <Show when={field.kind === "text" || field.kind === "password"}>
          <input
            type={field.kind === "password" ? "password" : "text"}
            value={String(value() ?? "")}
            placeholder={field.placeholder}
            onInput={(event) => setValue(field.key, event.currentTarget.value)}
          />
        </Show>
        <Show when={field.kind === "number"}>
          <input
            type="number"
            value={Number(value() ?? 0)}
            onInput={(event) => setValue(field.key, Number(event.currentTarget.value) || 0)}
          />
        </Show>
        <Show when={field.hint && field.kind !== "checkbox"}>
          <small class="field-hint">{field.hint}</small>
        </Show>
      </label>
    )
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

            <For each={rows()}>
              {(row) => (
                <Show when={row.length > 1} fallback={renderField(row[0])}>
                  <div class="field-row">
                    <For each={row}>{(field) => renderField(field)}</For>
                  </div>
                </Show>
              )}
            </For>

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
