# mini-conf Kickoff

## 1. 项目定位

建设一套轻量、开源、可私有部署的在线配置平台，核心目标是：

- 让消费端通过简单 HTTP 请求完成配置拉取，不强制引入重量 SDK
- 支持文本配置集中管理、校验、发布、版本历史、Diff 与审计
- 第一阶段优先服务 IoT / 边缘设备场景，但平台模型不绑定单一业务
- 面向服务端应用、定时任务、桌面程序、CLI 工具、边缘节点等通用在线配置场景
- 以 Linux / WSL2 作为真实开发与运行环境，当前 Windows 仓库仅用于设计和计划

一句话定位：

- `mini-conf` 是一个 API-first、私有部署友好、适合作为开源项目演进的轻量在线配置平台

## 2. 产品原则

- OpenAPI First，先定义消费协议，再实现管理端
- Clarification First，涉及产品语义变化时，先更新计划/澄清文档并单独提交，再开始代码实现
- SDK Optional，平台提供官方最小接入示例，但不要求业务方接入重量客户端
- Release Immutable，发布产物不可变，消费端永远拉取已发布版本
- Deployment First，MVP 通过“部署实例”建模一整套配置，而不是让用户先理解复杂 Scope 规则
- Model Extensible，后续版本支持在创建项目时选择配置组织模型
- Schema Guarded，配置内容支持 schema 校验与发布前检查
- Secret Aware，MVP 支持敏感配置脱敏展示和日志脱敏
- Self-host Friendly，默认私有部署、低依赖、易初始化
- Linux First，所有实际开发、测试、CI、部署均以 Linux / WSL2 为准

## 3. 技术选型

前端：

- Vue 3
- Vite
- TypeScript
- Element Plus
- Monaco Editor

后端：

- Rust
- Axum
- Tokio
- SQLx
- PostgreSQL 16+
- tower-http
- tracing
- utoipa

部署：

- 单后端服务提供 `/api` 和前端静态资源
- 外层使用 Caddy 或 Nginx
- 支持环境变量配置
- 启动时自动完成数据库迁移和基础 seed
- 数据库自动创建仅作为开发 / 初始化辅助能力，生产环境默认不强依赖

## 4. 仓库目标结构

```text
mini-conf/
  apps/
    web/
    server/
  crates/
    domain/
    schema/
    infra/
  migrations/
  scripts/
  deploy/
  docs/
  .github/workflows/
  justfile
  LICENSE-MIT
  LICENSE-APACHE
  NOTICE
  THIRD_PARTY_NOTICES.md
  README.md
  KICKOFF.md
```

## 5. 首版范围

首版必须完成：

- 登录
- 项目管理
- 项目级成员与权限管理
- 配置文件管理
- Deployment Instance 管理
- 从模板克隆部署实例
- Draft 编辑
- Draft 并发冲突检测
- Release 发布
- 版本历史
- Diff 查看
- 消费端查询当前版本
- 消费端拉取单配置文件内容
- 消费端拉取整部署实例配置包
- 消费端上报拉取 / 应用结果
- PostgreSQL 迁移与基础 seed
- PostgreSQL 迁移回滚命令与约定
- 默认管理员初始化
- 提供 `curl` / HTTP 示例，证明无需重量 SDK 也能接入

首版不做：

- 复杂审批流
- 细粒度 RBAC
- 实时推送配置
- 多租户隔离
- 插件系统
- 在线修改运行态状态
- 强依赖注册中心或消息队列
- 复杂动态 Scope DSL

## 6. 核心领域模型

需要先定义清楚这些概念：

- Project
- ProjectMember
- ConfigFile
- DeploymentInstance
- DeploymentCredential
- Draft
- Release
- DeploymentSyncRecord
- User
- AuditLog

核心约束：

- 一个 Project 下可以有多个 ConfigFile
- 一个 Project 下可以有多个 DeploymentInstance
- 一个 DeploymentInstance 属于一个明确的 Environment
- 一个 DeploymentInstance 持有该 Project 下多份 ConfigFile 在本实例上的配置版本
- 一个 ConfigFile 可以声明敏感配置元数据，用于脱敏展示和日志裁剪
- Release 不可变
- Draft 可反复保存
- Draft 保存采用乐观锁，避免多人编辑时静默覆盖
- 消费端只认发布后的 Release
- 一个实例可以被标记为模板，并被其他实例克隆
- 一个实例可供多个进程使用同一份实例级凭证访问平台
- MVP 阶段项目默认使用 `DeploymentInstance` 模型
- 后续版本支持在创建项目时选择配置组织模型，但模型创建后默认不可直接切换

## 7. 配置格式与消费策略

首版支持：

- YAML
- JSON
- TOML

统一策略：

- 文本格式只负责承载配置
- 前后端统一走 schema 校验
- 后端负责最终校验
- 平台维护配置元数据、变更等级和发布说明
- 不把消费端加载语义硬编码进用户配置文本结构

