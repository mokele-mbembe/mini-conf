# 项目脚手架与启动清单

## 1. Rust workspace

建议结构：

```text
mini-conf/
  Cargo.toml
  apps/
    server/
    web/
  crates/
    domain/
    infra/
    schema/
  migrations/
  scripts/
  deploy/
  docs/
  .github/workflows/
  justfile
```

## 2. server 依赖建议

`apps/server/Cargo.toml` 先放这些：

```toml
[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
tower-http = { version = "0.6", features = ["fs", "trace", "cors"] }
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "postgres", "macros", "migrate", "chrono", "uuid", "json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
utoipa = "5"
utoipa-swagger-ui = { version = "9", features = ["axum"] }
anyhow = "1"
thiserror = "2"
dotenvy = "0.15"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
sha2 = "0.10"
argon2 = "0.5"
```

## 3. 后端模块骨架

建议先建这些模块：

- `config`
- `bootstrap`
- `http`
- `state`
- `error`
- `auth`
- `project`
- `project_member`
- `config_file`
- `deployment_instance`
- `deployment_credential`
- `draft`
- `release`
- `deployment_access`
- `deployment_sync`
- `audit`
- `openapi`

## 4. 环境变量建议

```env
APP_ENV=dev
HTTP_ADDR=0.0.0.0:8080
DATABASE_URL=postgres://mini_conf:secret@127.0.0.1:5432/mini_conf
DATABASE_ADMIN_URL=postgres://postgres:secret@127.0.0.1:5432/postgres
INIT_DB_ON_BOOT=true
INIT_ADMIN_USERNAME=admin
INIT_ADMIN_PASSWORD=admin123456
INIT_USERS_FILE=tests/alpha/users.seed.yaml
STATIC_DIR=apps/web/dist
ADMIN_AUTH_MODE=session
JWT_ENABLED=false
OPENAPI_EXPORT_PATH=docs/artifacts/openapi.json
```

## 5. 首批 API 路由

管理端：

- `GET /api/healthz`
- `POST /api/auth/login`
- `POST /api/auth/logout`
- `GET /api/auth/me`
- `GET /api/projects`
- `POST /api/projects`
- `GET /api/projects/:id/members`
- `POST /api/projects/:id/members`
- `GET /api/config-files`
- `POST /api/config-files`
- `GET /api/deployment-instances`
- `POST /api/deployment-instances`
- `POST /api/deployment-instances/:id/clone`
- `GET /api/deployment-heartbeats`
- `GET /api/drafts/:deploymentId/:configFileId`
- `PUT /api/drafts/:deploymentId/:configFileId`
- `POST /api/releases/publish`
- `GET /api/releases`
- `GET /api/releases/:id`
- `GET /api/releases/:id/diff`
- `POST /api/deployment-instances/:id/token/reset`

开放消费端：

- `GET /api/open/configs/resolve`
- `GET /api/open/releases/:revision`
- `GET /api/open/deployments/:deploymentKey/config-bundle`
- `POST /api/open/deployment-sync-records`
- `POST /api/open/heartbeats`

## 6. 前端页面顺序

建议按这个顺序开发：

1. 登录页
2. 项目列表页
3. 项目成员页
4. 配置文件列表页
5. 部署实例列表页
6. Draft 编辑页
7. Release 历史页
8. Diff 对比页
9. 部署实例同步记录页

在继续推进前端页面前，建议先补一套本机长期保留的 runtime DB，用于观察真实页面状态，而不是一直复用测试库。

## 7. 自动化命令建议

- `just dev-server`
- `just dev-web`
- `just lint`
- `just test`
- `just test-backend-db`
- `just perf-smoke`
- `just sqlx-check`
- `just openapi-check`
- `just db-migrate-up`
- `just db-migrate-down`
- `just dev-seed-demo`
- `just dev-seed-demo-local`
- `just dev-db-prepare-local`
- `just test-e2e`
- `just ci-local`
- `just ci-local-db`
- `just ci-local-full`
- `just db-reset-dev`
- `just db-list-test-schemas-local`
- `just db-clean-test-schemas-local`

当前仓库约定：

