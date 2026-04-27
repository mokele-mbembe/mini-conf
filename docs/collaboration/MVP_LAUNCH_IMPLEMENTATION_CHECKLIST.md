# MVP 上线实施清单

## 1. 文档目标

这份清单把 `0012` 中已经确认的 MVP 方向拆成可执行批次。

它主要回答：

- 哪些是必须先改的数据模型和权限边界
- 哪些 API 和页面属于同一批，应该一起推进
- 哪些属于上线前必须完成的安全和初始化能力
- 哪些工作应该明确后移，避免和主线打架

这份清单默认只覆盖 **MVP 上线前必须完成** 的大项，不展开 post-MVP 规划。

相关方向真值见：

- [0012-mvp-launch-operability-and-admin-model.md](/home/zjj/Projects/mini-conf/docs/constraints/product-qa/0012-mvp-launch-operability-and-admin-model.md)
- [0005-project-members-permissions-audit.md](/home/zjj/Projects/mini-conf/docs/constraints/product-qa/0005-project-members-permissions-audit.md)
- [AUTH_AND_SECURITY.md](/home/zjj/Projects/mini-conf/docs/constraints/AUTH_AND_SECURITY.md)
- [DB_SCHEMA.md](/home/zjj/Projects/mini-conf/docs/constraints/DB_SCHEMA.md)

## 2. 当前实现状态

这份清单最初写于平台上线骨架开工前。当前仓库已经完成了前几阶段的大量代码工作，因此后续执行时先看本节状态，再看各 Phase 的细项。

### 2.1 已基本完成

- 平台级权限模型：
  - `users.is_platform_admin`
  - 平台管理员默认不自动拥有项目可见性
  - 项目创建要求平台管理员指定首个项目 `admin`
  - `/api/admin/projects`
- 用户管理后端与前端主路径：
  - 用户列表、创建、启用/禁用
  - 重置密码
  - `must_change_password`
  - `last_login_at`
  - `password_updated_at`
  - 禁用用户和重置密码时撤销已有 session
- Setup 核心链路：
  - `system_settings`
  - `GET /api/setup/status`
  - `POST /api/setup/complete`
  - 未完成 setup 时阻断业务接口
  - 前端 setup 页与首次改密页
- 管理端安全基线大部分：
  - Session Cookie `HttpOnly / SameSite`
  - staging / prod 默认 `Secure`
  - CSRF cookie + `X-CSRF-Token`
  - CSP / HSTS 等基础安全响应头
  - 登录失败节流
  - 密码强度校验
  - 首次/强制改密
- Open API 安全基线：
  - Bearer token 校验
  - 基础限流
  - 失败事件审计
  - 基础 HTTP tracing
- 平台级审计主路径：
  - 用户创建 / 禁用 / 重置密码
  - 平台项目创建
  - setup completed

### 2.2 部分完成

- Setup wizard 已覆盖“创建用户、创建首个项目、指定项目管理员、完成 setup”，但尚未覆盖首个环境、配置文件、模板实例。
- 项目创建兼容别名 `POST /api/projects` 已改成平台管理员创建项目语义，但长期应优先使用 `/api/admin/projects`。
- 审计日志已有平台级、项目级和 Open API 失败数据，前端 audit logs 页面已接入项目级查询与筛选。

### 2.3 尚未完成

- Linux binary 发布包方案已收口为 `just release-package` / `just release-package-check`，GitHub Actions `Release Package` workflow 已可生成并自检 artifact；后续只需要按正式版本策略决定是否创建 GitHub Release。
- 生产部署 runbook 已覆盖外部 PostgreSQL、`config-center.example.com` 示例域名、反向代理/TLS、环境变量、迁移与初始化；`just staging-smoke` 已提供真实环境只读探测，后续需要按真实 staging 试部署结果补充排障项。
- 文档压缩后的最终三层结构。
- 前端单元 / 组件测试已建立初始基线，后续继续扩展高状态密度组件覆盖。
- Config Workspace 统一升级已进入渐进收口阶段，后续继续抽薄 Draft 页面层并准备 Merge Workspace。

## 3. 后续固定顺序

按下面顺序推进，避免在编辑器体验或低风险页面上过早分散精力：

1. 上线实施方案
2. 资源生命周期与文案收口
3. 前端单元 / 组件测试补量
4. 文档压缩
5. Config Workspace 统一升级

## 4. Phase 1: 平台级权限模型与用户管理

状态：已基本完成，后续只做文档同步、细节修正和缺口补齐。

### 4.1 目标

把当前“登录用户 + 项目成员”模型扩成两层：

- 平台层：`platform_admin`
- 项目层：`admin / editor / viewer`

同时补齐 MVP 可长期运营的用户管理模型。

