# Frontend MVP Blueprint

## 1. 文档目标

这份文档用于固定前端 MVP 页面、主要交互、依赖接口和业务限制，保证后续实现能更准确还原当前确认过的产品语义。

原则：

- 按页面和流程组织，而不是按接口组织
- 只记录 MVP 要求，不展开未来增强方案
- 与 `docs/constraints/product-qa/` 的规则澄清保持一致

## 2. 全局前提

- 管理端登录态基于 HttpOnly Session Cookie
- 前端默认通过 `/api/auth/me` 判断当前登录状态
- 前端第一版就应感知项目角色，并按 `admin / editor / viewer` 控制高风险入口
- 后端仍然是最终权限真值，前端按钮隐藏只用于改善体验
- 非成员访问项目资源时，前端按资源未命中处理
- 成员但角色不足时，前端按 `403 project_permission_denied` 处理
- `Template` 是 `DeploymentInstance` 的特殊实例
- `publish` 是“实例下单配置文件发布”，不是整实例一键发布
- 项目级 `ConfigFile.is_required` 决定发布门槛

## 3. 登录页

页面目标：

- 让管理端用户登录

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
- `code` 在 UI 中统一按“配置标识”理解，不使用“编码”表述
- 当前配置文件主路径支持 `yaml / json / toml`；`text` 不在当前范围内

## 7. 项目环境页

页面目标：

- 管理项目下的环境对象
- 为部署实例创建提供受控环境集合

依赖接口：

- `GET /api/projects/:id/environments`
- `POST /api/projects/:id/environments`
- `GET /api/projects/:id/environments/:environmentId`
- `PUT /api/projects/:id/environments/:environmentId`
- `DELETE /api/projects/:id/environments/:environmentId`

关键交互：

- 列表按 `sort_order + code` 排序
- 支持创建、编辑、停用、删除未引用环境
- 空项目先提示“先创建环境，再创建部署实例”

失败提示：

- `project_environment_code_conflict`
- `project_environment_in_use`

## 8. 部署实例列表与详情页

页面目标：

- 管理项目下实例与模板
- 区分普通实例和模板实例

依赖接口：

- `GET /api/deployment-instances?project_id=:projectId&page=1&page_size=20`
- `GET /api/projects/:projectId/environments`
- `POST /api/deployment-instances`
- `GET /api/deployment-instances/:id`
- `PUT /api/deployment-instances/:id`
- `POST /api/deployment-instances/:id/activate`
- `POST /api/deployment-instances/:id/deactivate`

加载态 / 空状态 / 缺权限状态：

- 无实例时提示先创建实例或模板

表单字段与校验：

- `project_id`
- `environment_id`
- `deployment_key`
- `name`
- `description`
- `is_template`

关键交互：

- 列表应拆成“模板”和“部署实例”两个区块，避免模板与真实实例混排
- 首版可在前端基于 `is_template` 对同一接口响应分组；如数量继续增长，再考虑后端增加 `is_template` 查询参数
- 支持筛选 `environment_id` / `status`
- 列表使用分页响应中的 `items / total / page / page_size`
- 环境通过受控下拉选择，不再允许自由文本输入
- 无 active 环境时，创建入口置灰并提示先去环境管理
- 新建实例默认为 `inactive`
- 激活实例时展示一次性 token；停用后旧 token 应立即失效
- `PUT` 只允许修改 `environment_id / deployment_key / name / description`

后续归档需求：

- 当前部署实例状态仍只有 `active / inactive`
- 如需归档后默认隐藏，应新增 `is_archived` 软归档维度，不应把 `archived` 加回 `status`
- 已归档实例恢复后只能回到 `inactive`，不能直接恢复为 `active`
- 如需释放 `deployment_key`，应通过严格确认后的产品层 delete 完成
- delete 后底层保留 tombstone row 和 `deployment_uid`，用于审计和历史页面区分同 key 不同实体

失败提示：

- `deployment_key_conflict`
- `project_not_found`

与当前产品规则绑定的限制：

- Template 不能直接发布

## 9. 模板创建实例流程

页面目标：

- 从模板快速创建新实例

