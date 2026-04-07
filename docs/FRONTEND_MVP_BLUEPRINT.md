# Frontend MVP Blueprint

## 1. 文档目标

这份文档用于固定前端 MVP 页面、主要交互、依赖接口和业务限制，保证后续实现能更准确还原当前确认过的产品语义。

原则：

- 按页面和流程组织，而不是按接口组织
- 只记录 MVP 要求，不展开未来增强方案
- 与 `docs/product-qa/` 的规则澄清保持一致

## 2. 全局前提

- 管理端登录态基于 HttpOnly Session Cookie
- 前端默认通过 `/api/auth/me` 判断当前登录状态
- 暂不做细粒度权限 UI，先按“管理员已登录”语义设计
- `Template` 是 `DeploymentInstance` 的特殊实例
- `publish` 是“实例下单配置文件发布”，不是整实例一键发布
- 项目级 `ConfigFile.is_required` 决定发布门槛

## 3. 登录页

页面目标：

- 让管理员登录管理端

依赖接口：

- `POST /api/auth/login`
- `GET /api/auth/me`

加载态 / 空状态 / 缺权限状态：

- 初始加载时调用 `/api/auth/me`
- 已登录直接跳转项目列表
- 登录失败显示统一错误提示

表单字段与校验：

- `username`
- `password`

关键交互：

- 回车提交
- 登录成功后跳转项目列表

失败提示：

- 用户名或密码错误
- 会话过期时统一跳回登录页

## 4. 项目列表页

页面目标：

- 浏览项目
- 创建项目
- 进入项目详情

依赖接口：

- `GET /api/projects`
- `POST /api/projects`

加载态 / 空状态 / 缺权限状态：

- 空状态引导创建首个项目
- 未登录时跳转登录页

表单字段与校验：

- `code`
- `name`
- `description`

关键交互：

- 列表按 `code` 展示
- 创建后刷新列表并进入详情

失败提示：

- `project_code_conflict`

## 5. 项目详情页

页面目标：

- 展示项目基础信息
- 作为配置文件、部署实例、发布历史等子页入口

依赖接口：

- `GET /api/projects/:id`
- `PUT /api/projects/:id`

加载态 / 空状态 / 缺权限状态：

- 项目不存在时跳回列表并提示

表单字段与校验：

- `code`
- `name`
- `description`
- `status`

关键交互：

- 基础信息编辑
- 导航到配置文件页、部署实例页、发布历史页

## 6. 配置文件列表与编辑页

页面目标：

- 管理项目下配置文件清单
- 设置 `is_required`

依赖接口：

- `GET /api/config-files?project_id=:projectId`
- `POST /api/config-files`
- `GET /api/config-files/:id`
- `PUT /api/config-files/:id`

加载态 / 空状态 / 缺权限状态：

- 无配置文件时提示创建

表单字段与校验：

- `project_id`
- `code`
- `name`
- `format`
- `schema_name`
- `schema_version`
- `sensitivity`
- `secret_paths`
- `description`
- `is_required`
- `status`

关键交互：

- 在列表中清晰区分“必选”与“可选”配置
- 可在详情抽屉或页内编辑 `is_required`

失败提示：

- `config_file_code_conflict`
- `project_not_found`

与当前产品规则绑定的限制：

- `is_required` 是项目级规则，不支持按环境或实例覆盖

## 7. 部署实例列表与详情页

页面目标：

- 管理项目下实例与模板
- 区分普通实例和模板实例

依赖接口：

- `GET /api/deployment-instances?project_id=:projectId`
- `POST /api/deployment-instances`
- `GET /api/deployment-instances/:id`
- `PUT /api/deployment-instances/:id`

加载态 / 空状态 / 缺权限状态：

- 无实例时提示先创建实例或模板

表单字段与校验：

- `project_id`
- `environment`
- `deployment_key`
- `name`
- `description`
- `is_template`
- `status`

关键交互：

- 列表显式区分模板
- 支持筛选 `environment` / `status`

失败提示：

