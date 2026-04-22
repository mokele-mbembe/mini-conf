# 开发记录与会话交接

## 1. 文档目的

这份文档用于记录当前开发进度，并为下一个会话提供直接可执行的起手上下文。

适用场景：

- 隔一段时间后重新进入项目
- 切换机器继续开发
- 新开会话时快速恢复上下文

## 1.1 最近完成

2026-04-21 本轮完成 Release 回看、部署实例分区、归档 / 删除生命周期闭环，并提交 `c7563f1 Add deployment archive and deletion lifecycle`：

- Release 详情页和 Diff 页已从占位页补为真实页面，可从发布历史进入只读内容回看和上一版差异回看
- Release 详情 / Diff 已接入后端脱敏返回，页面以只读文本框展示内容，并显式展示发布时间、revision、发布人 ID、配置和实例上下文
- 部署实例列表已拆成“模板”和“部署实例”两个区块，前端分别请求 `is_template=true/false`，各自拥有分页、搜索、loading 和空态
- 部署实例归档 / 恢复 / 永久删除已落地：
  - `status` 仍只表达运行态 `active | inactive`
  - `is_archived` 表达软归档，归档后默认列表隐藏，可在“已归档实例”抽屉中恢复
  - `deleted_at` 表达不可恢复 tombstone 删除，删除后释放 `deployment_key`
  - `deployment_uid` 作为底层稳定实体身份，用于区分同 key 不同生命期
- 前端已补“已归档实例”抽屉、详情页归档提示、恢复和永久删除确认；永久删除前要求输入 `deployment_key`
- 后端已补 `visibility_filter=current|archived|all`、archive / restore / delete API、并发状态防护、OpenAPI 导出和集成测试
- Draft clone / publish / preview / activate / token reset 等工作流已对 archived / deleted 实例补 guard
- 本轮验证已通过 `just ci-local-full`，其中后端 DB 集成测试 277 条、Playwright E2E 8 条均通过

2026-04-20 本轮完成 Draft / preview-bundle / publish 主路径文档复核，并补充后续体验改造规划：

- 当前前端已基本打通 `Current Draft -> preview-bundle -> publish -> release history` 主闭环
- Saved Versions 后端和前端历史面板均已接入，保存 Current Draft 后会生成历史保存版本
- 单配置 clone 已改为专用 clone-sources 查询，支持可用来源元数据、远程搜索、分页加载和 E2E 覆盖
- 当时识别的 Release 详情 / Diff、模板与普通实例拆分、deployment archive / delete 缺口，已在 2026-04-21 批次落地
- 新增产品澄清 `docs/constraints/product-qa/0010-release-readonly-template-split-and-deployment-archive.md`

2026-04-20 本轮已完成 Saved Versions 后端初版落地，并补了对应的后端测试骨架：

- 新增迁移 `0014_saved_versions`
- 新增 `draft_saved_versions` 表，`content_hash` 使用 `VARCHAR(64)` 并带长度检查
- Current Draft 保存链路现在会自动生成 Saved Version；与最近一条相同内容时不重复生成
- 新增 `GET /api/draft-saved-versions`
- 新增 `GET /api/draft-saved-versions/:id`
- 新增 `PATCH /api/draft-saved-versions/:id`
- 新增 `POST /api/draft-saved-versions/:id/restore`
- 新增 `DELETE /api/draft-saved-versions/:id`
- 已补 `crates/schema::saved_version`
- 已补 `apps/server/tests/saved_versions.rs`
- 已补 `apps/server/tests/drafts.rs` 的 Saved Version 回归断言

本轮审查后确认：

- `cargo test -p server --test saved_versions` 通过
- `cargo test -p server --test drafts` 通过
- 后续已修复 Saved Versions 列表 viewer 可见性和 `saved_version.updated` 审计 detail 字段问题
- 前端已接入 Saved Versions 历史面板，可查看、备注、恢复和删除历史保存版本

2026-04-20 本轮已完成部署实例页前端加载优化与缓存/竞态收口：