依赖接口：

- `POST /api/deployment-instances/:id/clone`

加载态 / 空状态 / 缺权限状态：

- 只有模板实例展示“创建实例”入口

表单字段与校验：

- `deployment_key`
- `name`
- `environment_id`
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

## 10. Draft 编辑页

页面目标：

- 以“配置工作台”形式管理某个实例下单个配置文件的 Current Draft
- 让历史 Saved Versions 和 Releases 成为可恢复的只读来源，而不是并列可编辑 Draft

依赖接口：

- `GET /api/drafts/:deploymentId/:configFileId`
- `PUT /api/drafts/:deploymentId/:configFileId`
- `GET /api/draft-saved-versions?deployment_instance_id=&config_file_id=`
- `GET /api/draft-saved-versions/:id`
- `PATCH /api/draft-saved-versions/:id`
- `POST /api/draft-saved-versions/:id/restore`
- `DELETE /api/draft-saved-versions/:id`
- 当前后端和 Draft 编辑页前端均已接入 Saved Versions 主路径

加载态 / 空状态 / 缺权限状态：

- Current Draft 不存在时，前端进入“新建 Draft”态
- Saved Versions / Releases 面板为空时，显示空状态而不是隐藏整个历史区

表单字段与校验：

- `content`
- `format`
- `base_version`

关键交互：

- 编辑器严格只对应 Current Draft
- 显示当前版本号
- 保存时传递 `base_version`
- 冲突时提示用户刷新
- 页面层级建议采用：
  - 左栏：配置文件导航
  - 中栏：Current Draft 编辑区
  - 右栏：Saved Versions / Releases 历史面板

失败提示：

- `draft_version_conflict`
- `draft_validation_failed`

与当前产品规则绑定的限制：

- 同一实例同一配置文件只有一份 Current Draft
- 历史记录不应继续命名为多个 Draft

## 11. Saved Versions 历史面板

页面目标：

- 给用户提供从历史保存内容快速找回继续编辑的入口

依赖接口：

- 当前后端已提供：
  - `GET /api/draft-saved-versions?deployment_instance_id=&config_file_id=`
  - `GET /api/draft-saved-versions/:id`
  - `POST /api/draft-saved-versions/:id/restore`
  - `PATCH /api/draft-saved-versions/:id`
  - `DELETE /api/draft-saved-versions/:id`
- 当前仍未提供：
  - `POST /api/draft-saved-versions`

加载态 / 空状态 / 缺权限状态：

- 无历史保存版本时显示空状态

关键交互：

- 每次保存 Current Draft 后生成一条 Saved Version
- 自动生成基于时间的默认标题
- 允许用户追加备注 `note`
- Saved Version 只读，不直接编辑
- 可执行：
  - 查看
  - 对比 Current Draft
  - 恢复到 Current Draft
  - 删除历史版本

与当前产品规则绑定的限制：

- Saved Versions 不是并行可编辑 Draft 分支
- 发布时不能直接发布某条 Saved Version，必须先恢复到 Current Draft
- Saved Versions 属于工作过程数据，首版只对 `admin / editor` 可见

## 12. 单配置 Clone 弹窗 / 流程

页面目标：

- 从另一个实例复制单个配置文件到当前实例 Draft

依赖接口：

- `GET /api/clone-sources`
- `POST /api/drafts/:targetDeploymentId/:configFileId/clone`

加载态 / 空状态 / 缺权限状态：

- 只有进入某个具体 `config_file` 编辑态时才展示 clone 入口

表单字段与校验：

- `source_deployment_instance_id`
- `source_kind`

关键交互：

- 允许从同项目内其他实例选择可复制来源
- 来源列表应使用后端 clone-sources 专用接口，不再复用通用部署实例列表
- 来源选择支持远程搜索、分页加载和 `draft / latest_release` 可用性展示
- 前端批量 clone 时，通过多次调用单配置 clone 完成

失败提示：

- 跨项目来源被拒绝
- 来源实例缺少对应 Draft / Release

与当前产品规则绑定的限制：

- 不提供后端批量 clone 接口
- 单配置 clone 默认覆盖目标 Draft，并递增版本

