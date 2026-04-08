# 开发记录与会话交接

## 1. 文档目的

这份文档用于记录当前后端开发进度，并为下一个会话提供直接可执行的起手上下文。

适用场景：

- 隔一段时间后重新进入项目
- 切换机器继续开发
- 新开会话时快速恢复上下文

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

### 2.3 管理端认证

- [x] `POST /api/auth/login`
- [x] `GET /api/auth/me`
- [x] `POST /api/auth/logout`
- [x] 默认管理员 seed
- [x] session cookie

### 2.4 管理端资源

- [x] `projects` CRUD
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

### 2.5 产品规则收口

- [x] Template 仍然是实例概念
- [x] Template 禁止发布 release
- [x] 项目层支持必选配置文件
- [x] 发布单配置前检查实例是否已具备全部必选配置
- [x] 模板 clone 仅允许从 draft
- [x] 单配置 clone 支持 `draft | latest_release`
- [x] preview-bundle 返回业务预览明细和 consumer 侧整包预览
- [x] `release diff` 固定比较上一版并返回文本级摘要

### 2.6 测试基线

- [x] 单元测试
- [x] 路由级测试
- [x] 真实 PostgreSQL 集成测试
- [x] OpenAPI 导出检查
- [x] 性能 smoke 检查
- [ ] 覆盖率基线
- [ ] 前端测试基线
- [ ] compile-time SQLx metadata 检查

## 3. 当前剩余工作

### 3.1 后端主路径仍缺的模块

- [ ] `POST /api/deployment-instances/:id/token/reset`
- [ ] `project_members` 表与管理端 API
- [ ] 项目级权限校验从“管理员会话”收口到成员模型
- [ ] `audit_logs`

### 3.2 后端补强项

- [ ] token reset 后与 open API 鉴权的联动
- [ ] 管理端查看 deployment sync records
- [ ] 更完整的 OpenAPI 文档说明与示例
- [ ] `sqlx-check` 恢复为强制检查的时机评估

### 3.3 前端未来主路径

- [ ] 登录页
- [ ] 项目列表 / 详情页
- [ ] 配置文件列表 / 编辑页
- [ ] 部署实例列表 / 详情页
- [ ] 模板创建实例流程
- [ ] Draft 编辑页
- [ ] 单配置 clone 交互
- [ ] preview-bundle 预览页
- [ ] release history / diff 页

## 4. 下一个会话推荐起手工作

推荐顺序：

1. `POST /api/deployment-instances/:id/token/reset`
2. `project_members`
3. 项目级权限收口
4. `audit_logs`

理由：

- `diff` 是当前发布主链路里最直接的缺口
- `token/reset` 直接关系到部署实例生命周期闭环
- `project_members` 和权限应该放在资源 API 基本齐备之后再做

### 当前建议的第一个开发任务

优先从 `POST /api/deployment-instances/:id/token/reset` 开始。

建议做法：

- 先明确旧 token 失效与新 token 生效的联动语义
- 再补接口响应模型与 OpenAPI
- 最后写真实 PostgreSQL 集成测试，覆盖 open API 鉴权切换

## 5. 下一个会话建议先跑的命令

如果只是恢复上下文，先跑：

```bash
git status --short
cargo test --workspace
bash scripts/export-openapi.sh
```

如果要在本地复现 CI 基线，再跑：

```bash
just lint
just openapi-check
just test
```

如果要继续动数据库主路径，再补：

```bash
just test-backend-db
```

## 6. 下一个会话建议先阅读的文档

### 必读

- [项目脚手架与启动清单](./docs/BOOTSTRAP.md)
- [质量检查与测试收口计划](./docs/QUALITY_CHECK_PLAN.md)
- [产品澄清目录](./docs/product-qa/README.md)
- [必选配置与预览澄清](./docs/product-qa/0002-required-configs-and-preview.md)
- [前端 MVP 蓝图](./docs/FRONTEND_MVP_BLUEPRINT.md)

### 与环境相关

- [Linux / WSL2 开发与部署草案](./docs/DEV_LINUX_WSL2.md)
- [Fedora 43 开发环境与本地 Agent 约定](./docs/DEV_FEDORA43_WORKSTATION.md)
- [WSL 与 Fedora 双环境并列开发清单](./docs/DEV_DUAL_ENV_PARITY.md)

### 与当前已实现接口相关

- [管理端 API 草案](./docs/ADMIN_API.md)
- [开放消费端协议](./docs/CLIENT_HTTP_PROTOCOL.md)
- [数据库模型草案](./docs/DB_SCHEMA.md)

## 7. 当前会话结束前的注意事项

- OpenAPI 导出现在是强制检查项；接口或 schema 改动后，需要同步更新 `docs/openapi/openapi.json`
- `sqlx-check` 当前是条件启用，不是 CI runner 不支持，而是当前仓库尚未进入 compile-time SQLx metadata 阶段
- 当前工作区还有未提交改动时，不要只提交代码不提交 OpenAPI 产物

## 8. 建议的交接语句

如果下一个会话需要快速恢复上下文，可以直接从这里开始：

```text
先阅读 DEVELOPMENT_LOG.md、docs/QUALITY_CHECK_PLAN.md、docs/product-qa/0003-release-diff.md，然后继续做 POST /api/deployment-instances/:id/token/reset，并先补文档再改代码。
```
