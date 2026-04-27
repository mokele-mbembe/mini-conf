# 前端交接包

## 1. 文档目标

这份文档给前端实现管理台时直接使用。

目标：

- 把页面、接口、权限、关键业务规则收敛到一处
- 明确哪些信息可以只看 OpenAPI，哪些不能只靠接口猜
- 标出当前文档里已经过时的说法，避免前端按旧语义开发
- 列出还需要产品或后端拍板的交互问题

## 2. 前端应优先参考的资料

建议按这个顺序阅读：

1. [`docs/agents/AGENT_START_HERE.md`](../agents/AGENT_START_HERE.md)
2. [`docs/collaboration/FRONTEND_TASK_WORKFLOW.md`](./FRONTEND_TASK_WORKFLOW.md)
3. [`docs/collaboration/FRONTEND_HANDOFF.md`](./FRONTEND_HANDOFF.md)
4. [`docs/constraints/FRONTEND_MVP_BLUEPRINT.md`](../constraints/FRONTEND_MVP_BLUEPRINT.md)
5. [`docs/constraints/ADMIN_API.md`](../constraints/ADMIN_API.md)
6. [`docs/artifacts/openapi.json`](../artifacts/openapi.json)
7. [`docs/constraints/product-qa/README.md`](../constraints/product-qa/README.md)
8. [`docs/constraints/AUTH_AND_SECURITY.md`](../constraints/AUTH_AND_SECURITY.md)
9. [`docs/public/CLIENT_HTTP_PROTOCOL.md`](../public/CLIENT_HTTP_PROTOCOL.md)
10. [`docs/constraints/DEMO_SCENARIO_COFFEE_MIDDLEWARE.md`](../constraints/DEMO_SCENARIO_COFFEE_MIDDLEWARE.md)

推荐真值优先级：

- 页面流程与交互意图：`FRONTEND_MVP_BLUEPRINT`
- 前端执行流程、运行和测试：`FRONTEND_TASK_WORKFLOW`
- 业务规则与边界：`product-qa/*`
- 字段、错误码、响应形状：`openapi.json` + `ADMIN_API`
- 当前到底实现到哪：`DEVELOPMENT_LOG.md`

开始前端开发前，建议先准备本机 demo 运行库：

- `just dev-db-prepare-local`
- `just run-server-local`
- `just dev-web`

## 2.1 当前前端已落地基线

当前仓库已经有真实前端应用，不再是“只有前端预留 workspace”：

- `apps/web` 已初始化
- 已有登录、setup、首次改密、平台用户管理、平台项目创建
- 已有项目列表、配置文件、环境、部署实例、Draft、Saved Versions、preview、publish、Release detail/diff
- 已有 deployment archive / restore / permanent delete 前端路径
- 已有项目成员列表、添加成员、角色调整、删除成员前端路径
- 已有 sync records 列表与实例 / 配置 / action / status 筛选前端路径
- 已有 heartbeats 列表与实例 / 配置筛选前端路径
- 已有 audit logs 列表与用户 / action / resource_type 筛选前端路径
- 已有前端 build check 和覆盖核心管理链路的 Playwright E2E

因此后续前端工作应默认视为“在已有 scaffold 上继续开发”，而不是重新搭项目前端。

## 3. 当前后端已完成范围

管理端主路径当前已可用：

- 登录 / 登出 / 当前会话
- 项目 CRUD
- 项目成员 CRUD
- 配置文件 CRUD
- 部署实例 CRUD
- 模板 clone 创建新实例
- Draft 获取 / 保存
- 单配置 clone
- preview-bundle
- Release 发布、列表、详情、diff
- deployment token reset
- deployment activate / deactivate
- deployment sync records 查询
- deployment heartbeats 查询
- audit logs 查询

开放消费端主路径当前已可用：

- `resolve`
- `release`
- `config-bundle`
- `sync-record`
- `heartbeat`

低风险运营页面当前已补齐真实实现，不再是占位路由。

当前不在 MVP 主路径范围内：

- 多候选 Draft
- 整实例一键发布
- 动态 Scope / labels
- 审批流
- SSO / OAuth

## 4. 核心业务模型

前端必须先按下面的心智模型理解系统：