- 路由页面已改为懒加载，降低管理台主包首屏压力
- `DeploymentInstanceListPage` 首屏加载已从串行请求改为并行请求
- 部署实例列表页已拆分 header skeleton、环境筛选 loading 和表格局部 loading，减少整页阻塞感
- 新增 `useProjectEnvironments` composable，统一项目环境列表加载、排序与 30 秒 TTL 缓存
- `DeploymentInstanceCreateDialog` 与 `DeploymentInstanceCloneDialog` 已复用项目环境缓存，不再重复拉取环境列表
- `useProjectContext` 已改为模块级 stale-while-revalidate 缓存，并补 `_requestSeq` 防止跨项目快速切换时旧请求覆盖新页面
- 已修复缓存优化过程中暴露的三个问题：readonly ref 写入、环境缓存绑定初始 `projectId`、项目详情异步响应竞态
- 部署实例列表 keyword 搜索已补 300ms debounce，减少连续输入时的重复请求

本轮本地验证结果：

- `pnpm --dir apps/web typecheck` 通过
- `pnpm --dir apps/web exec eslint . --ext .vue,.ts,.tsx` 通过
- `pnpm --dir apps/web build` 通过
- Vite 生产构建已完成页面级拆包；当前仍保留 `index` 主 chunk 体积 warning，但不阻塞本地与 CI 构建通过

2026-04-10 本轮已完成后端权限与审计主线收口，并补完后端补强批次的主路径：

- 新增 `project_members` 与 `audit_logs` 迁移，并补历史项目给活动用户 `admin` 的成员回填
- 管理端资源访问已从“登录即管理员”切换为项目成员角色模型
- 新增 `GET /api/projects/{id}/members`、`POST /api/projects/{id}/members`、`PUT /api/projects/{id}/members/{memberId}`、`DELETE /api/projects/{id}/members/{memberId}`
- 新增 `GET /api/deployment-sync-records` 与 `GET /api/audit-logs`
- 现有项目、配置文件、部署实例、Draft、Release、token reset、登录流程已补审计日志
- OpenAPI、数据库文档、鉴权文档、产品澄清文档已经同步
- `docs/` 已按受众重组为 `public / constraints / agents / collaboration / artifacts`
- OpenAPI 导出产物已从 markdown 叙述目录中分离到 `docs/artifacts/openapi.json`
- 本机 CI 已新增 `just ci-local-db` 与 `just ci-local-full`，用于把 GitHub `backend-db` 校验纳入统一入口；`ci-local-db` 在缺少运行库配置时会复用 local test DB
- 新增 `INIT_USERS_FILE` 启动 seed，支持 JSON / YAML 导入多用户与项目成员绑定；alpha HTTP 默认接入 `tests/alpha/users.seed.yaml`
- 管理端列表已补 `projects/config-files` 的 `status` 过滤和 `deployment-instances` 的 `keyword` 过滤
- open consumer 侧 `resolve / release / config-bundle / sync-record / heartbeat` 已统一只接受 `project/config/deployment` 均为 `active` 的资源
- 新增 `GET /api/deployment-heartbeats`
- Draft 保存、clone 与发布前已接入 `yaml / json / toml` 基础格式合法性校验；MVP 不再暴露独立业务规则校验器能力
- 管理端 `release detail / diff` 已对 secret 配置做脱敏返回，并返回 redaction 标记字段
- `alpha-full` 已补多用户 seed、项目成员、同步记录、心跳、模板 clone、二次发布 diff、旧 token 失效回归
- 本机与 CI 已补后端覆盖率基线入口：`just coverage-check`
- 配置文件契约已完成产品语义收口：`text` 拒绝写入，TOML 全链路支持，`config_files.status` 收口为 `active | archived`，schema 字段从表结构、接口、OpenAPI、seed 和验收资产中移除

本轮本地验证结果：

- `cargo test --workspace` 通过
- `just lint-backend` 通过
- `docs/artifacts/openapi.json` 已重新导出

2026-04-13 本轮已完成前端首版 scaffold、前端 CI 基线和前端协作文档收口：

