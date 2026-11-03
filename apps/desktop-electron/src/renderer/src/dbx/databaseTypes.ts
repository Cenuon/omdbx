// Generated from the upstream descriptors in plugins/connection-types/*.yaml
// plus apps/desktop/src/types/generated/connectionProfiles.ts (picker labels, categories, icons).
// Re-generate when upstream adds or changes a database type.

export type DatabaseCategory =
  | "sql"
  | "analytics"
  | "domestic"
  | "lightweight"
  | "document"
  | "graph_ai"
  | "timeseries"
  | "mq"
  | "registry_config"

export interface DatabaseTypeInfo {
  key: string
  label: string
  category: DatabaseCategory
  /** value sent to the backend as db_type */
  dbType: string
  defaultPort: number
  defaultUser: string
  /** file-backed connections take a path in the host field */
  localFile: boolean
  icon: string
}

export const databaseCategories: Array<{ key: DatabaseCategory; label: string }> = [
  { key: "sql", label: "关系型数据库" },
  { key: "analytics", label: "数仓分析" },
  { key: "domestic", label: "国产数据库" },
  { key: "lightweight", label: "轻量数据库" },
  { key: "document", label: "文档/缓存/检索" },
  { key: "graph_ai", label: "图谱/向量" },
  { key: "timeseries", label: "时序数据库" },
  { key: "mq", label: "消息队列" },
  { key: "registry_config", label: "注册与配置" },
]