### 4.2 必须完成的设计改动

1. 平台管理员默认不自动拥有任何项目可见性
2. 项目创建只能由 `platform_admin` 发起
3. 创建项目时必须指定至少一个项目 `admin`
4. 用户对象改为长期可运营模型：
   - `active | disabled`
   - `must_change_password`
   - `last_login_at`
   - `password_updated_at`
5. 用户不做物理删除

### 4.3 后端实施项

#### 数据模型

- `users` 表增加：
  - `is_platform_admin`
  - `must_change_password`
  - `last_login_at`
  - `password_updated_at`
- 确认 `status` 语义统一为：
  - `active`
  - `disabled`

#### 鉴权 / 授权

- 增加平台级鉴权 helper：
  - `require_platform_admin`
- 保留并继续使用项目级：
  - `require_project_role`
- 收口 `/api/projects` 相关语义：
  - 项目列表仍只列出当前用户可见项目
  - 平台管理员若未加入项目，不应直接看到业务项目列表

#### 新 API

建议新增一组平台管理端 API：

- `GET /api/admin/users`
- `POST /api/admin/users`
- `GET /api/admin/users/:id`
- `PATCH /api/admin/users/:id`
- `POST /api/admin/users/:id/reset-password`
- `GET /api/admin/projects`
- `POST /api/admin/projects`

建议把“平台侧创建项目”和“项目内修改项目信息”分开，而不是继续复用同一路由语义。

### 4.4 前端实施项

新增平台级页面：

- 用户列表页
- 用户创建 / 编辑抽屉
- 平台项目创建页或 wizard 步骤

改造现有页面：

- 登录后首页需要根据身份分流：
  - 首次初始化未完成 -> setup
  - `platform_admin` -> 平台控制台
  - 普通项目成员 -> 项目列表

### 4.5 测试验收

至少补：

- `platform_admin` 未加入项目时无法通过项目页看到业务数据
- 被禁用用户无法登录
- 项目创建必须指定首个项目 `admin`
- 普通项目成员不能调用平台级用户管理接口

## 5. Phase 2: 系统初始化与首次登录 Setup

状态：核心链路已完成；初始化交付和 wizard 扩展仍需补齐。

### 5.1 目标

交付后的系统不再依赖手工 seed 文件编辑来完成首启。

### 5.2 必须完成的能力

1. 初始化 CLI / init 命令
2. 首次登录改密（已完成）
3. 首次登录 setup wizard（部分完成）
4. Linux binary 发布包方案
5. 生产部署 runbook：外部 PostgreSQL + 独立入口域名 + 反向代理/TLS

### 5.3 后端实施项

系统初始化状态来源已经采用 `system_settings` 表。

当前已实现初始化相关 API：

- `GET /api/setup/status`
- `POST /api/setup/complete`

约束建议：

- 未初始化时，仅开放 setup 相关接口和健康检查
- 完成初始化后，bootstrap 类能力不应再开放；当前仓库尚未实现独立 `bootstrap-admin` API，而是通过初始化配置 / seed 创建首个平台管理员。

### 5.4 前端实施项

新增：

- Setup 状态探测
- 首次改密页
- Setup Wizard

建议 wizard 后续补齐到这些步骤：

1. 修改初始平台管理员密码
2. 创建首批用户
3. 创建首个项目
4. 指定首个项目管理员
5. 创建首个环境
6. 创建首个配置文件 / 模板实例

### 5.5 交付物

MVP 交付包中应包含：

- 初始化命令说明
- 生产部署变量清单
- Linux binary 发布包布局与构建说明
- 外部 PostgreSQL 连接、迁移和备份边界说明
- `config-center.example.com` 这类独立入口域名下的反向代理/TLS 示例
- 首次启动排障手册

## 6. Phase 3: 上线安全基线

状态：MVP 代码侧安全基线已完成；后续主要进入上线实施 runbook 和运维页面。

### 6.1 目标

把当前已经写在规则文档里的安全边界真正落到代码和部署方案中。

### 6.2 必做项

#### 管理端

- Session Cookie:
  - `HttpOnly`（已完成）
  - `Secure`（staging / prod 默认启用）
  - `SameSite`（已完成）
- CSRF 防护（已完成）
- 基础安全响应头（已完成，包含 CSP；staging / prod 启用 HSTS）
- 登录失败节流（已完成）
- 密码强度校验（已完成）
- 首次改密 / 强制改密逻辑（已完成）

#### 开放消费端

- 基础限流（已完成）
- 关键失败事件留痕（已完成，写入 `audit_logs`）
- 请求链路日志（已有基础 tracing，失败事件进入审计）

#### 审计