- 新增 `apps/web`，完成 `Vue 3 + Vite + TypeScript + Pinia + Vue Router + Element Plus` 首版管理台骨架
- 已实现登录页、项目列表页、项目详情骨架页，以及基础 `AppShell / AuthLayout / ProjectLayout`
- 已接通 `POST /api/auth/login`、`GET /api/auth/me`、`POST /api/auth/logout`、`GET /api/projects`、`GET /api/projects/{id}`
- 已补 `loading / empty / error / forbidden / not-found` 通用状态组件
- 已修复登录页无限重定向、Vite 代理端口错误、`format:check` 被 `dist/` 污染等联调问题
- 新增 `docs/collaboration/FRONTEND_PAGE_TESTING.md`，记录本地页面测试顺序、白屏排查方法和前端 smoke 复现方式
- 新增 `docs/collaboration/FRONTEND_TASK_WORKFLOW.md`，用于记录前端续工流程和统一 kickoff prompt
- 已更新 `FRONTEND_HANDOFF / FRONTEND_WORKSPACE / QUALITY_CHECK_PLAN / docs/collaboration/README.md`，与当前前端真实状态对齐
- GitHub Actions 已接入前端 `build` 检查和最小 Playwright smoke E2E
- 本地已按接近 CI 的方式跑通 `login -> projects -> project detail` smoke 主路径

本轮本地验证结果：

- `pnpm --dir apps/web build` 通过
- `pnpm --dir apps/web run format:check` 通过
- `pnpm --dir apps/web typecheck` 通过
- `PLAYWRIGHT_BASE_URL=http://127.0.0.1:4173 pnpm --dir apps/web test:e2e` 通过
- 浏览器手工联调已确认 `/login -> /projects -> /projects/:id` 主路径可访问，Console 无新错误

2026-04-15 本轮已完成配置标识收口与部署实例生命周期调整，并提交 `8e28eae Align config identity and deployment lifecycle`：

- MVP 后端主路径不再引入独立 `process_key`
- open API、sync records、heartbeats 统一使用请求字段 `config`，服务端按 `ConfigFile.code` 解析并落库为 `config_file_id`
- `deployment_sync_records` 删除 `process_key`，管理端列表返回 `config_file_id` 和 `config`
- `deployment_heartbeats` 改为 `config_file_id`，唯一约束为 `deployment_instance_id + config_file_id`
- 新增迁移 `0012_config_identity_and_deployment_lifecycle`
- 部署实例状态收口为 `active | inactive`，不再为 deployment 引入 `archived`
- 创建和模板 clone 默认生成 `inactive` 普通实例，不生成 token
- 新增 `POST /api/deployment-instances/:id/activate`，激活普通实例并一次性返回默认 token
- 新增 `POST /api/deployment-instances/:id/deactivate`，停用实例并让默认 token 立即失效
- `POST /api/deployment-instances/:id/token/reset` 仅允许 active 普通实例
- `PUT /api/deployment-instances/:id` 只允许修改 `environment / deployment_key / name / description`
- `GET /api/deployment-instances` 已改为分页响应 `items / total / page / page_size`
- OpenAPI、alpha full Hurl、demo seed、DB/API/前端蓝图/客户端协议文档已同步
- 新增咖啡中间件 demo 规格文档 `docs/constraints/DEMO_SCENARIO_COFFEE_MIDDLEWARE.md`
- 新增产品澄清 `docs/constraints/product-qa/0007-config-identity-and-heartbeats.md`

本轮本地验证结果：

- `cargo test --workspace` 通过
- `just lint-backend` 通过
- `just test-backend-db-local` 通过，197 passed
- `cargo check --workspace` 通过
- `cargo test -p server --test deployment_tokens` 通过
- `bash scripts/export-openapi.sh` 已重新导出 `docs/artifacts/openapi.json`
- 提交时 pre-commit hook 已通过 `fmt-backend / lint-backend / fmt-frontend / lint-frontend`

## 2. 当前进度 Checklist

### 2.1 基础设施与工程基线

