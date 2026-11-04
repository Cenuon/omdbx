import { createMemo, createSignal, For, Show } from "solid-js"
import { Loader2, X } from "lucide-solid"

export interface SqlActionInput {
  key: string
  label: string
  placeholder?: string
  initial?: string
}

/**
 * Runs one SQL statement against a table. Either `build` (with `inputs`) or a
 * fixed `sql` is used; the statement is always shown before it runs.
 */
export default function TableActionDialog(props: {
  title: string
  hint?: string
  danger?: boolean
  confirmLabel?: string
  inputs?: SqlActionInput[]
  build?: (values: Record<string, string>) => string
  sql?: string
  editable?: boolean
  onClose: () => void
  onRun: (sql: string) => Promise<string>
}) {
  const [values, setValues] = createSignal<Record<string, string>>(
    Object.fromEntries((props.inputs ?? []).map((input) => [input.key, input.initial ?? ""])),
  )
  const [sql, setSql] = createSignal(props.sql ?? "")
  const [busy, setBusy] = createSignal(false)
  const [error, setError] = createSignal<string | null>(null)

  const statement = createMemo(() => (props.build ? props.build(values()) : sql()))
  const ready = createMemo(() => statement().trim().length > 0)

  function setValue(key: string, value: string) {
    setValues((state) => ({ ...state, [key]: value }))
  }

  async function run() {
    if (!ready() || busy()) return
    setBusy(true)
    setError(null)
    try {
      await props.onRun(statement())
      props.onClose()
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : String(failure))
      setBusy(false)
    }
  }

  return (
    <div class="palette-backdrop" onClick={(event) => event.target === event.currentTarget && props.onClose()}>
      <div
        class="dialog dialog-form"
        role="dialog"
        aria-modal="true"
        aria-label={props.title}
        onKeyDown={(event) => {
          if (event.key === "Escape") props.onClose()
        }}
      >
        <header class="dialog-head">
          <h2>{props.title}</h2>
          <button class="icon-btn" aria-label="关闭" title="关闭" onClick={props.onClose}>
            <X size={15} />
          </button>
        </header>

        <div class="dialog-body conn-form">
          <Show when={props.hint}>
            <p class="dialog-note">{props.hint}</p>
          </Show>

          <For each={props.inputs ?? []}>
            {(input) => (
              <div class="conn-row">
                <span class="conn-label">{input.label}</span>
                <div class="conn-controls">
                  <input
                    value={values()[input.key] ?? ""}
                    placeholder={input.placeholder}
                    autofocus
                    onInput={(event) => setValue(input.key, event.currentTarget.value)}
                  />
                </div>
              </div>
            )}
          </For>

          <div class="conn-row">
            <span class="conn-label">SQL</span>
            <div class="conn-controls">
              <Show
                when={props.editable}
                fallback={<pre class="sql-preview">{statement()}</pre>}
              >
                <textarea
                  rows="10"
                  class="sql-editor"
                  value={sql()}
                  onInput={(event) => setSql(event.currentTarget.value)}
                />
              </Show>
            </div>
          </div>

          <Show when={error()}>
            <div class="dialog-result">{error()}</div>
          </Show>
        </div>

        <footer class="dialog-foot">
          <div class="dialog-foot-right">
            <button class="ghost-btn" onClick={props.onClose} disabled={busy()}>
              取消
            </button>
            <button
              class="primary-btn"
              classList={{ danger: props.danger }}
              onClick={run}
              disabled={busy() || !ready()}
            >
              <Show when={busy()}>
                <Loader2 size={13} class="spin" />
              </Show>
              {props.confirmLabel ?? "执行"}
            </button>
          </div>
        </footer>
      </div>
    </div>
  )
}
