# mini-conf

`mini-conf` 是一个面向部署实例的轻量在线配置平台。

它为边缘节点、门店机器、设备宿主机和多进程部署场景而设计：不要求业务方接入重量 SDK，只通过简单 HTTP 请求就能完成配置发现、版本检查、配置拉取和结果回传。

如果传统配置中心更偏“应用名 + namespace + SDK”，那么 `mini-conf` 更偏：

- `DeploymentInstance` 驱动
- 多配置文件
- 多进程共享凭证
- 模板克隆
- 整部署实例配置包拉取
- 私有部署友好

第一阶段会优先服务 IoT / 边缘设备场景，但平台模型会保持通用，适用于：

- 服务端应用
- 定时任务
- CLI 工具
- 桌面程序
- 边缘节点
- IoT 设备

## 为什么做它

很多配置中心产品默认假设：

- 业务方愿意接入专有 SDK
- 场景主要是服务端应用
- 依赖组件和部署成本可以接受

但对很多轻量项目、边缘节点和设备场景来说，更现实的诉求是：

- 私有部署简单
- 接入成本低
- 只靠 HTTP 就能用
- 配置发布可追踪、可回滚、可审计

`mini-conf` 想解决的就是这个问题。

## 项目目标

- 通过简单 HTTP 请求完成在线配置加载
- 支持多项目、多配置文件、多环境、多部署实例
- 支持部署实例模板克隆
- 支持 Draft / Release / Diff / 审计
- 支持基础格式合法性校验和发布前检查
- 支持敏感配置的最小安全语义，包括脱敏展示与日志脱敏
- 支持私有部署和开源演进
- 保持对 Linux / WSL2 开发和运行环境友好

## MVP 范围

首个 MVP 聚焦这几件事：

- 登录
- 项目管理
- 项目级成员管理
- 配置文件管理
- 部署实例管理
- 模板克隆
- Draft 编辑
- Draft 并发冲突检测
- Release 发布
- 版本历史与 Diff
- 消费端查询当前版本
- 消费端拉取单配置文件内容
- 消费端拉取整部署实例配置包
- 消费端上报同步结果
- 消费端上报心跳

首版不会做：

- 复杂审批流
- 细粒度 RBAC
- 多租户隔离
- 实时推送
- 插件系统
- 复杂动态 Scope 规则

## 核心设计

### 1. API-first

先定义消费协议，再做管理端功能。消费端的最小能力应该可以直接用 `curl` 演示，而不是先绑定某个 SDK。

### 2. Deployment-first

MVP 不让用户先理解复杂的 Scope 或 labels。我们使用 `DeploymentInstance` 表示项目在某个环境下的一套独立部署实例。

一个部署实例可以：

- 持有该项目下多份配置文件
- 从模板克隆
- 被多个进程共享同一份部署实例凭证访问

部署实例在 MVP 中只使用 `active / inactive` 两种状态。新建和从模板克隆出的实例默认 `inactive`，激活后才生成默认 token 并允许消费端访问；停用后默认 token 立即失效。

消费端配置标识统一使用配置文件标识：`ConfigFile.code`。Open API 请求里的 `config`、同步记录和心跳落库的 `config_file_id` 都指向同一份配置文件，MVP 不再引入独立 `process_key`。

### 3. Release 不可变

配置编辑发生在 Draft 阶段。真正提供给消费端的永远是已发布的不可变 Release。

### 4. Secret-aware

MVP 先支持敏感配置的最小安全语义：配置文件可以声明敏感属性，管理端默认脱敏展示，日志和审计详情不记录明文。

字段级加密存储会放到后续版本，不阻塞首版交付。

### 5. SDK optional

后续可以提供官方轻量客户端，但平台设计不会要求使用者必须引入重量 SDK。

### 6. Model-extensible

MVP 默认采用 `DeploymentInstance` 作为配置组织模型，因为它最贴合当前业务。

但后续版本会支持在创建项目时选择配置组织模型，而不是把整个平台永久锁定在一种组织方式上。

## 技术方向

当前规划中的技术选型：

- 前端：Vue 3、Vite、TypeScript、Element Plus、CodeMirror 6
- 后端：Rust、Axum、Tokio、SQLx、PostgreSQL
- 部署：单服务托管 API 和前端静态资源，外层使用 Caddy 或 Nginx

## 业务示例

以你的 `coffee-legacy` 场景为例：