- [x] Rust workspace 初始化完成
- [x] Axum 服务可启动
- [x] `GET /api/healthz`
- [x] PostgreSQL migrations 接入
- [x] 本地开发库辅助脚本与 `just` 入口
- [x] OpenAPI 导出与 `/swagger-ui`
- [x] 静态资源托管
- [x] GitHub Actions 基本跑通
- [x] 质量检查计划文档
- [x] WSL / Fedora 双环境工具清单

### 2.2 Open API

- [x] `GET /api/open/configs/resolve`
- [x] `GET /api/open/releases/:revision`
- [x] `GET /api/open/deployments/:deploymentKey/config-bundle`
- [x] `POST /api/open/deployment-sync-records`
- [x] `POST /api/open/heartbeats`
- [x] deployment credential / Bearer 鉴权
- [x] open API 客户端配置标识统一为 `config`

### 2.3 管理端认证

- [x] `POST /api/auth/login`
- [x] `GET /api/auth/me`
- [x] `POST /api/auth/logout`
- [x] 默认管理员 seed
- [x] session cookie

### 2.4 管理端资源

- [x] `projects` CRUD
- [x] `project_members` CRUD
- [x] `config-files` CRUD
- [x] `config_files.is_required`
- [x] `deployment-instances` CRUD
- [x] template clone 创建新实例
- [x] 整实例 preview-bundle
- [x] `drafts` 的 `GET / PUT`
- [x] 单配置文件 clone
- [x] `releases/publish`
- [x] `GET /api/releases`
- [x] `GET /api/releases/:id`
- [x] `GET /api/releases/:id/diff`
- [x] `POST /api/deployment-instances/:id/token/reset`
- [x] `POST /api/deployment-instances/:id/activate`
- [x] `POST /api/deployment-instances/:id/deactivate`
- [x] `GET /api/deployment-sync-records`
- [x] `GET /api/deployment-heartbeats`
- [x] `GET /api/audit-logs`

### 2.5 产品规则收口

- [x] Template 仍然是实例概念
- [x] Template 禁止发布 release
- [x] 项目层支持必选配置文件
- [x] 发布单配置前检查实例是否已具备全部必选配置
- [x] 模板 clone 仅允许从 draft
- [x] 单配置 clone 支持 `draft | latest_release`
- [x] preview-bundle 返回业务预览明细和 consumer 侧整包预览
- [x] `release diff` 固定比较上一版并返回文本级摘要
- [x] token reset 原地轮换默认凭证并立即切换 open API 鉴权
- [x] 部署实例创建和 clone 默认 `inactive`
- [x] 部署实例只采用 `active | inactive`
- [x] active 实例才允许 open API 消费
- [x] `ConfigFile.code` 是 MVP 唯一客户端配置标识
- [x] 项目仅对成员可见
- [x] 项目创建者自动成为项目 `admin`
- [x] 写操作和关键认证事件写入 `audit_logs`
- [x] 管理端资源访问收口到项目成员角色

### 2.6 测试基线

- [x] 单元测试
- [x] 路由级测试
- [x] 真实 PostgreSQL 集成测试
- [x] OpenAPI 导出检查
- [x] 性能 smoke 检查
- [x] 覆盖率基线
- [x] 前端 build 基线
- [x] 前端 smoke E2E 基线
- [ ] 前端单元 / 组件测试基线
- [ ] compile-time SQLx metadata 检查

## 3. 当前剩余工作

### 3.1 后端主路径仍缺的模块

- [x] `project_members` 表与管理端 API
- [x] 项目级权限校验从“管理员会话”收口到成员模型
- [x] `audit_logs`

### 3.2 后端补强项

- [x] 管理端查看 deployment sync records
- [x] 管理端查看 deployment heartbeats
- [x] 管理端 deployment instances 分页、搜索和生命周期接口
- [x] 更完整的 OpenAPI 文档说明与示例
- [x] `alpha-full` 补 `project_members / audit_logs / deployment-sync-records` 的黑盒闭环
- [x] `alpha-full` 补模板 clone、二次发布 diff、旧 token 失效回归
- [x] 多用户 alpha seed / setup 方案，支撑项目级权限黑盒回归
- [ ] `sqlx-check` 恢复为强制检查的时机评估