- `Project` 表示一个业务或代码项目。
- `ConfigFile` 是项目级配置文件定义，例如 `main`、`vision`。
- `DeploymentInstance` 是项目在某个环境下的一份独立部署实例。
- `Template` 不是单独模型，而是 `DeploymentInstance.is_template = true` 的特殊实例。
- 同一个实例下有多份配置文件，每份配置文件各自拥有自己的 Current Draft / Release 历史。
- `publish` 不是整实例发布，而是“某实例下某配置文件发布”。
- 同一个 `deployment_instance + config_file` 只有一份 Current Draft。
- 长期推荐模型是 `Current Draft + Saved Versions + Releases`，而不是多个并列可编辑 Draft。
- `Saved Versions` 和 `Releases` 都是只读来源；需要继续编辑或发布时，先恢复到 Current Draft。
- 消费端 token 是部署实例级共享凭证，不是进程级凭证。
- 部署实例只使用 `active / inactive`。创建和 clone 默认 `inactive`，激活后才允许 Open API 消费。
- 客户端配置标识统一为 `ConfigFile.code`。前端不要再引入 `process_key`，同步记录和心跳筛选使用 `config_file_id`。

## 5. 页面与接口映射

### 5.1 登录

- `POST /api/auth/login`
- `GET /api/auth/me`
- `POST /api/auth/logout`

前端规则：

- 首屏先调用 `/api/auth/me`
- 未登录跳回登录页
- 会话失效统一回登录

### 5.2 项目列表 / 详情

- `GET /api/projects`
- `GET /api/projects/:id`
- `PUT /api/projects/:id`
- 平台创建入口使用 `GET /api/admin/projects` / `POST /api/admin/projects`

前端规则：

- 项目只对成员可见
- 平台管理员默认不自动拥有业务项目可见性
- 创建项目必须通过平台创建流程指定首个项目 `admin`
- `POST /api/projects` 只是后端兼容别名，新前端入口应使用 `/api/admin/projects`
- 列表默认不需要前端自行做权限过滤

### 5.3 项目成员

- `GET /api/projects/:id/members`
- `POST /api/projects/:id/members`
- `PUT /api/projects/:id/members/:memberId`
- `DELETE /api/projects/:id/members/:memberId`

前端规则：

- 通过 `username` 绑定已存在用户
- 成员接口不会顺带创建用户
- 不能删除或降级最后一个项目 `admin`

### 5.4 配置文件

- `GET /api/config-files?project_id=:projectId&status=`
- `POST /api/config-files`
- `GET /api/config-files/:id`
- `PUT /api/config-files/:id`

前端规则：

- `is_required` 是项目级规则，不支持按实例覆盖
- `sensitivity=secret` 时，管理端展示默认按脱敏语义理解
- `secret_paths` 主要用于后端脱敏，不要求前端自己实现脱敏算法
- `code` 在前端文案里统一显示为“配置标识”
- `text` 不是当前 MVP 预期格式，不应再暴露
- `toml` 已进入当前主路径支持范围，可与 `yaml / json` 一样用于配置文件创建、Draft 保存、发布和管理端脱敏展示

### 5.5 部署实例 / 模板

- `GET /api/deployment-instances?project_id=:projectId&environment=&status=&keyword=&page=&page_size=`
- `POST /api/deployment-instances`
- `GET /api/deployment-instances/:id`
- `PUT /api/deployment-instances/:id`
- `POST /api/deployment-instances/:id/clone`
- `POST /api/deployment-instances/:id/activate`
- `POST /api/deployment-instances/:id/deactivate`
- `POST /api/deployment-instances/:id/token/reset`
- `GET /api/deployment-instances/:id/preview-bundle`

前端规则：

- `Template` 通过 `is_template` 区分
- 模板只能作为 clone 来源，不能发布
- 模板 clone 第一版只允许 `clone_source = draft`
- 创建实例和模板 clone 结果默认是 `inactive`
- `PUT` 只允许修改 `environment / deployment_key / name / description`
- `activate` 仅普通实例可用，成功后返回一次性明文 token
- `deactivate` 仅 active 普通实例可用，成功后旧 token 立即失效
- token reset 仅 active 普通实例可用，成功后旧 token 立即失效
- 列表响应是分页形状：`items / total / page / page_size`

### 5.6 Current Draft / 配置工作台

- `GET /api/drafts/:deploymentId/:configFileId`
- `PUT /api/drafts/:deploymentId/:configFileId`
- `POST /api/drafts/:targetDeploymentId/:configFileId/clone`
- `GET /api/draft-saved-versions?deployment_instance_id=&config_file_id=`
- `GET /api/draft-saved-versions/:id`
- `PATCH /api/draft-saved-versions/:id`
- `POST /api/draft-saved-versions/:id/restore`
- `DELETE /api/draft-saved-versions/:id`

