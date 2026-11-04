import { createMemo, createSignal, For, Show } from "solid-js"
import { ChevronDown, FileSpreadsheet, Loader2, Upload, X } from "lucide-solid"
import {
  cancelTableImport,
  executeTableImport,
  previewTableImport,
  tableImportProgressUrl,
  type DbxConnection,
  type TableImportPreview,
} from "./dbx/client"

interface ImportProgress {
  status: "running" | "done" | "error" | "cancelled"
  rowsImported: number
  totalRows: number
  error?: string | null
  rowsFailed?: number
}

function baseName(fileName: string): string {
  // the server stores uploads as "<uuid>-<original name>"
  const stem = fileName
    .replace(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}-/i, "")
    .replace(/\.[^.]+$/, "")
  return stem.replace(/[^A-Za-z0-9_]/g, "_") || "imported_table"
}

export default function ImportWizard(props: {
  conn: DbxConnection
  database: string
  schema: string
  table: string
  onClose: () => void
  onToast: (message: string) => void
  onDone: () => void
}) {
  const [file, setFile] = createSignal<File | null>(null)
  const [preview, setPreview] = createSignal<TableImportPreview | null>(null)
  const [loading, setLoading] = createSignal(false)
  const [target, setTarget] = createSignal(props.table)
  const [mode, setMode] = createSignal<"append" | "truncate">("append")
  const [createTable, setCreateTable] = createSignal(false)
  const [conflict, setConflict] = createSignal<"error" | "skip" | "updateExisting">("error")
  const [batchSize, setBatchSize] = createSignal(500)
  const [mappings, setMappings] = createSignal<Record<string, string>>({})
  const [progress, setProgress] = createSignal<ImportProgress | null>(null)
  const [importId, setImportId] = createSignal("")
  const [error, setError] = createSignal<string | null>(null)

  const running = createMemo(() => progress()?.status === "running")
  const percent = createMemo(() => {
    const state = progress()
    if (!state || !state.totalRows) return 0
    return Math.min(100, Math.round((state.rowsImported / state.totalRows) * 100))
  })

  async function pickFile(selected: File | null) {
    setFile(selected)
    setPreview(null)
    setError(null)
    if (!selected) return
    setLoading(true)
    try {
      const result = await previewTableImport(selected, props.conn.id, props.database)
      setPreview(result)
      setTarget(baseName(result.fileName))
      setMappings(Object.fromEntries(result.columns.map((column) => [column, column])))
    } catch (failure) {
      setError(failure instanceof Error ? failure.message : String(failure))
    } finally {
      setLoading(false)
    }
  }

  function setMapping(source: string, value: string) {
    setMappings((state) => ({ ...state, [source]: value }))
  }

  async function start() {
    const source = preview()
    if (!source) return
    const list = source.columns
      .map((column) => ({ sourceColumn: column, targetColumn: (mappings()[column] ?? column).trim() }))
      .filter((mapping) => mapping.targetColumn.length > 0)
    if (list.length === 0) return setError("至少映射一个字段")
    if (!target().trim()) return setError("请填写目标表名")

    setError(null)
    setProgress({ status: "running", rowsImported: 0, totalRows: source.totalRows })
    try {
      const id = await executeTableImport({
        importId: crypto.randomUUID(),
        connectionId: props.conn.id,
        database: props.database,
        schema: props.schema,
        table: target().trim(),
        filePath: source.filePath,
        sourceRef: source.sourceRef,
        mappings: list,
        mode: mode(),
        createTable: createTable(),
        batchSize: batchSize(),
        conflictPolicy: conflict(),
        retainSource: true,
      })
      setImportId(id)
      const stream = new EventSource(tableImportProgressUrl(id))
      stream.onmessage = (event) => {
        const state = JSON.parse(event.data) as ImportProgress
        setProgress(state)
        if (state.status === "running") return
        stream.close()
        if (state.status === "done") {
          props.onToast(`已导入 ${state.rowsImported} 行到「${target().trim()}」`)
          props.onDone()
        } else if (state.status === "error") {
          setError(state.error || "导入失败")
        }
      }
      stream.onerror = () => {
        stream.close()
        if (progress()?.status === "running") setError("进度连接中断")
      }
    } catch (failure) {
      setProgress(null)
      setError(failure instanceof Error ? failure.message : String(failure))
    }
  }

  async function stop() {
    const id = importId()
    if (id) await cancelTableImport(id).catch(() => undefined)
  }

  return (
    <div class="palette-backdrop" onClick={(event) => event.target === event.currentTarget && !running() && props.onClose()}>
      <div
        class="dialog dialog-form"
        role="dialog"
        aria-modal="true"
        aria-label="导入向导"
        onKeyDown={(event) => {
          if (event.key === "Escape" && !running()) props.onClose()
        }}
      >
        <header class="dialog-head">
          <h2>导入向导</h2>
          <button class="icon-btn" aria-label="关闭" title="关闭" onClick={props.onClose} disabled={running()}>
            <X size={15} />
          </button>
        </header>

        <div class="dialog-body conn-form">
          <Show when={!progress()}>
            <div class="conn-row">
              <span class="conn-label">源文件</span>
              <div class="conn-controls">
                <label class="file-pick">
                  <Upload size={14} />
                  <span>{file()?.name ?? "选择 CSV / JSON / Excel 文件"}</span>
                  <input
                    type="file"
                    accept=".csv,.tsv,.txt,.json,.xlsx,.xls,.parquet,.sql"
                    onChange={(event) => void pickFile(event.currentTarget.files?.[0] ?? null)}
                  />
                </label>
                <Show when={loading()}>
                  <Loader2 size={14} class="spin" />
                </Show>
              </div>
            </div>

            <Show when={preview()}>
              {(source) => (
                <>
                  <div class="conn-row">
                    <span class="conn-label">预览</span>
                    <div class="conn-controls">
                      <div class="info-row">
                        <FileSpreadsheet size={15} />
                        {source().fileType.toUpperCase()} · {source().columns.length} 列 ·{" "}
                        {source().totalRows}
                        {source().totalRowsExact ? "" : "+"} 行 · {Math.max(1, Math.round(source().sizeBytes / 1024))} KB
                      </div>
                    </div>
                  </div>

                  <div class="conn-row">
                    <span class="conn-label">目标表</span>
                    <div class="conn-controls">
                      <input
                        value={target()}
                        onInput={(event) => setTarget(event.currentTarget.value)}
                        placeholder="目标表名"
                      />
                    </div>
                  </div>

                  <div class="conn-row">
                    <span class="conn-label">导入方式</span>
                    <div class="conn-controls">
                      <span class="select-wrap">
                        <select
                          value={mode()}
                          onChange={(event) => setMode(event.currentTarget.value as "append" | "truncate")}
                        >
                          <option value="append">追加到现有数据</option>
                          <option value="truncate">先清空表再导入</option>
                        </select>
                        <ChevronDown size={13} />
                      </span>
                      <span class="select-wrap">
                        <select
                          value={conflict()}
                          onChange={(event) =>
                            setConflict(event.currentTarget.value as "error" | "skip" | "updateExisting")
                          }
                        >
                          <option value="error">主键冲突：报错</option>
                          <option value="skip">主键冲突：跳过</option>
                          <option value="updateExisting">主键冲突：更新</option>
                        </select>
                        <ChevronDown size={13} />
                      </span>
                    </div>
                  </div>

                  <div class="conn-row">
                    <span class="conn-label">选项</span>
                    <div class="conn-controls">
                      <label class="check-row">
                        <input
                          type="checkbox"
                          class="field-check"
                          checked={createTable()}
                          onChange={(event) => setCreateTable(event.currentTarget.checked)}
                        />
                        表不存在时自动创建
                      </label>
                      <input
                        type="number"
                        class="conn-narrow"
                        value={batchSize()}
                        onInput={(event) => setBatchSize(Number(event.currentTarget.value) || 500)}
                      />
                      <small class="field-hint">每批行数</small>
                    </div>
                  </div>

                  <div class="conn-row">
                    <span class="conn-label">字段映射</span>
                    <div class="conn-controls mapping-list">
                      <For each={source().columns}>
                        {(column) => (
                          <div class="mapping-row">
                            <span class="mapping-source">{column}</span>
                            <span class="mapping-arrow">→</span>
                            <input
                              value={mappings()[column] ?? ""}
                              onInput={(event) => setMapping(column, event.currentTarget.value)}
                              placeholder="目标字段，留空则跳过"
                            />
                          </div>
                        )}
                      </For>
                    </div>
                  </div>
                </>
              )}
            </Show>
          </Show>

          <Show when={progress()}>
            {(state) => (
              <>
                <div class="conn-row">
                  <span class="conn-label">进度</span>
                  <div class="conn-controls">
                    <div class="import-bar">
                      <div class="import-bar-fill" style={{ width: `${percent()}%` }} />
                    </div>
                    <span class="import-count">
                      {state().rowsImported} / {state().totalRows || "?"}
                    </span>
                  </div>
                </div>
                <Show when={state().status === "done"}>
                  <div class="dialog-result ok">
                    导入完成 · {state().rowsImported} 行
                    <Show when={state().rowsFailed}>（失败 {state().rowsFailed} 行）</Show>
                  </div>
                </Show>
              </>
            )}
          </Show>

          <Show when={error()}>
            <div class="dialog-result">{error()}</div>
          </Show>
        </div>

        <footer class="dialog-foot">
          <div class="dialog-foot-right">
            <Show
              when={running()}
              fallback={
                <>
                  <button class="ghost-btn" onClick={props.onClose}>
                    取消
                  </button>
                  <button class="primary-btn" onClick={start} disabled={!preview() || loading()}>
                    <Upload size={13} />
                    开始导入
                  </button>
                </>
              }
            >
              <button class="ghost-btn" onClick={() => void stop()}>
                中止导入
              </button>
            </Show>
          </div>
        </footer>
      </div>
    </div>
  )
}