## 7.1 设计澄清工作流

当出现产品细节澄清、规则调整、接口语义收敛时，统一遵循：

1. 先更新 `KICKOFF.md` / 相关计划文档
2. 在 `docs/product-qa/` 记录本次澄清 Q&A 和当前实现选择
3. 单独提交一条“文档澄清/计划更新” commit
4. 再开始代码修改

目的：

- 让设计变化先于实现落地
- 让“当前代码为什么这样做”可追溯
- 避免讨论结论只留在聊天记录中

消费端接入策略：

- 不强制 SDK
- 提供基于 HTTP 的最小协议
- 首版使用轮询 + `ETag / If-None-Match`
- 首版主路径按“部署实例 + 配置文件”解析
- 由客户端自己决定兜底策略
- 后续可扩展长轮询、Webhook、SSE，但不进入 MVP

建议首版开放接口语义：

- `GET /api/open/configs/resolve`
- `GET /api/open/releases/:revision`
- `GET /api/open/deployments/:deploymentKey/config-bundle`
- `POST /api/open/deployment-sync-records`

最小请求要素：

- `project`
- `environment`
- `deployment_key`
- `config`
- `process_key`
- `Authorization: Bearer <token>`

## 8. 前端工作清单

基础工程：

- 初始化 Vue 3 + Vite + TypeScript
- 集成 Element Plus
- 集成 Vue Router
- 集成 Pinia
- 集成 Monaco Editor
- 封装 Monaco DiffEditor

页面：

- 登录页
- 项目列表页
- 项目成员页
- 配置文件列表页
- 部署实例列表页
- Draft 编辑页
- 发布确认页
- Release 历史页
- Diff 对比页
- 部署实例同步记录页

前端能力：

- YAML / JSON / TOML 编辑
- 语法高亮
- 格式化
- schema 校验提示
- 敏感配置默认脱敏展示
- 发布前 Diff 展示
- 版本切换查看
- 部署实例克隆
- API 调试示例展示

## 9. 后端工作清单

基础工程：

- 初始化 Rust workspace
- 创建 `apps/server`
- 创建 `crates/domain`
- 创建 `crates/schema`
- 创建 `crates/infra`

基础能力：

- Axum 路由骨架
- 统一错误处理
- tracing 日志
- 配置加载
- 健康检查接口
- 静态资源托管
- OpenAPI 文档与导出检查

数据库：

- 接入 PostgreSQL
- SQLx 连接池
- migrations
- migration rollback 约定
- 启动时检测数据库连通性
- 在显式开启引导参数时可自动创建数据库
- 自动执行建表迁移
- 自动执行 seed

接口：

- 用户登录
- 项目 CRUD
- 项目成员管理
- 配置文件 CRUD
- 部署实例 CRUD
- 部署实例模板克隆
- Draft 保存与读取
- 发布 Release
- Release 历史
- Diff 查询
- 部署实例凭证重置
- 消费端查询当前版本
- 消费端拉取配置
- 消费端上报同步记录
- 心跳上报

## 10. 数据库初始化策略

要求：

- 新环境只需配置数据库连接信息即可启动服务
- 项目首次启动自动检测目标数据库
- 如果显式提供管理员连接串并开启引导开关，可自动创建数据库
- 表不存在时自动迁移
- 初始化默认管理员账号
- 初始化默认 Project、默认环境和一个示例模板部署实例

建议环境变量：

- `APP_ENV`
- `HTTP_ADDR`
- `DATABASE_URL`
- `DATABASE_ADMIN_URL`
- `INIT_DB_ON_BOOT`
- `INIT_ADMIN_USERNAME`
- `INIT_ADMIN_PASSWORD`
- `STATIC_DIR`

## 11. 首版表设计清单

第一批表：

- `users`
- `projects`
- `project_members`
- `config_files`
- `deployment_instances`
- `drafts`
- `releases`
- `deployment_credentials`
- `deployment_sync_records`
- `audit_logs`

第一批关键字段：

- `projects.code`
- `config_files.code`
- `deployment_instances.environment`
- `deployment_instances.deployment_key`
- `deployment_instances.is_template`
- `deployment_instances.template_source_id`
- `drafts.content`
- `drafts.version`
- `releases.revision`
- `releases.content_hash`
- `deployment_credentials.token_hash`
- `deployment_sync_records.status`

## 12. 版本与发布规则

发布流程：

- 编辑 Draft
- 保存 Draft
- 校验通过后允许发布
- 发布时生成 revision
- 计算 content hash
- 记录发布说明
- 生成与上一版的 Diff
- DeploymentInstance 当前版本指向新 Release
- 即使 Draft 内容与上一版相同，重复发布也生成新 revision
- Draft 保存必须带上当前版本号或等价条件，版本冲突返回 `409 Conflict`

消费端同步规则：