### 3.3 前端未来主路径

- [x] 登录页
- [x] 项目列表 / 详情页骨架
- [x] 配置文件列表 / 编辑页
- [x] 部署实例列表 / 详情页
- [x] 模板创建实例流程
- [x] Draft 编辑页
- [x] 单配置 clone 交互
- [x] preview-bundle 预览页
- [x] release history 列表
- [x] release 详情 / diff 前端页面
- [x] 部署实例列表拆分模板 / 普通实例区块
- [x] deployment archive / tombstone delete 生命周期

### 3.4 新确认的 MVP 大块

- [ ] 平台级权限模型：引入 `platform_admin`，把平台管理与项目业务访问分层
- [ ] 用户管理：创建用户、禁用/启用、重置密码、强制改密、项目成员绑定
- [ ] 项目创建语义调整：由平台管理员创建项目并指定首个项目 `admin`
- [ ] 系统初始化与首次登录 setup wizard
- [ ] 上线实施方案：Docker Compose + 通用 Linux runbook
- [ ] 上线安全基线：CSRF、安全响应头、登录节流、开放接口限流
- [ ] projects / config_files 的删除能力与生命周期文案统一
- [ ] 低风险管理页面补齐：项目成员、sync records、heartbeats、audit logs
- [ ] 中间文档压缩整理
- [ ] 配置编辑体验统一升级（延后到上述骨架完成之后）

## 4. 当前阶段剩余工作

推荐顺序：

1. 平台级权限模型与用户管理：`platform_admin`、用户状态、项目首个管理员指定
2. 系统初始化与上线实施方案：init 脚本、首次登录 setup wizard、Docker Compose / Linux runbook
3. 上线安全基线：CSRF、安全响应头、登录节流、开放接口限流、平台级/项目级审计边界
4. 资源生命周期与文案收口：projects / config_files 删除能力、用户禁用模型、状态词统一
5. 项目成员页、sync records、heartbeats、audit logs 等低风险管理页面补齐
6. 前端单元 / 组件测试基线，优先覆盖高状态密度组件
7. `sqlx-check` 恢复为强制检查的时机评估
8. 黑盒与覆盖率基线的持续补量
9. 配置编辑体验统一升级：Draft / Release / Diff / Merge 的 Config Workspace

理由：

- 当前业务主路径已经基本闭环，但距离“可上线、可运营、可长期使用”仍缺平台骨架
- 当前最大的剩余风险不再是单个业务页面，而是平台权限分层、初始化上线和安全基线
- 项目成员、sync records、heartbeats、audit logs 已有后端接口，但前端仍未形成完整运营闭环
- 配置编辑体验升级仍然重要，但顺序应后移，避免与平台骨架建设互相打断
- 详细方向已收口到 `docs/constraints/product-qa/0012-mvp-launch-operability-and-admin-model.md`

前端下一批推荐顺序：

1. 平台初始化与登录后首屏：初始密码修改、系统未初始化 / 已初始化分流
2. 用户管理页：用户列表、创建、禁用、重置密码、强制改密
3. 项目创建入口改造：由平台管理员创建并指定首个项目管理员
4. 项目成员页：成员列表、添加成员、角色调整、最后 admin 保护错误提示
5. sync records / heartbeats / audit logs 页面：形成完整运营可见性
6. 前端组件测试基线：优先覆盖高风险状态页和权限相关交互
7. 配置编辑体验统一升级：最后再收束到统一 Config Workspace

## 5. 下一个会话建议先跑的命令

如果只是恢复上下文，先跑：

```bash
git status --short
cargo test --workspace
bash scripts/export-openapi.sh
just coverage-check
```

如果要继续前端主路径，先起本地环境：

```bash
pnpm install
just dev-db-prepare-local
just run-server-local
just dev-web
```

如果要按接近 CI 的方式复现前端 smoke，再跑：

```bash
pnpm --dir apps/web build
PLAYWRIGHT_BASE_URL=http://127.0.0.1:4173 pnpm --dir apps/web test:e2e
```

