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
STATIC_DIR=apps/web/dist
ADMIN_AUTH_MODE=session
JWT_ENABLED=false
OPENAPI_EXPORT_PATH=docs/openapi/openapi.json
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
- `GET /api/drafts/:deploymentId/:configFileId`
- `PUT /api/drafts/:deploymentId/:configFileId`
- `POST /api/releases/publish`
- `GET /api/releases`
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
- `just test-e2e`
- `just ci-local`
- `just db-reset-dev`

当前仓库约定：

- `just dev-server`
- `just db-migrate-up`
- `just db-migrate-down`
- `just test-backend-db`

会优先使用已设置的 `DATABASE_URL` / `TEST_DATABASE_URL`。
如果 `TEST_DATABASE_URL` 为空字符串，测试会自动回退到 `DATABASE_URL`。
如果未设置，则尝试从 `secret-tool lookup service mini-conf env dev role app-db user mini_conf` 读取开发库密码，并自动做 URL 编码后再连接本地 PostgreSQL。

数据库集成测试约定：

- 测试文件不要各自重复解析 `TEST_DATABASE_URL` / `DATABASE_URL`
- 统一复用 [`infra::testing`](/home/zjj/Projects/mini-conf/crates/infra/src/testing.rs) 中的 `test_database_url`、`unique_schema_name`、`with_search_path`
- 这样可以把 Linux / WSL2 / 本地 shell 的环境差异收口在一处，避免后续新增测试时再引入分叉

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