- 消费端轮询当前版本
- 发现 revision 变化后拉取新配置
- 平台返回内容和元数据
- 如果部署实例未命中配置，明确返回失败
- 客户端按自己的加载机制决定热更新、重启或本地兜底
- 应用结果回传平台

## 13. 安全与权限

首版权限以项目级成员为主：

- admin
- editor
- viewer

管理端认证：

- Session Cookie
- JWT

约束：

- MVP 只完整实现 Session Cookie 方案
- 代码层面预留 JWT 扩展点
- 后续版本补齐 JWT 与 OAuth 2.0 接入
- 这一点要在 release note 中明确说明

消费端鉴权：

- deployment token
- 默认长期有效
- 支持手动重置和吊销

基础安全要求：

- 密码哈希存储
- 登录态过期
- 接口审计日志
- 发布操作留痕
- 开放接口限流
- token 仅存 hash，不回存明文
- 敏感配置默认脱敏展示，日志与审计详情不得记录明文

## 14. 自动化质量与 TDD 工作流

为避免后续进入 vibe coding 后代码质量失控，首版就把质量门槛和工作流定下来：

后端质量工具：

- `cargo fmt`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `cargo nextest run`
- `cargo llvm-cov`
- `sqlx prepare --check`
- `just openapi-check`
- `just perf-smoke`

前端质量工具：

- `eslint`
- `prettier`
- `vue-tsc --noEmit`
- `vitest`
- `playwright`

工程自动化：

- 使用 `justfile` 统一本地命令入口
- 使用 `lefthook` 或 `pre-commit` 执行提交前检查
- 使用 GitHub Actions 执行 lint、test、coverage、build
- 使用可回滚 migration 约定，并在本地保留回滚命令
- 使用 OpenAPI 导出检查前后端契约是否漂移
- 使用性能 smoke scaffold 为后续性能测试留出入口

TDD 约束：

- 新增领域逻辑先写失败测试，再补实现
- 发布流程、部署实例克隆、版本解析必须有集成测试
- 开放接口至少覆盖契约测试和 `curl` 级别的最小接入示例
- Draft 并发冲突必须有回归测试
- 修 bug 先补回归测试，再修复

## 15. 开发环境策略

当前阶段约束：

- Windows 仓库只用于产品设计、技术选型和文档沉淀
- 真正编码、联调、测试、CI 均以 Linux 或 WSL2 为准
- 所有自动化脚本优先提供 shell / `just` 版本，不以 PowerShell 为主
- 数据库、后端、前端的本地开发方式都要可在 WSL2 中直接重建

## 16. 开源与许可

目标：

- 支持商用
- 支持私有部署
- 降低法务阻力
- 避免引入强 copyleft 风险依赖

建议项目许可：

- MIT OR Apache-2.0

需要补齐：

- `LICENSE-MIT`
- `LICENSE-APACHE`
- `NOTICE`
- `THIRD_PARTY_NOTICES.md`

## 17. 第一周任务拆解

Day 1:

- 创建仓库基础目录
- 初始化 Rust workspace
- 初始化 Vue 工程
- 写 README 和 KICKOFF
- 补齐 `justfile` 与基础 CI 草案

Day 2:

- 接入 Axum、SQLx、PostgreSQL
- 跑通 health check
- 跑通静态资源托管
- 建立 migrations 机制
- 预留 migration rollback 命令
- 接入 lint / test 基础命令

Day 3:

- 建第一批核心表
- 完成数据库初始化与迁移
- 完成默认管理员 seed
- 建立后端测试基座

Day 4:

- 完成项目、配置文件、部署实例基础接口
- 完成前端列表和表单页骨架
- 为部署实例克隆补测试

Day 5:

- 接入 Monaco Editor
- 完成 Draft 编辑与保存
- 完成 Release 发布基础流程
- 提供最小 HTTP 拉取示例

## 18. 验收标准

满足以下条件视为 Kickoff 阶段完成：

- 新机器在 Linux / WSL2 中拉代码后能一键启动开发环境
- 后端能自动执行 PostgreSQL 迁移和 seed
- 前端能打开并编辑配置文本
- 可以创建项目、配置文件、部署实例
- 可以从模板克隆部署实例
- 可以保存 Draft
- 可以发布 Release
- 可以查看版本历史和 Diff
- 可以通过简单 HTTP 请求按部署实例拉取当前配置
- 质量检查命令可以在本地和 CI 中稳定执行
- OpenAPI 契约检查和 migration rollback 命令已经纳入工程基线

## 19. 立即执行清单

开始开发前先完成：

- 创建本地仓库并拉取远程 `mini-conf`
- 确认 LICENSE 策略
- 确认 PostgreSQL 连接信息与 Linux / WSL2 开发环境
- 初始化 Rust workspace
- 初始化 Vue 3 + Vite 前端
- 设计 `justfile`、CI 和提交前检查
- 提交第一版目录骨架
- 提交第一版 README 和本 Kickoff 文档
