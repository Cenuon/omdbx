export interface DbxConnection {
  id: string
  name: string
  db_type: string
  host?: string
  port?: number
  database?: string
  default_schema?: string
  connected?: boolean
  /** hex colour used to tint the connection in the sidebar */
  color?: string | null
}

export interface DbxTable {
  name: string
  table_type?: string
  comment?: string | null
}

export interface DbxColumn {
  name: string
  data_type: string
  is_nullable?: boolean
  is_primary_key?: boolean
}

export interface DbxQueryResult {
  columns: string[]
  column_types?: string[]
  rows: (string | number | boolean | null)[][]
  affected_rows: number
  execution_time_ms: number
  truncated?: boolean
  has_more?: boolean
  error?: { message?: string }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    credentials: "include",
    headers: {
      "Content-Type": "application/json",
      ...(init?.headers ?? {}),
    },
    ...init,
  })
  const text = await response.text()
  if (!response.ok) {
    let message = `${response.status} ${response.statusText}`
    try {
      const body = JSON.parse(text) as { detail?: string; message?: string }
      message = body.detail || body.message || message
    } catch {
      if (text) message = text.slice(0, 300)
    }
    throw new Error(message)
  }
  if (!text) return null as T
  return JSON.parse(text) as T
}

export function getConnections(): Promise<DbxConnection[]> {
  return request<DbxConnection[]>("/api/connection/list")
}

/** Payload accepted by /connection/test and /connection/save. */
export interface DbxConnectionInput {
  id: string
  name: string
  db_type: string
  host: string
  port: number
  username: string
  password: string
  database?: string
  /** Per-type ConnectionConfig fields (url_params, redis_*, ssl, …). */
  [key: string]: unknown
}

export function testConnection(config: DbxConnectionInput): Promise<string> {
  return request<string>("/api/connection/test", {
    method: "POST",
    body: JSON.stringify({ config }),
  })
}

export function saveConnection(config: DbxConnectionInput): Promise<void> {
  return saveConnections([config], [])
}

export function saveConnections(configs: unknown[], removedIds: string[]): Promise<void> {
  return request<void>("/api/connection/save", {
    method: "POST",
    body: JSON.stringify({ configs, removedIds }),
  })
}

export function disconnectDb(connectionId: string): Promise<void> {
  return request<void>("/api/connection/disconnect", {
    method: "POST",
    body: JSON.stringify({ connectionId }),
  })
}

/** Free-form JSON blob the backend stores in a single sidebar_layout row. */
export function saveSidebarLayout(layout: unknown): Promise<void> {
  return request<void>("/api/layout/sidebar", {
    method: "POST",
    body: JSON.stringify({ layout }),
  })
}

export function loadSidebarLayout(): Promise<unknown> {
  return request<unknown>("/api/layout/sidebar")
}

export function connectDb(config: DbxConnection): Promise<string> {
  return request<string>("/api/connection/connect", {
    method: "POST",
    body: JSON.stringify({ config }),
  })
}

export function getDatabases(connectionId: string): Promise<Array<{ name: string }>> {
  return request(`/api/schema/databases?connection_id=${encodeURIComponent(connectionId)}`)
}

export function getSchemas(connectionId: string, database: string): Promise<string[]> {
  const query = new URLSearchParams({ connection_id: connectionId, database })
  return request(`/api/schema/schemas?${query}`)
}

export function getTables(connectionId: string, database: string, schema: string): Promise<DbxTable[]> {
  const query = new URLSearchParams({ connection_id: connectionId, database, schema })
  return request(`/api/schema/tables?${query}`)
}

export function getColumns(connectionId: string, database: string, schema: string, table: string): Promise<DbxColumn[]> {
  const query = new URLSearchParams({ connection_id: connectionId, database, schema, table })
  return request(`/api/schema/columns?${query}`)
}

export function executeQuery(
  connectionId: string,
  database: string,
  sql: string,
  schema?: string,
): Promise<DbxQueryResult> {
  return request<DbxQueryResult>("/api/query/execute", {
    method: "POST",
    body: JSON.stringify({ connectionId, database, sql, schema, maxRows: 1000 }),
  })
}

export function getIdentifierQuote(connectionId: string, database?: string): Promise<string | null> {
  return request<string | null>("/api/connection/identifier-quote", {
    method: "POST",
    body: JSON.stringify({ connectionId, database: database || null }),
  })
}

