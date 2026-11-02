export interface DbxConnection {
  id: string
  name: string
  db_type: string
  host?: string
  port?: number
  database?: string
  default_schema?: string
}

export interface DbxQueryResult {
  columns: string[]
  column_types?: string[]
  rows: (string | number | boolean | null)[][]
  affected_rows: number
  execution_time_ms: number
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
  if (!response.ok) {
    const body = await response.text().catch(() => "")
    throw new Error(`${response.status} ${response.statusText}${body ? `: ${body.slice(0, 300)}` : ""}`)
  }
  return (await response.json()) as T
}

export function getConnections(): Promise<DbxConnection[]> {
  return request<DbxConnection[]>("/api/connection/list")
}

export function getDatabases(connectionId: string): Promise<Array<{ name: string }>> {
  return request(`/api/schema/databases?connection_id=${encodeURIComponent(connectionId)}`)
}

export function getSchemas(connectionId: string, database: string): Promise<string[]> {
  const query = new URLSearchParams({ connection_id: connectionId, database })
  return request(`/api/schema/schemas?${query}`)
}

export function getTables(connectionId: string, database: string, schema: string): Promise<Array<{ name: string; table_type?: string }>> {
  const query = new URLSearchParams({ connection_id: connectionId, database, schema })
  return request(`/api/schema/tables?${query}`)
}

export function executeQuery(connectionId: string, database: string, sql: string, schema?: string): Promise<DbxQueryResult> {
  return request<DbxQueryResult>("/api/query/execute", {
    method: "POST",
    body: JSON.stringify({ connectionId, database, sql, schema, maxRows: 1000 }),
  })
}