前端规则：

- 当前后端已提供的是唯一 `Current Draft`
- Current Draft 不存在时进入“新建 Draft”态
- 保存时必须带 `base_version`
- 冲突错误码是 `draft_version_conflict`
- 单配置 clone 默认覆盖 Current Draft，并递增版本
- 长期入口建议命名为“配置工作台”，不要只暴露“编辑 Draft”
- 前端已在 Draft 编辑页接入 Saved Versions 历史面板
- Saved Versions 当前以 `admin / editor` 可见为准，viewer 不应看到工作过程数据

### 5.7 发布历史 / Diff / 发布确认

- `GET /api/releases`
- `GET /api/releases/:id`
- `GET /api/releases/:id/diff`
- `POST /api/releases/publish`

前端规则：

- `publish` 是单配置发布
- 发布对象始终是 Current Draft
- 如果要发布历史 Saved Version 或历史 Release，先恢复到 Current Draft 再发布
- `diff` 固定和上一版比较，不支持任意两版自由比较
- secret 配置在管理端 `release detail / diff` 中会返回脱敏内容
- 发布前如果必选配置缺失，会阻塞当前这次单配置发布
- 当前前端已有 Release 列表、Release 详情和 Release Diff 独立页面
- Release 详情页以不可编辑文本框回看 `content`，并显眼展示发布账号 ID、发布时间、revision、配置和实例上下文
- Post-MVP 可把只读文本框升级为 Monaco 只读高亮，把 Diff 页升级为带颜色的 DiffEditor

### 5.8 审计 / 同步 / 心跳

- `GET /api/audit-logs`
- `GET /api/deployment-sync-records`
- `GET /api/deployment-heartbeats`

前端规则：

- 审计详情只应按安全元数据展示
- 心跳接口当前只返回最近一次状态，不做前端“在线/离线”推断真值
- sync records 和 heartbeats 都按 `config_file_id` 过滤，并展示返回的 `config`

## 6. 不能只靠接口猜的业务规则

这些规则前端必须直接采用，不能从接口名字自行推断：

- `Template` 不是单独资源类型，而是实例的一个状态。
- 模板不能发布，只能 clone。
- `publish` 是单配置发布，不是整实例发布。
- 同一实例同一配置文件只有一份 Current Draft，不支持多候选稿。
- 推荐历史模型是 `Current Draft + Saved Versions + Releases`，不是多个并列可编辑 Draft。
- `Saved Versions` 和 `Releases` 只能恢复到 Current Draft，不能直接编辑。
- `Draft > latest_release` 是 preview-bundle 的固定优先级。
- `is_required` 的含义是“发布门槛”，不是“实例创建时立刻必填”。
- `required` 配置满足条件是“有 Draft 或有历史 Release”，不是必须两者都有。
- 非成员访问项目资源返回资源自身 `404`，不是统一 `403`。
- 成员但权限不足才返回 `403 project_permission_denied`。
- 管理端 release detail/diff 默认脱敏；开放消费端读取仍是明文。
- token reset 没有灰度切换窗口，旧 token 立即失效。
- activate 也会生成或覆盖默认 token，并且明文 token 只在响应里返回一次。
- 当前 deployment 没有 `archived` 状态；未启用和已停用统一显示为 `inactive`。
- “归档后默认隐藏、归档抽屉可找回、恢复为 inactive”已经通过 `is_archived` 软归档维度落地，没有把 `archived` 加回 `status`。
- 释放 `deployment_key` 通过产品层 permanent delete 完成：底层保留 tombstone row 和 `deployment_uid`，deleted 实例不可恢复且默认不可见。
- `process_key` 已退出 MVP 后端主路径；前端只使用 `config` / `config_file_id`。
- `config-bundle` 只返回已有发布的配置，不会给未发布配置补空对象。

## 7. 当前已知的文档过时点

以下说法前端不要再按字面执行：

- `FRONTEND_MVP_BLUEPRINT` 里“先按管理员已登录语义设计”已经落后。
  当前后端已经按项目角色 `admin / editor / viewer` 收口。
- `FRONTEND_MVP_BLUEPRINT` 里“diff 接口仍待实现”已经落后。
  当前 `GET /api/releases/:id/diff` 已实现。
- 早期与中间阶段文档里提到的 `schema_name / schema_version / schema validator` 主路径语义已经过时。
  当前 MVP 口径已收口为“基础格式合法性校验”，不再把 schema 视作当前对外能力。
