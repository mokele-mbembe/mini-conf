# 仓库初始化清单

> 注：这份文档主要服务于“从空仓或换机器恢复环境”的场景。当前 `mini-conf` 已经完成初始化并进入上线前收口阶段；继续开发时请优先看 `docs/agents/AGENT_START_HERE.md` 和 `DEVELOPMENT_LOG.md`。

## 1. 文档目标

这份清单用于指导你在 Linux 主机或 WSL2 中重新拉取 `mini-conf` 后，按顺序完成真正的工程初始化。

目标是：

- 减少遗漏
- 减少环境差异导致的返工
- 让后续执行开发工作的 AI agent 更容易按正确顺序落地

工作流规范以 [标准 Linux 开发工作流](../agents/STANDARD_WORKFLOW.md) 为准；本文只负责初始化顺序。

## 2. 初始化顺序

### 1. 拉取仓库

- 在 Linux 主机或 WSL2 Linux 文件系统中 clone 仓库
- 不建议长期在 `/mnt/c/...` 下做高频开发

### 2. 安装基础工具

- Rust stable
- `cargo-nextest`
- `cargo-llvm-cov`
- `sqlx-cli`
- Node.js 20+
- `pnpm`
- `just`
- PostgreSQL 16+
- 对 Fedora WSL，额外确认 `openssl` 命令行和 `openssl-devel` 都已安装

对当前已验证的 `Fedora Linux 43 (WSL)`，实际顺序是：

1. 先创建仓库外缓存目录和 `~/.config/mini-conf/dev-env.sh`
2. 再安装系统包、Rust 和 pnpm
3. 然后初始化 PostgreSQL，并把 `pg_hba.conf` 改成 `scram-sha-256`
4. 最后跑 `pnpm install`、`pnpm dlx lefthook install`、`just db-migrate-up`、`just test-backend-db`

参考文档：

- [docs/agents/STANDARD_WORKFLOW.md](../agents/STANDARD_WORKFLOW.md)
- [docs/agents/DEV_LINUX_WSL2.md](../agents/DEV_LINUX_WSL2.md)

### 3. 初始化前端工作区

- 执行 `pnpm install`
- 初始化 `apps/web`
- 补齐前端脚本：`lint`、`format:check`、`typecheck`、`test`、`test:e2e`

参考文档：

- [docs/collaboration/FRONTEND_TASK_WORKFLOW.md](./FRONTEND_TASK_WORKFLOW.md)

### 4. 初始化 Rust workspace

- 创建根 `Cargo.toml`
- 创建 `apps/server`
- 创建 `crates/domain`
- 创建 `crates/infra`
- 创建 `crates/schema`

参考文档：

- [docs/public/BOOTSTRAP.md](../public/BOOTSTRAP.md)

### 5. 初始化数据库与迁移

- 优先配置 `~/.config/mini-conf/dev-env.sh`，不要把 WSL 初始化绑定到 `secret-tool`
- 优先为每个场景单独建库，不把数据库名固定成产品名
- 运行时与迁移使用显式 `DATABASE_URL`
- 数据库集成测试使用显式 `TEST_DATABASE_URL`
- 本机开发便利变量交给 `scripts/local-db-env.sh` 解析
- 初始化 PostgreSQL 16 数据目录
- 把默认 `pg_hba.conf` 的 `peer` / `ident` 改成 `scram-sha-256`
- 创建应用用户与场景化数据库，例如 `mini_conf_dev` 或 `mini_conf_ci`
- 跑 `just db-migrate-up` 或 `just db-migrate-up-local`

参考文档：

- [docs/constraints/DB_SCHEMA.md](../constraints/DB_SCHEMA.md)
- [docs/agents/DEV_LINUX_WSL2.md](../agents/DEV_LINUX_WSL2.md)

### 6. 落地开放消费端最小协议

优先实现：

- `GET /api/open/configs/resolve`
- `GET /api/open/releases/:revision`
- `GET /api/open/deployments/:deploymentKey/config-bundle`
- `POST /api/open/deployment-sync-records`

参考文档：

- [docs/public/CLIENT_HTTP_PROTOCOL.md](../public/CLIENT_HTTP_PROTOCOL.md)

### 7. 落地管理端主干接口

优先实现：

- 登录
- 项目管理
- 项目成员管理
- 配置文件管理
- 部署实例管理
- 模板克隆
- Draft
- Release

参考文档：

- [docs/constraints/ADMIN_API.md](../constraints/ADMIN_API.md)

### 8. 接入质量门槛

先完成 `Core`：

- `just lint`
- `just test`
- `just openapi-check`

再补 `Isolated DB`：

- `just db-migrate-up`
- `just test-backend-db`

最后补 CI / 进阶检查：

- `just perf-smoke`
- `just sqlx-check`
- `pnpm dlx lefthook install`

参考文档：

- [docs/agents/STANDARD_WORKFLOW.md](../agents/STANDARD_WORKFLOW.md)
- [docs/public/PERFORMANCE.md](../public/PERFORMANCE.md)

### 9. 验证 CI

- 确认 GitHub Actions 工作流语法正确
- 确认本地 `Core` 和 `Isolated DB` 命令都可跑
- 确认 GitHub `quality` 与 `backend-db` 两条工作流职责清晰

## 3. MVP 初始化完成标准

满足以下条件，可认为初始化阶段完成：

- Rust workspace 已建立
- 前端 workspace 已建立
- PostgreSQL 迁移可执行
- WSL 本地环境文件可被 `scripts/load-dev-env.sh` 自动读取
- PostgreSQL 回滚命令可执行
- `just test-backend-db` 在至少一套 Linux / WSL 环境里已实际跑通
- 管理端最小登录可跑
- 开放消费端最小协议可跑
- `just lint`、`just test`、`just perf-smoke`、`just openapi-check` 可执行
- GitHub Actions 可触发

## 4. 不要过早做的事

在初始化阶段，不建议过早做这些：

- 复杂 Scope / labels 动态匹配
- JWT 完整实现
- OAuth 2.0 接入
- 模板同步更新
- 深度性能优化

这些都已经在后续规划中记录，先把 MVP 主路径跑通更重要。