- 区分平台级审计和项目级审计
- 补平台级动作：
  - 用户创建
  - 用户禁用
  - 初始化完成
  - 项目创建

### 6.3 技术实施项

- HTTP 层安全 header middleware
- Session / 登录接口的 CSRF 方案
- 登录节流实现
- Open API 限流中间件（已完成）
- 错误日志 / 审计日志脱敏复查（Open API 失败审计不记录 token 明文）

### 6.4 测试验收

- 未携带合法 CSRF 的管理端写请求被拒绝
- 登录错误达到阈值后被节流
- Open API 高频错误请求被限流
- 响应头包含预期安全 header
- 审计和 tracing 中不出现明文 secret / token

## 7. Phase 4: 资源生命周期与文案收口

状态：deployment instance 已完成；projects / config_files 删除能力和引用检查已完成，后续只做状态词与边角文案复核。

### 7.1 目标

不强行把所有资源做成同一种生命周期，但统一“用词、删除边界、错误提示”。

### 7.2 必做的资源模型调整

#### Projects

- 保持 `active | archived`
- 已新增“可删除但需未被引用”的删除能力

#### ConfigFiles

- 保持 `active | archived`
- 已新增“可删除但需未被引用”的删除能力

#### ProjectEnvironments

- 保持 `active | inactive`
- 明确未被引用时可删除

#### Users

- 统一为 `active | disabled`
- 不物理删除

### 7.3 文案收口

统一这些术语在前端、错误码、文档中的含义：

- `active`
- `inactive`
- `archived`
- `disabled`
- `delete`
- `restore`

### 7.4 后端实施项

- `projects` 删除前引用检查已完成
- `config_files` 删除前引用检查已完成
- 相关错误码已补齐
- OpenAPI 与接口文档已同步

### 7.5 前端实施项

- 项目页已补删除入口和确认逻辑
- 平台项目列表已补删除入口和确认逻辑
- 配置文件页已补删除入口和确认逻辑
- 后续继续复核状态 badge、表单选项、错误提示边角一致性

## 8. Phase 5: 低风险页面补齐

这些页面不应先于前四个阶段，但在骨架完成后应尽快补齐。

状态：项目成员页、sync records、heartbeats、audit logs 均已完成真实页面和主路径 E2E。

### 8.1 必补页面

- 项目成员页（已完成）
- sync records 页面（已完成）
- heartbeats 页面（已完成）
- audit logs 页面（已完成）
- 创建项目入口改造后的平台项目创建流程（已完成）

### 8.2 前端验收

- admin/editor/viewer 的入口展示符合权限矩阵
- 非权限用户进入页面时提示明确
- 从页面可以完成真实项目协作闭环，而不是只存在后端接口

## 9. Phase 6: 文档压缩

前几阶段完成后，把当前文档体系收成三层：

### 保留

- `docs/constraints/`
- `docs/runbooks/`
- `docs/public/`

### 迁移 / 压缩

- 中间 handoff
- 阶段性 checklist
- 已完成的过渡文档

建议原则：

- `constraints/` 只留最终规则
- `runbooks/` 只留部署 / 初始化 / 运维 / 恢复
- 历史阶段文档移到 `archive/`

## 10. Phase 7: 最后推进 Config Workspace

只有在前六阶段完成后，再推进：

- Draft 编辑统一升级
- Release 只读代码视图
- Diff 统一升级
- Merge Workspace

原因：

- 它重要，但不是当前上线运营骨架的 blocker
- 等平台权限、初始化和安全基线稳定后再做，返工更少

## 11. 明确不在本轮优先推进的事项

以下内容当前不应抢占前述阶段：

- 批量 merge / 批量发布
- AI merge 建议
- SSO / OAuth
- Scope / labels 主模型扩展
- 增量拉取
- 灰度发布

## 12. 建议的提交批次

已完成批次：

1. `docs-sync-and-compaction`
2. `open-api-security-baseline`

后续建议按下面批次推进，避免一个分支里混太多横向变化：

1. `launch-runbooks`
2. `resource-lifecycle-alignment`
3. `operations-pages`
4. `config-workspace`

## 13. 最终上线前验收清单

上线前至少确认：

1. 能从空数据库初始化出首个平台管理员
2. 平台管理员可创建用户、创建项目并指定首个项目管理员
3. 平台管理员默认看不到未加入项目的业务数据
4. 项目管理员可完成配置中心主业务闭环
5. 被禁用用户无法继续登录
6. 管理端安全基线已启用
7. Open API 限流与日志已启用
8. 项目、配置文件、环境、实例的生命周期文案一致且删除边界明确
9. 运维查询页面可支撑真实排障
10. 部署 runbook 和初始化 runbook 可被独立执行
