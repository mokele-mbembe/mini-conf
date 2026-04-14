# 数据库保留与清理

本文记录本机 PostgreSQL 里哪些 database / schema 应该保留，哪些只是数据库测试留下的临时残留。

## GitHub Actions

GitHub Actions job 使用自己的临时 `postgres:16` service container。

- CI database URL 是 `postgres://postgres:postgres@127.0.0.1:5432/postgres`
- CI cache 缓存 Rust、Node、pnpm 和工具下载内容
- CI cache 不持久化 PostgreSQL service 的数据目录
- 已结束的 GitHub Actions job 不会在开发者本机 PostgreSQL 实例里留下 schema 或数据行

`backend-db` job 仍会在数据库测试后执行一次 best-effort 清理。这只是让 job 内部状态更干净；它不是长期存储清理机制，因为 service container 会在 job 结束后销毁。

## 需要保留

优先使用 database 名表达阶段用途：

- `mini_conf_ui_dev`：本机前端联调 / Local Preview runtime DB
- `mini_conf_test`：本机数据库集成测试 base DB
- `mini_conf_ci`：CI 或类 CI 环境 DB
- `mini_conf_staging`：共享黑盒 / staging DB

如果本机账号暂时没有 `CREATEDB` 权限，可以临时退回到同一个 database 下用显式 schema 区分用途。当前 WSL 环境采用的是：

- Database `mini_conf`
- Schema `mini_conf_ui_dev`：本机前端联调 / Local Preview runtime schema
- 测试仍使用隔离临时 schema：`mini_conf_<test-prefix>_<numeric timestamp>`

这是前端开发期为了方便 DataGrip 观察和本机联调采用的临时形态，不是长期目标。前端主路径开发完成后，应迁移回独立 database 命名：

- `mini_conf_ui_dev` 承载本机 UI runtime 数据
- `mini_conf_test` 承载本机数据库集成测试 base 数据
- `mini_conf_ci` / `mini_conf_staging` 按需要分别用于类 CI 与共享黑盒环境

迁移完成后，本机 `DATABASE_URL` 应直接指向 `mini_conf_ui_dev` database，不再依赖 `mini_conf?options=-csearch_path%3Dmini_conf_ui_dev`；本机 `TEST_DATABASE_URL` 应直接指向 `mini_conf_test` database。

除非你明确要重置整套本机环境，否则保留：

- 当前 `DATABASE_URL` 指向的 runtime database 或 runtime schema
- PostgreSQL 系统 schema：`pg_catalog`、`information_schema`
- PostgreSQL 维护 database：`postgres`、`template0`、`template1`

`public` 不应承载 mini-conf 应用数据。它是 PostgreSQL 默认 schema，名字无法表达阶段用途；如果发现 `mini_conf.public` 里有业务表，通常是早期未配置 `search_path` 或误用默认连接串留下的历史数据。

当 runtime URL 长这样时，`mini_conf_ui_dev` 就是本机前端联调常驻 schema：

```text
postgres://mini_conf:<password>@127.0.0.1:5432/mini_conf?options=-csearch_path%3Dmini_conf_ui_dev
```

在 DataGrip 里要确认 query console 连接的是 `mini_conf` database，而不是 `postgres` 维护库。提示符如果是 `postgres.public>`，说明当前查询不在本机 UI 数据所在的 database 上。

前端开发期结束后迁移到独立 database 时，建议先重新跑 `just dev-db-prepare-local` 生成 demo 数据；当前阶段的数据本来就是脚本生成的，可以丢弃，不需要做业务数据迁移。

## 临时残留

带数字后缀的 schema 由数据库集成测试通过 `infra::testing::unique_schema_name(...)` 创建。

示例：

```text
mini_conf_projects_1776132045830527268
mini_conf_releases_1775810732149102197
mini_conf_open_sync_177...
```

正常情况下，每个测试的 teardown 会 drop 掉自己的临时 schema。如果本地测试在 teardown 前被中断、取消或杀掉，这些 schema 就可能残留在本机 database 里。它们不是前端联调数据；确认没有本地数据库测试正在运行后，可以清理。

## 清理

预览测试残留 schema：

```bash
just db-list-test-schemas-local
```

删除匹配到的测试残留 schema：

```bash
just db-clean-test-schemas-local
```

清理命令只匹配测试命名约定：

```text
mini_conf_<test-prefix>_<numeric timestamp>
```

它不会匹配 `mini_conf_ui_dev`、`public`、`pg_catalog` 或 `information_schema`。