export const databaseTypes: DatabaseTypeInfo[] = [
  { key: "mysql", label: "MySQL", category: "sql", dbType: "mysql", defaultPort: 3306, defaultUser: "root", localFile: false, icon: "/db-icons/mysql.svg" },
  { key: "postgres", label: "PostgreSQL", category: "sql", dbType: "postgres", defaultPort: 5432, defaultUser: "postgres", localFile: false, icon: "/db-icons/postgres.svg" },
  { key: "cloudberry", label: "Apache Cloudberry", category: "analytics", dbType: "postgres", defaultPort: 5432, defaultUser: "postgres", localFile: false, icon: "/db-icons/cloudberry.svg" },
  { key: "opentenbase", label: "OpenTenBase", category: "domestic", dbType: "postgres", defaultPort: 11000, defaultUser: "opentenbase", localFile: false, icon: "/db-icons/opentenbase.svg" },
  { key: "redis", label: "Redis", category: "document", dbType: "redis", defaultPort: 6379, defaultUser: "", localFile: false, icon: "/db-icons/redis.svg" },
  { key: "sqlite", label: "SQLite", category: "lightweight", dbType: "sqlite", defaultPort: 0, defaultUser: "", localFile: true, icon: "/db-icons/sqlite.svg" },
  { key: "rqlite", label: "RQLite", category: "lightweight", dbType: "rqlite", defaultPort: 4001, defaultUser: "", localFile: false, icon: "/db-icons/rqlite.png" },
  { key: "turso", label: "Turso", category: "lightweight", dbType: "turso", defaultPort: 443, defaultUser: "", localFile: false, icon: "/db-icons/turso.png" },
  { key: "cloudflare-d1", label: "Cloudflare D1", category: "lightweight", dbType: "cloudflare-d1", defaultPort: 443, defaultUser: "", localFile: false, icon: "/db-icons/cloudflare-d1.svg" },
  { key: "duckdb", label: "DuckDB", category: "lightweight", dbType: "duckdb", defaultPort: 0, defaultUser: "", localFile: true, icon: "/db-icons/duckdb.svg" },
  { key: "access", label: "Microsoft Access", category: "lightweight", dbType: "access", defaultPort: 0, defaultUser: "", localFile: true, icon: "/db-icons/access.png" },
  { key: "mongodb", label: "MongoDB", category: "document", dbType: "mongodb", defaultPort: 27017, defaultUser: "", localFile: false, icon: "/db-icons/mongodb.svg" },
  { key: "dynamodb", label: "Amazon DynamoDB", category: "document", dbType: "dynamodb", defaultPort: 443, defaultUser: "", localFile: false, icon: "/db-icons/dynamodb.svg" },
  { key: "clickhouse", label: "ClickHouse", category: "analytics", dbType: "clickhouse", defaultPort: 8123, defaultUser: "default", localFile: false, icon: "/db-icons/clickhouse.svg" },
  { key: "sqlserver", label: "SQL Server", category: "sql", dbType: "sqlserver", defaultPort: 1433, defaultUser: "sa", localFile: false, icon: "/db-icons/sqlserver.svg" },
  { key: "oracle", label: "Oracle", category: "sql", dbType: "oracle", defaultPort: 1521, defaultUser: "system", localFile: false, icon: "/db-icons/oracle.svg" },
  { key: "elasticsearch", label: "Elasticsearch", category: "document", dbType: "elasticsearch", defaultPort: 9200, defaultUser: "", localFile: false, icon: "/db-icons/elasticsearch.svg" },
  { key: "easysearch", label: "Easysearch", category: "document", dbType: "easysearch", defaultPort: 9200, defaultUser: "", localFile: false, icon: "/db-icons/easysearch.svg" },
  { key: "solr", label: "Apache Solr", category: "document", dbType: "solr", defaultPort: 8983, defaultUser: "", localFile: false, icon: "/db-icons/solr.svg" },
  { key: "couchdb", label: "Apache CouchDB", category: "document", dbType: "couchdb", defaultPort: 5984, defaultUser: "", localFile: false, icon: "/db-icons/couchdb.svg" },
  { key: "meilisearch", label: "Meilisearch", category: "document", dbType: "meilisearch", defaultPort: 7700, defaultUser: "", localFile: false, icon: "/db-icons/meilisearch.svg" },
  { key: "hbase", label: "Apache HBase", category: "document", dbType: "hbase", defaultPort: 8080, defaultUser: "", localFile: false, icon: "/db-icons/hbase.svg" },
  { key: "qdrant", label: "Qdrant", category: "graph_ai", dbType: "qdrant", defaultPort: 6333, defaultUser: "", localFile: false, icon: "/db-icons/qdrant.svg" },
  { key: "milvus", label: "Milvus", category: "graph_ai", dbType: "milvus", defaultPort: 19530, defaultUser: "root", localFile: false, icon: "/db-icons/milvus.png" },
  { key: "weaviate", label: "Weaviate", category: "graph_ai", dbType: "weaviate", defaultPort: 8080, defaultUser: "", localFile: false, icon: "/db-icons/weaviate.svg" },
  { key: "chromadb", label: "ChromaDB", category: "graph_ai", dbType: "chromadb", defaultPort: 8000, defaultUser: "", localFile: false, icon: "/db-icons/chromadb.svg" },
  { key: "mariadb", label: "MariaDB", category: "sql", dbType: "mysql", defaultPort: 3306, defaultUser: "root", localFile: false, icon: "/db-icons/mariadb.svg" },
  { key: "tidb", label: "TiDB", category: "domestic", dbType: "mysql", defaultPort: 4000, defaultUser: "root", localFile: false, icon: "/db-icons/tidb.svg" },
  { key: "oceanbase", label: "OceanBase", category: "domestic", dbType: "mysql", defaultPort: 2883, defaultUser: "root", localFile: false, icon: "/db-icons/oceanbase.svg" },
  { key: "goldendb", label: "金篆 GoldenDB", category: "domestic", dbType: "goldendb", defaultPort: 3306, defaultUser: "root", localFile: false, icon: "/db-icons/goldendb.png" },
  { key: "databend", label: "Databend", category: "analytics", dbType: "databend", defaultPort: 8000, defaultUser: "databend", localFile: false, icon: "/db-icons/databend.svg" },
  { key: "tdsql", label: "TDSQL", category: "domestic", dbType: "mysql", defaultPort: 3306, defaultUser: "root", localFile: false, icon: "/db-icons/tdsql.svg" },
  { key: "polardb", label: "PolarDB", category: "domestic", dbType: "mysql", defaultPort: 3306, defaultUser: "root", localFile: false, icon: "/db-icons/polardb.webp" },
  { key: "greatsql", label: "GreatSQL", category: "domestic", dbType: "mysql", defaultPort: 3306, defaultUser: "root", localFile: false, icon: "/db-icons/greatsql.webp" },
  { key: "databricks", label: "Databricks SQL", category: "analytics", dbType: "databricks", defaultPort: 443, defaultUser: "token", localFile: false, icon: "/db-icons/databricks.svg" },
  { key: "saphana", label: "SAP HANA", category: "analytics", dbType: "saphana", defaultPort: 30015, defaultUser: "SYSTEM", localFile: false, icon: "/db-icons/saphana.svg" },
  { key: "teradata", label: "Teradata", category: "analytics", dbType: "teradata", defaultPort: 1025, defaultUser: "", localFile: false, icon: "/db-icons/teradata.svg" },
  { key: "vertica", label: "Vertica", category: "analytics", dbType: "vertica", defaultPort: 5433, defaultUser: "dbadmin", localFile: false, icon: "/db-icons/vertica.webp" },
  { key: "firebird", label: "Firebird", category: "sql", dbType: "firebird", defaultPort: 3050, defaultUser: "SYSDBA", localFile: false, icon: "/db-icons/firebird.svg" },
  { key: "exasol", label: "Exasol", category: "analytics", dbType: "exasol", defaultPort: 8563, defaultUser: "sys", localFile: false, icon: "/db-icons/exasol.svg" },
  { key: "gbase", label: "南大通用 GBase", category: "domestic", dbType: "gbase", defaultPort: 5258, defaultUser: "gbasedbt", localFile: false, icon: "/db-icons/gbase.png" },
  { key: "opengauss", label: "openGauss", category: "domestic", dbType: "opengauss", defaultPort: 5432, defaultUser: "gaussdb", localFile: false, icon: "/db-icons/opengauss.svg" },
  { key: "gaussdb", label: "GaussDB", category: "domestic", dbType: "gaussdb", defaultPort: 5432, defaultUser: "gaussdb", localFile: false, icon: "/db-icons/gaussdb.svg" },
  { key: "kwdb", label: "KWDB", category: "domestic", dbType: "kwdb", defaultPort: 26257, defaultUser: "root", localFile: false, icon: "/db-icons/kwdb.svg" },
  { key: "questdb", label: "QuestDB", category: "timeseries", dbType: "questdb", defaultPort: 8812, defaultUser: "questdb", localFile: false, icon: "/db-icons/questdb.svg" },
  { key: "kingbase", label: "金仓KingbaseES", category: "domestic", dbType: "kingbase", defaultPort: 54321, defaultUser: "system", localFile: false, icon: "/db-icons/kingbase.svg" },
  { key: "highgo", label: "瀚高 HighGo", category: "domestic", dbType: "highgo", defaultPort: 5866, defaultUser: "highgo", localFile: false, icon: "/db-icons/highgo.png" },
  { key: "uxdb", label: "优炫 UXDB", category: "domestic", dbType: "uxdb", defaultPort: 52025, defaultUser: "uxdb", localFile: false, icon: "/db-icons/uxdb.svg" },
  { key: "yashandb", label: "崖山 YashanDB", category: "domestic", dbType: "yashandb", defaultPort: 1688, defaultUser: "sys", localFile: false, icon: "/db-icons/yashandb.png" },
  { key: "vastbase", label: "海量 Vastbase", category: "domestic", dbType: "vastbase", defaultPort: 5432, defaultUser: "vastbase", localFile: false, icon: "/db-icons/vastbase.svg" },
  { key: "doris", label: "Doris", category: "analytics", dbType: "mysql", defaultPort: 9030, defaultUser: "root", localFile: false, icon: "/db-icons/doris.svg" },
  { key: "selectdb", label: "SelectDB", category: "analytics", dbType: "mysql", defaultPort: 9030, defaultUser: "root", localFile: false, icon: "/db-icons/selectdb.svg" },
  { key: "starrocks", label: "StarRocks", category: "analytics", dbType: "mysql", defaultPort: 9030, defaultUser: "root", localFile: false, icon: "/db-icons/starrocks.svg" },
  { key: "manticoresearch", label: "Manticore Search", category: "document", dbType: "manticoresearch", defaultPort: 9306, defaultUser: "root", localFile: false, icon: "/db-icons/manticoresearch.png" },
  { key: "redshift", label: "Redshift", category: "analytics", dbType: "redshift", defaultPort: 5439, defaultUser: "awsuser", localFile: false, icon: "/db-icons/redshift.svg" },
  { key: "cockroachdb", label: "CockroachDB", category: "sql", dbType: "postgres", defaultPort: 26257, defaultUser: "root", localFile: false, icon: "/db-icons/cockroachdb.svg" },
  { key: "dm", label: "达梦 Dameng", category: "domestic", dbType: "dameng", defaultPort: 5236, defaultUser: "SYSDBA", localFile: false, icon: "/db-icons/dm.svg" },
  { key: "h2", label: "H2", category: "lightweight", dbType: "h2", defaultPort: 9092, defaultUser: "sa", localFile: false, icon: "/db-icons/h2.svg" },
  { key: "snowflake", label: "Snowflake", category: "analytics", dbType: "snowflake", defaultPort: 443, defaultUser: "", localFile: false, icon: "/db-icons/snowflake.svg" },
  { key: "trino", label: "Trino", category: "analytics", dbType: "trino", defaultPort: 8080, defaultUser: "", localFile: false, icon: "/db-icons/trino.svg" },
  { key: "prestosql", label: "PrestoSQL", category: "analytics", dbType: "prestosql", defaultPort: 8080, defaultUser: "", localFile: false, icon: "/db-icons/presto.svg" },
  { key: "hive", label: "Hive", category: "analytics", dbType: "hive", defaultPort: 10000, defaultUser: "", localFile: false, icon: "/db-icons/hive.svg" },
  { key: "kyuubi", label: "Apache Kyuubi", category: "analytics", dbType: "kyuubi", defaultPort: 10009, defaultUser: "", localFile: false, icon: "/db-icons/kyuubi.png" },
  { key: "transwarp-inceptor", label: "星环Inceptor", category: "domestic", dbType: "transwarp", defaultPort: 10000, defaultUser: "", localFile: false, icon: "/db-icons/transwarp-inceptor.png" },
  { key: "argo", label: "ArgoDB", category: "domestic", dbType: "argo", defaultPort: 10000, defaultUser: "", localFile: false, icon: "/db-icons/hive.svg" },
  { key: "impala", label: "Apache Impala", category: "analytics", dbType: "impala", defaultPort: 21050, defaultUser: "", localFile: false, icon: "/db-icons/impala.svg" },
  { key: "spark", label: "Apache Spark", category: "analytics", dbType: "spark", defaultPort: 10015, defaultUser: "", localFile: false, icon: "/db-icons/spark-logo.png" },
  { key: "db2", label: "DB2", category: "sql", dbType: "db2", defaultPort: 50000, defaultUser: "db2inst1", localFile: false, icon: "/db-icons/db2.svg" },
  { key: "informix", label: "Informix", category: "sql", dbType: "informix", defaultPort: 9088, defaultUser: "informix", localFile: false, icon: "/db-icons/informix.svg" },
  { key: "dremio", label: "Dremio", category: "analytics", dbType: "jdbc", defaultPort: 31010, defaultUser: "", localFile: false, icon: "/db-icons/dremio.svg" },
  { key: "jdbcx", label: "JDBCX", category: "sql", dbType: "jdbc", defaultPort: 0, defaultUser: "", localFile: true, icon: "/db-icons/jdbcx.svg" },
  { key: "neo4j", label: "Neo4j", category: "graph_ai", dbType: "neo4j", defaultPort: 7687, defaultUser: "neo4j", localFile: false, icon: "/db-icons/neo4j.svg" },
  { key: "nebula", label: "NebulaGraph", category: "graph_ai", dbType: "nebula", defaultPort: 9669, defaultUser: "root", localFile: false, icon: "/db-icons/nebula.png" },
  { key: "cassandra", label: "Cassandra", category: "document", dbType: "cassandra", defaultPort: 9042, defaultUser: "cassandra", localFile: false, icon: "/db-icons/cassandra.svg" },
  { key: "bigquery", label: "BigQuery", category: "analytics", dbType: "bigquery", defaultPort: 443, defaultUser: "", localFile: false, icon: "/db-icons/bigquery.svg" },
  { key: "spanner", label: "Cloud Spanner", category: "sql", dbType: "spanner", defaultPort: 443, defaultUser: "", localFile: false, icon: "/db-icons/spanner.svg" },
  { key: "kylin", label: "Kylin", category: "analytics", dbType: "kylin", defaultPort: 7070, defaultUser: "ADMIN", localFile: false, icon: "/db-icons/apache_kylin.svg" },
  { key: "ignite", label: "Apache Ignite", category: "analytics", dbType: "ignite", defaultPort: 10800, defaultUser: "", localFile: false, icon: "/db-icons/apache_ignite.svg" },
  { key: "ignite3", label: "Apache Ignite 3", category: "analytics", dbType: "ignite3", defaultPort: 10800, defaultUser: "", localFile: false, icon: "/db-icons/apache_ignite.svg" },
  { key: "sundb", label: "科蓝 SUNDB", category: "domestic", dbType: "sundb", defaultPort: 22000, defaultUser: "root", localFile: false, icon: "/db-icons/sundb.svg" },
  { key: "oscar", label: "神通 OSCAR", category: "domestic", dbType: "oscar", defaultPort: 2003, defaultUser: "SYSDBA", localFile: false, icon: "/db-icons/oscar.png" },
  { key: "tdengine", label: "TDengine", category: "timeseries", dbType: "tdengine", defaultPort: 6041, defaultUser: "root", localFile: false, icon: "/db-icons/tdengine.svg" },
  { key: "xugu", label: "虚谷 XuguDB", category: "domestic", dbType: "xugu", defaultPort: 5138, defaultUser: "", localFile: false, icon: "/db-icons/xugu.png" },
  { key: "iotdb", label: "Apache IoTDB", category: "timeseries", dbType: "iotdb", defaultPort: 6667, defaultUser: "root", localFile: false, icon: "/db-icons/iotdb.svg" },
  { key: "etcd", label: "etcd", category: "registry_config", dbType: "etcd", defaultPort: 2379, defaultUser: "", localFile: false, icon: "/db-icons/etcd.svg" },
  { key: "zookeeper", label: "Apache ZooKeeper", category: "registry_config", dbType: "zookeeper", defaultPort: 2181, defaultUser: "", localFile: false, icon: "/db-icons/zookeeper.svg" },
  { key: "mq", label: "Apache Pulsar", category: "mq", dbType: "mq", defaultPort: 8080, defaultUser: "", localFile: false, icon: "/db-icons/pulsar.svg" },
  { key: "kafka", label: "Apache Kafka", category: "mq", dbType: "mq", defaultPort: 9092, defaultUser: "", localFile: false, icon: "/db-icons/kafka.svg" },
  { key: "rocketmq", label: "Apache RocketMQ", category: "mq", dbType: "mq", defaultPort: 9876, defaultUser: "", localFile: false, icon: "/db-icons/rocketmq.svg" },
  { key: "rabbitmq", label: "RabbitMQ", category: "mq", dbType: "mq", defaultPort: 5672, defaultUser: "", localFile: false, icon: "/db-icons/rabbitmq.svg" },
  { key: "nacos", label: "Nacos", category: "registry_config", dbType: "nacos", defaultPort: 8848, defaultUser: "nacos", localFile: false, icon: "/db-icons/nacos.png" },
  { key: "consul", label: "Consul", category: "registry_config", dbType: "consul", defaultPort: 8500, defaultUser: "", localFile: false, icon: "/db-icons/consul.svg" },
  { key: "mqtt", label: "MQTT", category: "mq", dbType: "mqtt", defaultPort: 1883, defaultUser: "", localFile: false, icon: "/db-icons/mqtt.svg" },
  { key: "iris", label: "IRIS", category: "sql", dbType: "iris", defaultPort: 1972, defaultUser: "_SYSTEM", localFile: false, icon: "/db-icons/iris.svg" },
  { key: "cache", label: "InterSystems Caché", category: "sql", dbType: "iris", defaultPort: 1972, defaultUser: "_SYSTEM", localFile: false, icon: "/db-icons/iris.svg" },
  { key: "influxdb", label: "InfluxDB", category: "timeseries", dbType: "influxdb", defaultPort: 8086, defaultUser: "", localFile: false, icon: "/db-icons/influxdb.svg" },
  { key: "victoriametrics", label: "VictoriaMetrics", category: "timeseries", dbType: "victoriametrics", defaultPort: 8428, defaultUser: "", localFile: false, icon: "/db-icons/victoriametrics.png" },
  { key: "salesforce", label: "Salesforce", category: "analytics", dbType: "salesforce", defaultPort: 443, defaultUser: "", localFile: false, icon: "/db-icons/salesforce.svg" },
  { key: "dolt", label: "Dolt", category: "sql", dbType: "mysql", defaultPort: 3306, defaultUser: "root", localFile: false, icon: "/db-icons/dolt.svg" },
]