- 平台里创建一个项目：`coffee-legacy`
- 这个项目下有 3 份配置文件：`main`、`ad-screen`、`vision`
- 在 `prod` 环境下，可以创建多个部署实例，例如 `store-001`、`store-002`
- 每个部署实例都持有这 3 份配置文件各自的一套配置
- 新店上线时，可以从模板实例克隆一整套配置
- 同一台机器上的主进程、广告屏客户端、视觉服务可以共享同一份凭证访问平台
- 每个进程按自己的配置文件名拉取配置，或者一次拉取整部署实例配置包

这个例子是当前 MVP 设计的直接目标场景。

## 开发环境约束

实际编码、联调、测试和部署以 Linux 主机或本机 WSL2 实例为准。这意味着：

- 脚本优先提供 shell / `just` 版本
- 本地调试流程优先针对 Linux / WSL2 编写
- CI 也应以 Linux 环境为标准

标准工作流规范见 [docs/agents/STANDARD_WORKFLOW.md](./docs/agents/STANDARD_WORKFLOW.md)。

当前约定：

- 所有 Linux 协作者至少满足 `Core` 工作流：`just lint`、`just test`、`just openapi-check`
- MVP 发布前采用 `single-maintainer main-first`：单人开发默认直接在 `main` 上迭代，push 前至少跑 `just ci-local`，数据库或前端主链路改动优先跑 `just ci-local-full`
- 承担数据库主路径开发与 PR 级 PostgreSQL 集成测试的环境满足 `Isolated DB` 工作流：`just db-migrate-up`、`just test-backend-db`
- 共享黑盒环境与生产部署额外属于 `Blackbox / Staging` 和 `Production` 工作流，不复用开发机脚本
- `~/.config/mini-conf/dev-env.sh` 是唯一推荐的本机环境入口
- 默认采用 `database-per-instance`，数据库名由部署者按场景自定义；允许同一 PostgreSQL server 承载多套独立 database
- `secret-tool` 只是兼容选项，不是标准前提
- 本机 CI 入口分层为：`just ci-local` 负责非 DB 基线，`just ci-local-db` 对齐 GitHub `backend-db`，`just test-e2e-local` 以临时 schema 启动隔离前后端，`just ci-local-full` 串联三层
- 当前后端开发阶段，本机优先恢复 `just test-backend-db-local`；`just run-server-local` 只在确实需要联调时启用

当前管理台已把 deployment 配置工作入口收口到列表页：部署实例列表行可展开紧凑配置详情，主行直接打开 floating workspace；旧 deployment detail URL 仅作为兼容入口重定向回列表展开态。

## 生产发布包

MVP 生产部署主路径是 Linux binary 发布包，不把 PostgreSQL、DNS、TLS 或反向代理纳入项目编排。

生成发布包：

```bash
pnpm install --frozen-lockfile
just release-package
just release-package-check
```

默认产物是 `dist/mini-conf-linux-x86_64.tar.gz`，包含 `bin/mini-conf-server`、`web/`、`migrations/`、生产 env 示例和 systemd 示例。完整部署步骤见 [docs/runbooks/PRODUCTION_BINARY.md](./docs/runbooks/PRODUCTION_BINARY.md)。

GitHub Actions 的 `Release Package` workflow 可手动触发，也会在推送 `v*` tag 时上传同名 artifact。

真实 staging 或 production-like 环境部署后，可以用只读 smoke 验证入口：

```bash
STAGING_BASE_URL=https://config-center.example.com just staging-smoke
```

## 本机联调启动

如果你已经按仓库约定配置好了 `~/.config/mini-conf/dev-env.sh`，可以直接用下面这组命令启动前后端联调。

首次准备一次：

```bash
source scripts/local-db-env.sh
just db-migrate-up-local
just dev-seed-demo-local
```

后端终端：

```bash
source scripts/local-db-env.sh
just dev-server
```

前端终端：

```bash
just dev-web
```

启动后默认访问：

- 前端：`http://127.0.0.1:5173`
- 后端：`http://127.0.0.1:8080`
- 登录账号：`admin / admin123456`
- `just dev-db-prepare-local` 会把本地 demo 数据库标记为已完成 setup，便于直接进入管理台联调。

如果只是日常重启联调，通常只需要重新开两个终端执行：

```bash
source scripts/local-db-env.sh
just dev-server
```

```bash
just dev-web
```

查看或清理联调 runtime 库里的历史 `alpha-*` 项目：

```bash
just db-list-alpha-runtime-local
just db-clean-alpha-runtime-local
```

这组 runtime 清理命令只用于历史污染恢复。正常自动化测试不应再写入联调 runtime 库。

## 测试约定

数据库集成测试统一使用 [`infra::testing`](./crates/infra/src/testing.rs) 提供的 helper。