- 早期文档里把部署实例状态写成 `active / inactive / archived` 的说法与当前实现不一致。
  当前 deployment 运行态只采用 `active / inactive`；归档 / 删除已经按 `0010` 的 `deployment_uid + is_archived + deleted_at` 模型落地。
- 早期文档和讨论中的 `process_key` 只作为历史澄清背景保留，不应出现在前端新代码或新请求中。

如果前端发现蓝图和 OpenAPI / 当前后端行为冲突，以当前实现和这份交接文档为准，再回头补文档统一。

## 8. 权限落地建议

首版前端建议直接按后端角色做按钮和入口控制，而不是继续假设“所有已登录用户都是管理员”。

推荐矩阵：

- `admin`
  - 可见并可操作：项目编辑、成员管理、配置文件编辑、实例编辑、模板 clone、Draft 编辑、preview、发布、token reset、audit logs
- `editor`
  - 可见并可操作：Draft 编辑、单配置 clone、preview、发布、release 历史、sync records、heartbeats
  - 只读：项目、配置文件、部署实例
- `viewer`
  - 可见：项目、配置文件、部署实例、release 历史、sync records、heartbeats
  - 不可操作：Draft 编辑、preview、发布、token reset、成员管理、audit logs

注意：

- 后端仍然是最终权限真值，前端按钮隐藏只是辅助体验。
- 非成员资源页建议按“未找到”处理，不在 UI 上暴露“你没有这个项目”的存在信息。

## 9. 错误处理建议

前端建议区分三类错误：

- 认证错误
  - 典型表现：未登录、会话过期
  - 处理：跳转登录页
- 授权错误
  - 典型表现：`403 project_permission_denied`
  - 处理：保留页面上下文，提示当前角色权限不足
- 业务错误
  - 典型表现：`draft_version_conflict`、`required_config_missing`、`deployment_instance_template_publish_forbidden`
  - 处理：在当前页面直接展示可理解的业务提示

建议重点接业务错误码，而不是只显示通用 message。

## 10. 前端仍需人工拍板的问题

这些问题不会影响后端接口使用，但会影响页面体验，需要产品或你来定：

1. 首版是否就按角色隐藏按钮，还是先全部展示后依赖后端拦截。
2. 项目 / 配置文件的 archived 资源在列表里默认是否展示。
3. 项目详情是页内 tabs、左侧导航，还是独立路由拆页。
4. 发布确认是抽屉、弹窗，还是独立页面。
5. preview-bundle 更偏“工程工具页”还是“业务可读页”。
6. secret 内容是否要在前端加二次复制确认或额外遮罩。
7. 心跳页面是否需要前端自行定义“离线阈值”：当前不定义，只展示后端最近一次上报。
8. 审计页是否只给 `admin`，以及是否需要事件类型筛选器：当前仅项目 `admin` 可见，提供 action / resource_type / user 过滤。
9. 配置工作台是否继续保留在 Draft 编辑页内，还是升级为独立三栏工作台路由。
10. Release 详情页恢复到 Current Draft 后是否自动跳转编辑页，还是停留在只读详情页。
11. deleted 历史实例在 Release / audit 页面展示为“已删除于 ...”还是更短的状态标签。

## 11. 建议的前端第一阶段实现顺序

建议这样排：

1. 前端组件测试基线
2. 权限矩阵和异常分支 E2E 补量
3. 咖啡中间件 demo 管理端辅助入口
4. Release 详情 / Diff 体验增强
5. Config Workspace 统一升级

原因：

- “项目 -> 配置 -> 实例 -> Draft -> 发布 -> Release 回看 / Diff -> 归档删除”主路径已经能跑通
- 下一批应补 demo 观察面、权限管理和可维护性，让真实演示与后续使用更稳

## 12. 对前端最有价值的真值文件

- OpenAPI 产物：[`docs/artifacts/openapi.json`](../artifacts/openapi.json)
- 后端集成测试目录：[`apps/server/tests`](/home/zjj/Projects/mini-conf/apps/server/tests)
- 当前实现进度：[`DEVELOPMENT_LOG.md`](/home/zjj/Projects/mini-conf/DEVELOPMENT_LOG.md)

如果某个接口字段语义不清楚，优先看：

1. OpenAPI
2. 对应的 `apps/server/tests/*.rs`
3. product-qa 澄清文档
4. 再找后端确认
