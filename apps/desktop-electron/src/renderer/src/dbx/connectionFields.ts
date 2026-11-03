// Per-database-type connection form fields, mirroring the groups the upstream
// ConnectionDialog.vue renders. Every `key` is a real ConnectionConfig field so
// the payload can be posted straight to /api/connection/save.
import type { DatabaseTypeInfo } from "./databaseTypes"

export type FieldKind = "text" | "password" | "number" | "textarea" | "select" | "checkbox"

export interface FieldOption {
  value: string
  label: string
}

export type Values = Record<string, string | number | boolean>

export interface FieldSpec {
  /** ConnectionConfig key this control writes to. */
  key: string
  label: string
  kind: FieldKind
  placeholder?: string
  options?: FieldOption[]
  /** Seeded when the type is picked. */
  initial?: string | number | boolean
  hint?: string
  visibleWhen?: (values: Values) => boolean
}

const HOST: FieldSpec = { key: "host", label: "主机", kind: "text", placeholder: "127.0.0.1" }
const PORT: FieldSpec = { key: "port", label: "端口", kind: "number" }
const USER: FieldSpec = { key: "username", label: "用户名", kind: "text" }
const PASSWORD: FieldSpec = { key: "password", label: "密码", kind: "password" }
const DATABASE: FieldSpec = { key: "database", label: "数据库", kind: "text", placeholder: "可选" }
const SSL: FieldSpec = { key: "ssl", label: "启用 SSL", kind: "checkbox" }
const URL_PARAMS: FieldSpec = { key: "url_params", label: "附加参数", kind: "text", placeholder: "k=v&k2=v2" }

const DEFAULT_FIELDS: FieldSpec[] = [HOST, PORT, USER, PASSWORD, DATABASE]

/** sqlite / duckdb / access / h2 / jdbcx take a file path in the host field. */
function fileFields(type: DatabaseTypeInfo): FieldSpec[] {
  const fields: FieldSpec[] = [
    { key: "host", label: "数据库文件", kind: "text", placeholder: type.key === "jdbcx" ? "/path/to/database" : "/path/to/database.db" },
  ]
  if (type.key === "sqlite") {
    fields.push({ key: "password", label: "加密密钥", kind: "password", placeholder: "未加密则留空" })
  }
  if (type.key === "duckdb") {
    fields.push({ key: "init_script", label: "初始化脚本", kind: "textarea", placeholder: "INSTALL httpfs;\nLOAD httpfs;", hint: "连接建立后立即执行的 SQL" })
  }
  if (type.key === "access" || type.key === "h2") {
    fields.push({ ...USER, placeholder: type.key === "access" ? "可选" : "sa" }, PASSWORD)
  }
  if (type.key === "jdbcx") {
    fields.push({ key: "jdbc_driver_paths", label: "驱动路径", kind: "text", placeholder: "可选" })
  }
  return fields
}

const REDIS_FIELDS: FieldSpec[] = [
  {
    key: "redis_connection_mode",
    label: "模式",
    kind: "select",
    initial: "standalone",
    options: [
      { value: "standalone", label: "单机" },
      { value: "sentinel", label: "哨兵" },
      { value: "cluster", label: "集群" },
    ],
  },
  { ...HOST, visibleWhen: (v) => v.redis_connection_mode !== "sentinel" && v.redis_connection_mode !== "cluster" },
  { ...PORT, visibleWhen: (v) => v.redis_connection_mode !== "sentinel" && v.redis_connection_mode !== "cluster" },
  {
    key: "redis_sentinel_nodes",
    label: "哨兵节点",
    kind: "textarea",
    placeholder: "sentinel-1:26379\nsentinel-2:26379",
    visibleWhen: (v) => v.redis_connection_mode === "sentinel",
  },
  { key: "redis_sentinel_master", label: "主节点名", kind: "text", placeholder: "mymaster", visibleWhen: (v) => v.redis_connection_mode === "sentinel" },
  { key: "redis_sentinel_username", label: "哨兵用户名", kind: "text", visibleWhen: (v) => v.redis_connection_mode === "sentinel" },
  { key: "redis_sentinel_password", label: "哨兵密码", kind: "password", visibleWhen: (v) => v.redis_connection_mode === "sentinel" },
  { key: "redis_sentinel_tls", label: "哨兵启用 TLS", kind: "checkbox", visibleWhen: (v) => v.redis_connection_mode === "sentinel" },
  {
    key: "redis_cluster_nodes",
    label: "集群节点",
    kind: "textarea",
    placeholder: "redis-1:6379\nredis-2:6379",
    visibleWhen: (v) => v.redis_connection_mode === "cluster",
  },
  { ...USER, placeholder: "default" },
  PASSWORD,
]

