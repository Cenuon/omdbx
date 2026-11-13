<div align="center">

# OMDBX · Cenuon fork

**Rust + GPUI 原生桌面界面，后端沿用上游 DBX（`dbx-web`）**

连接 → schema 树 → 表数据浏览 → SQL 查询，全部由真实 DBX 后端驱动。

[下载安装包](https://github.com/Cenuon/omdbx/releases) · [上游项目](https://github.com/t8y2/dbx)

</div>

---

> **这是 [`t8y2/dbx`](https://github.com/t8y2/dbx) 的个人 fork。**
> 后端保持上游 `crates/dbx-web` 不变，桌面端换成自研的 Rust + GPUI 界面，放在 `apps/desktop-gpui/`。

## 这是什么

- **后端**：上游 DBX 的 Web 服务 `dbx-web`（Rust），以独立进程运行，监听 `127.0.0.1:4225`。
- **前端**：本 fork 自研的 Rust + GPUI 桌面界面（`apps/desktop-gpui/`），通过 HTTP 调用 `dbx-web`，不重复实现连接层。
- **数据来源**：界面显示的连接、库/schema、表、列、行都是 `dbx-web` 的真实返回。

上游 DBX 本身支持 MySQL / PostgreSQL / SQLite / Redis / MongoDB / DuckDB / SQL Server / 达梦 等大量数据库，完整的数据库与功能列表见 [上游 README](https://github.com/t8y2/dbx#readme)。

## 已接通的功能

- 左侧连接列表：读取已保存的连接，显示连接状态。
- schema 树：database / schema → 表列表。
- 表数据浏览：按真实列渲染表头、类型与主键，显示行数与耗时。
- SQL 查询：编辑器输入 SQL 执行 `/api/query/execute`，结果按真实返回的列渲染。
- 结果导出：CSV / JSON / Markdown / HTML / SQL INSERT / TXT / XLSX。
- 明暗主题、多语言（简体中文 / 繁體中文 / English / Português / Русский / Italiano）。
- 启动时检查 GitHub Releases 更新。

## 下载安装

Release 中的 macOS 安装包已经内置 `dbx-web`：启动应用会自动拉起后端，退出时自动回收，不需要另外安装或启动服务。

- macOS Apple Silicon：`OMDBX-<版本>-arm64.dmg`（推荐）或 `OMDBX-<版本>-arm64-mac.zip`
- 安装包未使用 Apple Developer ID 签名；首次打开请右键应用图标 →「打开」。
- 连接、查询历史等数据保存在 `~/Library/Application Support/OMDBX/dbx-web/`，与旧版 Electron 构建共用。

## 从源码运行

需要 Rust 1.97+。后端与前端都是 Rust，不需要 Node。

```bash
# 一条命令：编译后端 → 起 sidecar（127.0.0.1:4225）→ 起 GPUI 窗口
make dev

# 只起后端
make dev-backend

# 只编译
make build

# 生成当前 Mac 架构的 .app / .dmg / zip（产物在 dist/desktop-gpui/）
make package-macos
```

也可以手动来：

```bash
cargo build --release -p dbx-web \
  --no-default-features \
  --features duckdb-sidecar,dynamodb,mq-admin,dbx-core/sqlite-bundled

sh apps/desktop-gpui/run-backend.sh          # 终端 1
cd apps/desktop-gpui && cargo run --release  # 终端 2
```

本地开发的数据目录：`apps/desktop-gpui/.dbx-web-data/`（连接与历史保存在这里）。打包后的应用使用上面的 `~/Library/Application Support/OMDBX/dbx-web/`。

## 目录结构

```
apps/desktop-gpui/       本 fork 的 Rust + GPUI 前端
crates/dbx-web/          上游后端（不改动）
crates/                  上游各数据库驱动与核心库（不改动）
plugins/                 后端插件（驱动方言等）
vendor/                  后端依赖的本地补丁
```

## 跟随上游升级

```bash
git remote add upstream https://github.com/t8y2/dbx.git   # 只需一次
git fetch upstream
git merge upstream/main
```

前端全部在 `apps/desktop-gpui/`，后端与其余目录保持上游原样，因此合并冲突通常只可能出现在本 README。

## 许可与致谢

本项目基于上游 DBX，遵循 [Apache-2.0](LICENSE)；上游的 `NOTICE` 一并保留。

- 上游项目：<https://github.com/t8y2/dbx>
- 上游文档与完整功能说明：<https://github.com/t8y2/dbx#readme>