如果要在本地复现 CI 基线，再跑：

```bash
just ci-local-full
```

如果要继续动数据库主路径，再补：

```bash
just ci-local-db
```

## 6. 下一个会话建议先阅读的文档

### 必读

- [项目脚手架与启动清单](./docs/public/BOOTSTRAP.md)
- [质量检查与测试收口计划](./docs/collaboration/QUALITY_CHECK_PLAN.md)
- [前端任务执行与续工流程](./docs/collaboration/FRONTEND_TASK_WORKFLOW.md)
- [前端接手说明](./docs/collaboration/FRONTEND_HANDOFF.md)
- [前端工作区与运行方式](./docs/collaboration/FRONTEND_WORKSPACE.md)
- [前端页面测试与白屏排查](./docs/collaboration/FRONTEND_PAGE_TESTING.md)
- [产品澄清目录](./docs/constraints/product-qa/README.md)
- [必选配置与预览澄清](./docs/constraints/product-qa/0002-required-configs-and-preview.md)
- [部署实例 Token 重置澄清](./docs/constraints/product-qa/0004-token-reset.md)
- [项目成员、项目级权限与审计日志澄清](./docs/constraints/product-qa/0005-project-members-permissions-audit.md)
- [配置标识与心跳澄清](./docs/constraints/product-qa/0007-config-identity-and-heartbeats.md)
- [MVP 上线运营闭环与平台管理模型澄清](./docs/constraints/product-qa/0012-mvp-launch-operability-and-admin-model.md)
- [咖啡中间件演示案例规格](./docs/constraints/DEMO_SCENARIO_COFFEE_MIDDLEWARE.md)
- [前端 MVP 蓝图](./docs/constraints/FRONTEND_MVP_BLUEPRINT.md)

### 与环境相关

- [Linux / WSL2 开发与部署草案](./docs/agents/DEV_LINUX_WSL2.md)
- [Fedora 43 开发环境与本地 Agent 约定](./docs/agents/DEV_FEDORA43_WORKSTATION.md)
- [WSL 与 Fedora 双环境并列开发清单](./docs/agents/DEV_DUAL_ENV_PARITY.md)

### 与当前已实现接口相关

- [管理端 API 草案](./docs/constraints/ADMIN_API.md)
- [开放消费端协议](./docs/public/CLIENT_HTTP_PROTOCOL.md)
- [数据库模型草案](./docs/constraints/DB_SCHEMA.md)

## 7. 当前会话结束前的注意事项

- OpenAPI 导出现在是强制检查项；接口或 schema 改动后，需要同步更新 `docs/artifacts/openapi.json`
- `sqlx-check` 当前是条件启用，不是 CI runner 不支持，而是当前仓库尚未进入 compile-time SQLx metadata 阶段
- 当前工作区还有未提交改动时，不要只提交代码不提交 OpenAPI 产物
- 前端后续任务默认先按 `FRONTEND_TASK_WORKFLOW.md` 输出任务规范和执行计划，再由 Codex 本地实现并自验
- 前端白屏或联调异常时，不要只看 `/api/healthz`；至少同时验证 `/api/auth/me`、登录链路和浏览器 Console
- 前端 smoke 依赖后端真正建立 `db_pool`；CI / 本地复现都不要把 `INIT_DB_ON_BOOT` 设成会禁用数据库初始化的值

## 8. 建议的交接语句

为避免在多个文档里维护近似但不完全一致的 prompt，完整 kickoff 模板统一以 [docs/collaboration/FRONTEND_TASK_WORKFLOW.md](./docs/collaboration/FRONTEND_TASK_WORKFLOW.md) 第 10 节为准。

如果下一个会话需要快速恢复上下文，可以直接先贴这一段：

```text
请先阅读 DEVELOPMENT_LOG.md，然后按 docs/collaboration/FRONTEND_TASK_WORKFLOW.md 第 10 节的统一 kickoff prompt 继续。

本轮任务是：
[把这里替换成具体页面或模块]
```