const MONGO_FIELDS: FieldSpec[] = [
  { key: "connection_string", label: "连接串", kind: "textarea", placeholder: "mongodb+srv://user:密码@cluster.mongodb.net/db", hint: "填了就以它为准，下面的表单可留空" },
  HOST,
  PORT,
  SSL,
  USER,
  PASSWORD,
  DATABASE,
  { ...URL_PARAMS, placeholder: "replicaSet=rs0&authSource=admin" },
]

const DYNAMO_FIELDS: FieldSpec[] = [
  { key: "host", label: "端点", kind: "text", placeholder: "dynamodb.us-east-1.amazonaws.com" },
  PORT,
  { ...SSL, initial: true },
  { key: "database", label: "区域", kind: "text", placeholder: "us-east-1" },
  { key: "username", label: "Access Key ID", kind: "text" },
  { key: "password", label: "Secret Access Key", kind: "password" },
  { key: "connection_string", label: "Session Token", kind: "password", placeholder: "可选" },
]

const INFLUX_FIELDS: FieldSpec[] = [
  HOST,
  PORT,
  SSL,
  { key: "database", label: "数据库 / Bucket", kind: "text", placeholder: "my-database" },
  { key: "username", label: "用户名（v1）", kind: "text", placeholder: "可选" },
  { key: "password", label: "密码 / Token", kind: "password", placeholder: "v2/v3 填 Token" },
  { ...URL_PARAMS, placeholder: "epoch=ms" },
]

const MQTT_FIELDS: FieldSpec[] = [
  { key: "host", label: "Broker 地址", kind: "text", placeholder: "127.0.0.1" },
  PORT,
  USER,
  PASSWORD,
  SSL,
  { ...URL_PARAMS, placeholder: "clientId=…&keepAlive=60" },
]

const ETCD_FIELDS: FieldSpec[] = [
  HOST,
  PORT,
  { key: "etcd_endpoints", label: "Endpoints", kind: "textarea", placeholder: "http://127.0.0.1:2379\nhttps://etcd-2:2379", hint: "留空则用上面的主机和端口" },
  USER,
  PASSWORD,
]

const ZOOKEEPER_FIELDS: FieldSpec[] = [
  HOST,
  PORT,
  { key: "connection_string", label: "连接串", kind: "textarea", placeholder: "127.0.0.1:2181\nzk-2:2181" },
  { key: "url_params", label: "认证方式", kind: "text", placeholder: "digest 或 sasl_digest，可选" },
  USER,
  PASSWORD,
]

const NACOS_FIELDS: FieldSpec[] = [
  { key: "host", label: "服务地址", kind: "text", placeholder: "http://127.0.0.1:8848" },
  PORT,
  { ...URL_PARAMS, label: "命名空间 / 参数", placeholder: "namespace=public" },
  USER,
  PASSWORD,
]

const CONSUL_FIELDS: FieldSpec[] = [
  { key: "host", label: "地址", kind: "text", placeholder: "http://127.0.0.1:8500" },
  PORT,
  { key: "password", label: "ACL Token", kind: "password", placeholder: "可选" },
  { ...URL_PARAMS, label: "数据中心 / 参数", placeholder: "datacenter=dc1" },
]