/** Table DDL as the server would emit it. */
export function getTableDdl(
  connectionId: string,
  database: string,
  schema: string,
  table: string,
): Promise<string> {
  const params = new URLSearchParams({ connection_id: connectionId, database, schema, table })
  return request<string>(`/api/schema/ddl?${params.toString()}`)
}

export interface TableImportPreview {
  fileName: string
  filePath: string
  sourceRef?: string
  fileType: string
  sizeBytes: number
  columns: string[]
  rows: Array<Array<string | number | boolean | null>>
  totalRows: number
  totalRowsExact: boolean
}

/** Uploads the file and returns the server-side preview + temp path. */
export async function previewTableImport(
  file: File,
  connectionId: string,
  database: string,
): Promise<TableImportPreview> {
  const form = new FormData()
  form.append("file", file)
  form.append("connectionId", connectionId)
  form.append("database", database)
  form.append("previewLimit", "50")
  const response = await fetch("/api/import/preview", { method: "POST", body: form, credentials: "include" })
  const text = await response.text()
  if (!response.ok) {
    let message = `${response.status} ${response.statusText}`
    try {
      const body = JSON.parse(text) as { detail?: string; message?: string }
      message = body.detail || body.message || message
    } catch {
      if (text) message = text.slice(0, 300)
    }
    throw new Error(message)
  }
  return JSON.parse(text) as TableImportPreview
}

export interface TableImportRequest {
  importId: string
  connectionId: string
  database: string
  schema: string
  table: string
  filePath: string
  sourceRef?: string
  mappings: Array<{ sourceColumn: string; targetColumn: string }>
  mode: "append" | "truncate"
  createTable: boolean
  batchSize: number
  conflictPolicy: "error" | "skip" | "updateExisting"
  retainSource: boolean
}

/** Starts the import; progress streams from {@link tableImportProgressUrl}. */
export async function executeTableImport(payload: TableImportRequest): Promise<string> {
  const data = await request<{ importId: string }>("/api/import/execute", {
    method: "POST",
    body: JSON.stringify({ request: payload }),
  })
  return data.importId
}

export function tableImportProgressUrl(importId: string): string {
  return `/api/import/progress/${encodeURIComponent(importId)}`
}

export function cancelTableImport(importId: string): Promise<unknown> {
  return request("/api/import/cancel", { method: "POST", body: JSON.stringify({ importId }) })
}

export interface DataGridSaveOptions {
  databaseType?: string
  identifierQuote?: string
  tableMeta: {
    database?: string
    schema?: string
    tableName: string
    primaryKeys: string[]
  }
  columns: string[]
  rows: (string | number | boolean | null)[][]
  /** [row index, [[column index, new value]]] */
  dirtyRows: Array<[number, Array<[number, string | number | null]>]>
}

export interface DataGridSavePreparation {
  validationError?: string
  statements: string[]
  rollbackStatements: string[]
  executionSchema?: string
  keylessGuards?: Array<{ sql: string; maxMatchedRows: number; minMatchedRows: number; message: string }>
}

/** The backend builds the UPDATE statements so quoting and escaping stay dialect-correct. */
export function prepareDataGridSave(options: DataGridSaveOptions): Promise<DataGridSavePreparation> {
  return request<DataGridSavePreparation>("/api/query/prepare-data-grid-save", {
    method: "POST",
    body: JSON.stringify({ options }),
  })
}

/**
 * The backend only reports the quote for a few drivers, so fall back to the
 * dialect convention: backticks for MySQL, brackets for SQL Server, quotes elsewhere.
 */
export function quoteIdentifier(dbType: string, remote: string | null, name: string): string {
  if (dbType === "sqlserver") return `[${name.replace(/]/g, "]]")}]`
  const quote = remote ?? (dbType === "mysql" ? "`" : '"')
  return `${quote}${name.split(quote).join(quote + quote)}${quote}`
}

export async function browseTable(
  connection: DbxConnection,
  database: string,
  schema: string,
  table: string,
): Promise<{ result: DbxQueryResult; columns: DbxColumn[]; sql: string; quote: string }> {
  const remote = await getIdentifierQuote(connection.id, database).catch(() => null)
  const quote = remote ?? (connection.db_type === "mysql" ? "`" : '"')
  const sql = `SELECT * FROM ${quoteIdentifier(connection.db_type, remote, table)} LIMIT 500`
  const [result, columns] = await Promise.all([
    executeQuery(connection.id, database, sql, schema),
    getColumns(connection.id, database, schema, table).catch(() => [] as DbxColumn[]),
  ])
  return { result, columns, sql, quote }
}