- `deployment_key_conflict`
- `project_not_found`

与当前产品规则绑定的限制：

- Template 不能直接发布

## 8. 模板创建实例流程

页面目标：

- 从模板快速创建新实例

依赖接口：

- `POST /api/deployment-instances/:id/clone`

加载态 / 空状态 / 缺权限状态：

- 只有模板实例展示“创建实例”入口

表单字段与校验：

- `deployment_key`
- `name`
- `environment`
- `description`
- `clone_source`

关键交互：

- `clone_source` 第一版固定为 `draft`
- 成功后跳转新实例详情页

失败提示：

- 模板不存在
- `deployment_key_conflict`

与当前产品规则绑定的限制：

- 模板 clone 不允许 `latest_release`

## 9. Draft 编辑页

页面目标：

- 编辑某个实例下单个配置文件的当前 Draft

依赖接口：

- `GET /api/drafts/:deploymentId/:configFileId`
- `PUT /api/drafts/:deploymentId/:configFileId`

加载态 / 空状态 / 缺权限状态：

- Draft 不存在时，前端进入“新建 Draft”态

表单字段与校验：

- `content`
- `format`
- `base_version`

关键交互：

- 显示当前版本号
- 保存时传递 `base_version`
- 冲突时提示用户刷新

失败提示：

- `draft_version_conflict`
- `draft_validation_failed`

## 10. 单配置 Clone 弹窗 / 流程

页面目标：

- 从另一个实例复制单个配置文件到当前实例 Draft

依赖接口：

- `POST /api/drafts/:targetDeploymentId/:configFileId/clone`

加载态 / 空状态 / 缺权限状态：

- 只有进入某个具体 `config_file` 编辑态时才展示 clone 入口

表单字段与校验：

- `source_deployment_instance_id`
- `source_kind`

关键交互：

- 允许从同项目内其他实例选择来源
- 前端批量 clone 时，通过多次调用单配置 clone 完成

失败提示：

- 跨项目来源被拒绝
- 来源实例缺少对应 Draft / Release

与当前产品规则绑定的限制：

- 不提供后端批量 clone 接口
- 单配置 clone 默认覆盖目标 Draft，并递增版本

## 11. 整实例预览页

页面目标：

- 让编辑者在发布前查看某个实例“当前实际将生效的整包配置效果”

依赖接口：

- `GET /api/deployment-instances/:id/preview-bundle`

加载态 / 空状态 / 缺权限状态：

- 无任何可展示配置时显示空状态

关键交互：

- 明确区分每个配置项来源：
  - `draft`
  - `latest_release`
  - `missing_optional`
  - `missing_required`
- 提供“复制 open bundle 预览 JSON”按钮

失败提示：

- 实例不存在

与当前产品规则绑定的限制：

- Draft 优先于 Release
- 必选配置缺失必须显式展示，不允许静默忽略

## 12. Release 历史页

页面目标：

- 浏览实例 / 配置文件的发布历史

依赖接口：

- `GET /api/releases`
- `GET /api/releases/:id`

加载态 / 空状态 / 缺权限状态：

- 无历史发布时显示空状态

关键交互：

- 支持按 `deployment_instance_id` / `config_file_id` 过滤
- 进入详情后查看原始 `content`、`revision`、`change_summary`

## 13. Diff 对比页

页面目标：

- 比较不同发布版本之间的差异

依赖接口：

- 预留 `GET /api/releases/:id/diff`

当前状态：

- 页面先作为前端蓝图保留
- 后端 diff 接口仍待实现

## 14. 发布确认流程

页面目标：

- 发布某个实例下的单个配置文件

依赖接口：

- `POST /api/releases/publish`

关键交互：

- 发布前建议先跳转预览页或内嵌预览摘要
- 发布失败时直接透出业务错误码

失败提示：

- `deployment_instance_template_publish_forbidden`
- `required_config_missing`
- `draft_not_found`

与当前产品规则绑定的限制：

- Template 不允许发布
- 任一必选配置缺失都会阻塞当前这次单配置发布