- `Core` 工作流：`just lint`、`just test`、`just openapi-check`
- `Isolated DB` 工作流：`just db-migrate-up`、`just db-migrate-down`、`just test-backend-db`
- `Local Preview / UI Dev` 工作流：`just db-migrate-up-local`、`just dev-seed-demo-local`、`just dev-db-prepare-local`、`just run-server-local`、`just dev-web`
- 本机 local wrapper：`just run-server-local`、`just db-migrate-up-local`、`just db-migrate-down-local`、`just test-backend-db-local`
- 本机 CI 分层：`just ci-local` 不要求数据库；`just ci-local-db` 复用 local wrapper 对齐 GitHub `backend-db`，并在缺少运行库配置时回退到 local test DB；`just ci-local-full` 串联两层

统一规范见 [docs/agents/STANDARD_WORKFLOW.md](../agents/STANDARD_WORKFLOW.md)。

数据库命令入口约定：

- portable 命令只读取显式 `DATABASE_URL` / `TEST_DATABASE_URL`
- `scripts/local-db-env.sh` 只负责本机 local wrapper 的 DSN 解析
- `scripts/dev-db-env.sh` 仅保留为兼容壳
- Rust 数据库测试代码本身只读取 `TEST_DATABASE_URL`
- 数据库名不绑定产品名，推荐按场景显式命名，例如 `mini_conf_ui_dev`、`mini_conf_test`、`mini_conf_ci`、`mini_conf_staging`
- 前端联调推荐额外保留一套独立运行库，例如 `mini_conf_ui_dev`
- demo 数据脚本只写运行库，不写测试库
- 如果本机数据库账号没有 `CREATEDB` 权限，可先使用同一 database 下的显式 schema，并通过 `DATABASE_URL` 的 `search_path` 指向它
- 同一 database 下显式 schema 只是前端开发期的便利形态；前端主路径完成后，回归独立 database 命名，至少拆出 `mini_conf_ui_dev` 和 `mini_conf_test`
- `public` schema 不承载 mini-conf 应用数据；如果本机看到 `mini_conf.public` 有业务表，通常是历史默认连接串或误用无 `search_path` 连接留下的数据
- GitHub Actions 的 PostgreSQL service 是一次性容器；Actions 缓存只覆盖依赖和工具缓存，不会把 CI 数据库内容写回本机或长期保存
- 本机残留的 `mini_conf_<test-prefix>_<数字时间戳>` schema 来自本地数据库测试中断或异常退出，不是 CI 缓存数据；确认无本地测试运行后可用 `just db-clean-test-schemas-local` 清理

数据库集成测试约定：

- 测试文件不要各自重复解析环境变量
- 统一复用 [`infra::testing`](../../crates/infra/src/testing.rs) 中的 `test_database_url`、`unique_schema_name`、`with_search_path`
- 这样可以把 Linux / WSL2 / 本地 shell 的环境差异收口在一处，避免后续新增测试时再引入分叉

本地前端联调建议：

1. 配置 `MINI_CONF_LOCAL_DB_*` 或 `MINI_CONF_LOCAL_DATABASE_URL`
2. 执行 `just dev-db-prepare-local`
3. 运行 `just run-server-local`
4. 运行 `just dev-web`

`just dev-db-prepare-local` 会：

- 对 runtime DB 执行迁移
- 写入可重复执行的 demo 用户、项目、配置、部署实例、Draft、Release、sync records、heartbeats、audit logs
- 输出本地可直接登录的账号和 open API demo token

## 8. 代码质量与 TDD 基线

后端检查命令：

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo nextest run --workspace`
- `cargo llvm-cov --workspace --lcov --output-path target/lcov.info`
- `cargo sqlx prepare --check`
- `just openapi-check`
- `just perf-smoke`

前端检查命令：

- `pnpm lint`
- `pnpm format:check`
- `pnpm typecheck`
- `pnpm test`
- `pnpm test:e2e`

## 9. 首个里程碑

满足以下条件就算脚手架完成：

- Rust 服务可以启动
- PostgreSQL 可以完成迁移与 seed
- PostgreSQL 回滚命令约定清晰且可执行
- Vue 页面可以访问
- `/api/healthz` 正常
- 静态资源可由后端托管
- 可以登录默认管理员
- 可以创建一个项目
- 可以创建一个配置文件
- 可以创建一个部署实例
- 可以从模板克隆一个部署实例
- 可以保存一份 Draft
- Draft 乐观锁冲突可以返回 `409`
- 可以通过 HTTP 拉取一份已发布配置
- 可以通过 HTTP 拉取一份整部署实例配置包