## 13. 整实例预览页

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

## 14. Release 历史页

页面目标：

- 浏览实例 / 配置文件的发布历史
- 作为只读回看来源，不直接编辑
- 提供不可编辑文本框直接回看某次 Release 内容

依赖接口：

- `GET /api/releases`
- `GET /api/releases/:id`

加载态 / 空状态 / 缺权限状态：

- 无历史发布时显示空状态

关键交互：

- 支持按 `deployment_instance_id` / `config_file_id` 过滤
- 进入详情后查看返回的 `content`、`revision`、`change_summary`
- 详情页使用不可编辑文本框展示 `content`
- secret 配置按后端脱敏后的内容展示
- 在显眼位置展示：
  - `published_at`
  - `published_by`
  - `published_by_username`（若后端可提供）
- 可执行：
  - 查看
  - 对比 Current Draft
  - 恢复到 Current Draft

当前状态：

- Release 列表已实现
- Release 详情 / Diff 前端路由仍需从占位页补为真实页面

## 15. Diff 对比页

页面目标：

- 比较不同发布版本之间的差异

依赖接口：

- `GET /api/releases/:id/diff`

关键交互：

- 以前后文本直接驱动 DiffEditor
- 固定展示“当前 release 与上一版 release”的对比
- secret 配置按后端脱敏后的内容展示

## 16. 发布确认流程

页面目标：

- 发布某个实例下单个配置文件的 Current Draft

依赖接口：

- `POST /api/releases/publish`

关键交互：

- 发布前建议先跳转预览页或内嵌预览摘要
- 发布失败时直接透出业务错误码
- 如果用户想发布历史 Saved Version 或历史 Release：
  - 必须先恢复到 Current Draft
  - 再发布 Current Draft

失败提示：

- `deployment_instance_template_publish_forbidden`
- `required_config_missing`
- `draft_not_found`

与当前产品规则绑定的限制：

- Template 不允许发布
- 任一必选配置缺失都会阻塞当前这次单配置发布

## 17. 项目成员页

页面目标：

- 管理项目成员和角色

依赖接口：

- `GET /api/projects/:id/members`
- `POST /api/projects/:id/members`
- `PUT /api/projects/:id/members/:memberId`
- `DELETE /api/projects/:id/members/:memberId`

加载态 / 空状态 / 缺权限状态：

- 无成员异常空状态时，至少应保留创建者可见
- 非 admin 不展示写操作入口

表单字段与校验：

- `username`
- `role`

关键交互：

- admin 可添加、修改、删除成员
- 删除或降级最后一个 admin 时要展示明确错误提示

失败提示：

- `project_member_conflict`
- `user_not_found`
- `last_project_admin_required`

## 16. 同步记录页

页面目标：

- 查看实例配置应用结果和回传记录

依赖接口：

- `GET /api/deployment-sync-records`

关键交互：

- 支持按实例、配置文件、动作、状态筛选
- 筛选字段使用 `deployment_instance_id`、`config_file_id`、`action`、`status`
- 明确展示 `config`、`revision`、`action`、`status`、`reported_at`

与当前产品规则绑定的限制：

- `admin / editor / viewer` 都可查看

## 17. 心跳页

页面目标：

- 查看实例最近一次配置组件心跳状态

依赖接口：

- `GET /api/deployment-heartbeats`

关键交互：

- 支持按实例和 `config_file_id` 筛选
- 展示 `reported_at`、`config` 和最近状态

与当前产品规则绑定的限制：

- 当前接口只返回最近状态，不直接给出在线/离线结论
- `admin / editor / viewer` 都可查看

## 18. 审计日志页

页面目标：

- 查看高风险操作和关键认证事件

依赖接口：

- `GET /api/audit-logs`

关键交互：

- 支持按 `project_id`、`user_id`、`action`、`resource_type` 过滤
- 明确展示安全元数据，不尝试渲染被裁剪的敏感内容

与当前产品规则绑定的限制：

- 仅项目 `admin` 可查看
- 不展示或拼接 Draft / Release 明文、原始 token、完整 diff 文本
