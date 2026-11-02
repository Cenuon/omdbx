export interface DbxConnection {
  id: string
  name: string
  db_type: string
  host?: string
  port?: number
  database?: string
  default_schema?: string
  connected?: boolean
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

export async function browseTable(
  connectionId: string,
  database: string,
  schema: string,
  table: string,
): Promise<{ result: DbxQueryResult; columns: DbxColumn[] }> {
  const quoted = `"${table.replace(/"/g, '""')}"`
  const [result, columns] = await Promise.all([
    executeQuery(connectionId, database, `SELECT * FROM ${quoted} LIMIT 500`, schema),
    getColumns(connectionId, database, schema, table).catch(() => [] as DbxColumn[]),
  ])
  return { result, columns }
}