const TURSO_FIELDS: FieldSpec[] = [
  { key: "host", label: "数据库 URL", kind: "text", placeholder: "libsql://db-name.turso.io" },
  { key: "password", label: "Auth Token", kind: "password", placeholder: "turso db tokens create …" },
  URL_PARAMS,
]

const D1_FIELDS: FieldSpec[] = [
  { key: "host", label: "Account ID", kind: "text" },
  { key: "database", label: "Database ID", kind: "text" },
  { key: "password", label: "API Token", kind: "password" },
]

const JDBC_FIELDS: FieldSpec[] = [
  { key: "connection_string", label: "JDBC URL", kind: "textarea", placeholder: "jdbc:…" },
  USER,
  PASSWORD,
  { key: "jdbc_driver_class", label: "驱动类", kind: "text", placeholder: "org.h2.Driver，可选" },
  { key: "jdbc_driver_paths", label: "驱动路径", kind: "text", placeholder: "可选" },
]

const VICTORIA_FIELDS: FieldSpec[] = [
  HOST,
  PORT,
  SSL,
  USER,
  PASSWORD,
  { ...URL_PARAMS, label: "API 路径 / 参数", placeholder: "/prometheus" },
]

const ORACLE_FIELDS: FieldSpec[] = [
  {
    key: "oracle_connection_type",
    label: "连接方式",
    kind: "select",
    initial: "service_name",
    options: [
      { value: "service_name", label: "Service Name" },
      { value: "sid", label: "SID" },
      { value: "tns", label: "TNS 别名" },
    ],
  },
  { ...HOST, visibleWhen: (v) => v.oracle_connection_type !== "tns" },
  { ...PORT, visibleWhen: (v) => v.oracle_connection_type !== "tns" },
  {
    key: "database",
    label: "服务名 / SID",
    kind: "text",
    placeholder: "ORCL",
    visibleWhen: (v) => v.oracle_connection_type !== "tns",
  },
  { key: "connection_string", label: "TNS 别名", kind: "text", placeholder: "ORCLPDB", visibleWhen: (v) => v.oracle_connection_type === "tns" },
  USER,
  PASSWORD,
]

const MONGO_KEYS = new Set(["mongodb"])
const FILE_KEYS = new Set(["sqlite", "duckdb", "access", "h2", "jdbcx"])

/** The field set the upstream dialog shows for this picker entry. */
export function connectionFields(type: DatabaseTypeInfo): FieldSpec[] {
  const key = type.key
  if (FILE_KEYS.has(key)) return fileFields(type)
  if (key === "redis") return REDIS_FIELDS
  if (MONGO_KEYS.has(key)) return MONGO_FIELDS
  if (key === "dynamodb") return DYNAMO_FIELDS
  if (key === "influxdb") return INFLUX_FIELDS
  if (key === "mqtt") return MQTT_FIELDS
  if (key === "etcd") return ETCD_FIELDS
  if (key === "zookeeper") return ZOOKEEPER_FIELDS
  if (key === "nacos") return NACOS_FIELDS
  if (key === "consul") return CONSUL_FIELDS
  if (key === "turso") return TURSO_FIELDS
  if (key === "cloudflare-d1") return D1_FIELDS
  if (key === "dremio" || key === "jdbcx") return JDBC_FIELDS
  if (key === "victoriametrics") return VICTORIA_FIELDS
  if (key === "oracle") return ORACLE_FIELDS
  return DEFAULT_FIELDS
}

/** Starting values for a freshly picked type. */
export function connectionDefaults(type: DatabaseTypeInfo): Values {
  const values: Values = { port: type.localFile ? 0 : type.defaultPort, username: type.defaultUser }
  for (const field of connectionFields(type)) {
    if (field.initial !== undefined) values[field.key] = field.initial
  }
  return values
}

/** Pair a number field with the text field before it so they share a row. */
export function fieldRows(fields: FieldSpec[]): FieldSpec[][] {
  const rows: FieldSpec[][] = []
  for (const field of fields) {
    const last = rows[rows.length - 1]
    if (field.kind === "number" && last?.length === 1 && last[0].kind === "text") last.push(field)
    else rows.push([field])
  }
  return rows
}
