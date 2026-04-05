# 仓库初始化清单

## 1. 文档目标

这份清单用于指导你在 Linux 主机或 WSL2 中重新拉取 `mini-conf` 后，按顺序完成真正的工程初始化。

目标是：

- 减少遗漏
- 减少环境差异导致的返工
- 让后续执行开发工作的 AI agent 更容易按正确顺序落地

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

参考文档：

- [docs/DEV_LINUX_WSL2.md](c:\Users\zhaoj\Projects\mini-conf\docs\DEV_LINUX_WSL2.md)

### 3. 初始化前端工作区

- 执行 `pnpm install`
- 初始化 `apps/web`
- 补齐前端脚本：`lint`、`format:check`、`typecheck`、`test`、`test:e2e`

参考文档：

- [docs/FRONTEND_WORKSPACE.md](c:\Users\zhaoj\Projects\mini-conf\docs\FRONTEND_WORKSPACE.md)

### 4. 初始化 Rust workspace

- 创建根 `Cargo.toml`
- 创建 `apps/server`
- 创建 `crates/domain`
- 创建 `crates/infra`
- 创建 `crates/schema`

参考文档：

- [docs/BOOTSTRAP.md](c:\Users\zhaoj\Projects\mini-conf\docs\BOOTSTRAP.md)

### 5. 初始化数据库与迁移

- 配置 `DATABASE_URL`
- 配置 `DATABASE_ADMIN_URL`
- 创建第一批 migrations
- 建立 seed 逻辑

参考文档：

- [docs/DB_SCHEMA.md](c:\Users\zhaoj\Projects\mini-conf\docs\DB_SCHEMA.md)

### 6. 落地开放消费端最小协议

优先实现：

- `GET /api/open/configs/resolve`
- `GET /api/open/releases/:revision`
- `GET /api/open/deployments/:deploymentKey/config-bundle`
- `POST /api/open/deployment-sync-records`

参考文档：

- [docs/CLIENT_HTTP_PROTOCOL.md](c:\Users\zhaoj\Projects\mini-conf\docs\CLIENT_HTTP_PROTOCOL.md)

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

- [docs/ADMIN_API.md](c:\Users\zhaoj\Projects\mini-conf\docs\ADMIN_API.md)

### 8. 接入质量门槛

- `just lint`
- `just test`
- `just perf-smoke`
- `lefthook install`

参考文档：

- [docs/PERFORMANCE.md](c:\Users\zhaoj\Projects\mini-conf\docs\PERFORMANCE.md)

### 9. 验证 CI

- 确认 GitHub Actions 工作流语法正确
- 确认本地 `just ci-local` 可跑

## 3. MVP 初始化完成标准

满足以下条件，可认为初始化阶段完成：

- Rust workspace 已建立
- 前端 workspace 已建立
- PostgreSQL 迁移可执行
- 管理端最小登录可跑
- 开放消费端最小协议可跑
- `just lint`、`just test`、`just perf-smoke` 可执行
- GitHub Actions 可触发

## 4. 不要过早做的事

在初始化阶段，不建议过早做这些：

- 复杂 Scope / labels 动态匹配
- JWT 完整实现
- OAuth 2.0 接入
- 模板同步更新
- 深度性能优化

这些都已经在后续规划中记录，先把 MVP 主路径跑通更重要。
