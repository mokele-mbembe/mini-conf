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
- 支持 schema 校验和发布前检查
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

- 前端：Vue 3、Vite、TypeScript、Element Plus、Monaco Editor
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

当前这个 Windows 仓库主要用于设计和规划，不作为实际开发环境。

后续实际编码、联调、测试和部署以 Linux 主机或本机 WSL2 实例为准。这意味着：

- 脚本优先提供 shell / `just` 版本
- 本地调试流程优先针对 Linux / WSL2 编写
- CI 也应以 Linux 环境为标准

## 文档导航

- 项目启动与里程碑：[KICKOFF.md](c:\Users\zhaoj\Projects\mini-conf\KICKOFF.md)
- 脚手架与启动规划：[docs/BOOTSTRAP.md](c:\Users\zhaoj\Projects\mini-conf\docs\BOOTSTRAP.md)
- 数据模型草案：[docs/DB_SCHEMA.md](c:\Users\zhaoj\Projects\mini-conf\docs\DB_SCHEMA.md)
- 管理端 API 草案：[docs/ADMIN_API.md](c:\Users\zhaoj\Projects\mini-conf\docs\ADMIN_API.md)
- 部署实例模型与未来 Scope 规划：[docs/SCOPE_RULES.md](c:\Users\zhaoj\Projects\mini-conf\docs\SCOPE_RULES.md)
- 鉴权与安全草案：[docs/AUTH_AND_SECURITY.md](c:\Users\zhaoj\Projects\mini-conf\docs\AUTH_AND_SECURITY.md)
- 消费端 HTTP 协议草案：[docs/CLIENT_HTTP_PROTOCOL.md](c:\Users\zhaoj\Projects\mini-conf\docs\CLIENT_HTTP_PROTOCOL.md)
- 前端 workspace 最小脚手架：[docs/FRONTEND_WORKSPACE.md](c:\Users\zhaoj\Projects\mini-conf\docs\FRONTEND_WORKSPACE.md)
- 性能测试 scaffold：[docs/PERFORMANCE.md](c:\Users\zhaoj\Projects\mini-conf\docs\PERFORMANCE.md)
- MVP 之后的版本规划：[docs/POST_MVP_PLAN.md](c:\Users\zhaoj\Projects\mini-conf\docs\POST_MVP_PLAN.md)
- 仓库初始化清单：[docs/REPO_INIT_CHECKLIST.md](c:\Users\zhaoj\Projects\mini-conf\docs\REPO_INIT_CHECKLIST.md)
- 提交前 Review 清单：[docs/SUBMISSION_CHECKLIST.md](c:\Users\zhaoj\Projects\mini-conf\docs\SUBMISSION_CHECKLIST.md)
- 首次提交说明草案：[docs/INITIAL_PR_DRAFT.md](c:\Users\zhaoj\Projects\mini-conf\docs\INITIAL_PR_DRAFT.md)
- Linux / WSL2 开发环境实录：[docs/DEV_LINUX_WSL2.md](c:\Users\zhaoj\Projects\mini-conf\docs\DEV_LINUX_WSL2.md)

## 当前状态

项目目前处于设计 / 规划阶段，当前仓库重点是：

- 明确产品边界
- 固化领域模型
- 完成数据库和 API 设计
- 提前建立代码质量和 TDD 工作流

## 后续优先事项

建议按这个顺序推进：

1. 固化 README、部署实例模型、消费协议和开发环境文档
2. 初始化 Rust workspace 和前端工程
3. 建立 PostgreSQL migrations
4. 建立 `justfile`、lint、test、CI 基线
5. 从开放消费端最小协议倒推管理端实现

## License

计划使用：

- MIT OR Apache-2.0

相关文件后续补齐：

- `LICENSE-MIT`
- `LICENSE-APACHE`
- `NOTICE`
- `THIRD_PARTY_NOTICES.md`