- 新增数据库集成测试时，使用 `test_database_url(...)` 解析测试连接串，不要在测试文件里重复读取环境变量
- Rust 测试代码只读取 `TEST_DATABASE_URL`
- portable 运行时和迁移命令只读取显式 `DATABASE_URL`
- 本机 local wrapper 通过 [`scripts/local-db-env.sh`](./scripts/local-db-env.sh) 解析开发机便利变量
- `TEST_DATABASE_URL` 不再由 `DATABASE_URL` 隐式补齐
- 当前 Rust 后端集成测试主要通过 in-process router `oneshot(...)` 执行，不依赖 `HTTP_ADDR` 或固定 `8080`
- 需要隔离 schema 时，使用 `unique_schema_name(...)`
- 需要按 schema 建连接时，使用 `with_search_path(...)`
- Alpha HTTP 与 Web E2E 都只允许使用 `TEST_DATABASE_URL` 派生临时 schema；`just test-e2e-local` 会自动启动隔离后端、隔离 Vite 和 Playwright
- 裸 `pnpm --dir apps/web test:e2e` 默认拒绝连接共享服务；如需手动打已有服务，必须显式设置 `E2E_ALLOW_SHARED_SERVER=1 PLAYWRIGHT_BASE_URL=...`

这样可以把多环境差异收口到一处，避免新增测试时遗漏空值回退或 search path 逻辑

## 文档导航

- 文档总索引与分类说明：[docs/README.md](./docs/README.md)
- 未完成工作索引与续工入口：[KICKOFF.md](./KICKOFF.md)
- AI agent 唯一续工入口：[docs/agents/AGENT_START_HERE.md](./docs/agents/AGENT_START_HERE.md)
- 单人 `main` 开发每日清单：[MAIN_DEV_CHECKLIST.md](./MAIN_DEV_CHECKLIST.md)
- 对外公开与使用侧文档：[docs/public/README.md](./docs/public/README.md)
- 项目约束与产品边界：[docs/constraints/README.md](./docs/constraints/README.md)
- AI agent 工作流与环境约定：[docs/agents/README.md](./docs/agents/README.md)
- 协作者与贡献流程文档：[docs/collaboration/README.md](./docs/collaboration/README.md)
- 生成产物与 OpenAPI 文件：[docs/artifacts/README.md](./docs/artifacts/README.md)

## 当前状态

项目目前已经完成后端 MVP 主链路和前端管理台核心业务链路，当前仓库重点转为：

- 保持后端权限、审计、配置标识和部署实例生命周期主链路稳定
- 用真实 staging 反馈校准部署 runbook、初始化交付和生产变量清单
- 继续补 alpha 黑盒、覆盖率和前端单元 / 组件测试基线
- 继续推进 Config Workspace 的 release/history 右栏和 merge workspace 后续增强
- 继续固化产品边界、协作流程与部署约定并压缩过时入口

当前前端基线已经不是“待初始化”状态，而是：

- `apps/web` 已存在并可运行
- 已有登录、setup、首次改密、平台用户管理、平台项目创建
- 已有项目列表、配置文件、环境、部署实例列表展开、floating workspace、Draft、Saved Versions、preview、publish、Release detail/diff
- 已有 deployment archive / restore / permanent delete 主路径
- 已有 projects / config_files 删除能力与引用检查
- 已有项目成员、sync records、heartbeats、audit logs 页面
- 已接入前端 `lint / format:check / typecheck / build`
- 已接入覆盖核心管理链路的 Playwright E2E
- 前端协作入口已收口到 [FRONTEND_TASK_WORKFLOW.md](./docs/collaboration/FRONTEND_TASK_WORKFLOW.md)

## 后续优先事项

建议按这个顺序推进：

1. 文档同步和入口压缩，保持 README / KICKOFF / DEVELOPMENT_LOG / constraints 与真实实现一致
2. 上线实施方案：Linux binary 发布包、外部 PostgreSQL、`config-center.example.com` 示例入口域名、反向代理/TLS、初始化和生产变量清单
3. 前端单元 / 组件测试补量和更完整页面级 E2E，优先覆盖 deployment list / workspace overlay / Draft 历史面板
4. Config Workspace 后续增强：Release/history 右栏、Merge Workspace
5. 覆盖率持续补量和 `sqlx-check` 恢复时机评估

## License

计划使用：

- MIT OR Apache-2.0

相关文件后续补齐：

- `LICENSE-MIT`
- `LICENSE-APACHE`
- `NOTICE`
- `THIRD_PARTY_NOTICES.md`
