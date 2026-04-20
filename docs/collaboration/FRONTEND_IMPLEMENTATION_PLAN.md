# 前端实施清单

## 1. 文档目标

这份文档给前端直接排期和拆任务用。

它不再重复解释业务背景，而是回答：

- 路由怎么拆更顺
- 页面第一版做到什么程度算完成
- 不同角色哪些按钮该显示
- 错误码在 UI 上建议怎么翻译
- 应该按什么顺序推进，才能最快打通主路径

## 2. 推荐路由结构

推荐按项目作为顶层上下文组织：

```text
/login
/projects
/projects/:projectId
/projects/:projectId/config-files
/projects/:projectId/config-files/:configFileId
/projects/:projectId/deployments
/projects/:projectId/deployments/:deploymentId
/projects/:projectId/deployments/:deploymentId/preview
/projects/:projectId/deployments/:deploymentId/configs/:configFileId/draft
/projects/:projectId/releases
/projects/:projectId/releases/:releaseId
/projects/:projectId/releases/:releaseId/diff
/projects/:projectId/members
/projects/:projectId/sync-records
/projects/:projectId/heartbeats
/projects/:projectId/audit-logs
```

推荐原则：

- 所有项目内资源都挂在 `projectId` 下面，减少上下文切换。
- `deploymentId + configFileId` 的 Draft 编辑页单独成路由，便于深链接与刷新恢复。
- release 详情和 diff 拆独立路由，方便复制链接和回跳。
- 运维页 `sync-records / heartbeats / audit-logs` 都保留在项目上下文下，避免做全局筛选器。

## 3. 第一版页面完成标准

### 3.1 登录页

完成标准：

- 首屏探测 `/api/auth/me`
- 未登录显示登录表单
- 已登录直接跳 `/projects`
- 登录失败显示统一错误提示

### 3.2 项目列表页

完成标准：

- 列出当前用户可见项目
- 支持创建项目
- 创建成功后跳转项目详情骨架页
- 空状态有“创建首个项目”引导

### 3.3 项目详情骨架

完成标准：

- 展示项目基础信息
- 提供子导航入口
- admin 可编辑基础信息
- editor / viewer 只读展示

### 3.4 配置文件页

完成标准：

- 列表展示 `code / name / format / is_required / sensitivity / status`
- 支持按 `status` 过滤
- admin 可新建和编辑
- secret 配置有明显标识

### 3.5 部署实例页

完成标准：

- 列表展示 `environment / deployment_key / name / is_template / status`
- 支持 `environment / status / keyword` 过滤
- 明确区分模板和普通实例
- 使用分页响应 `items / total / page / page_size`
- admin 可创建、编辑、clone、activate、deactivate、reset token
- 创建和 clone 后默认 `inactive`
- `active` 普通实例才显示 reset token

### 3.6 Draft 编辑页

完成标准：

- 展示当前 Draft 内容和版本号
- 支持保存
- 支持单配置 clone
- 能处理 `draft_version_conflict`
- 能处理 `draft_validation_failed`

### 3.7 发布主路径

完成标准：

- 能从 Draft 页进入发布确认
- 能查看 preview-bundle 或至少看到发布前摘要
- 能发起单配置 publish
- 能看到发布失败的明确业务原因
- 发布成功后跳转 release 详情或历史页

### 3.8 Release 历史 / 详情 / Diff

完成标准：

- 支持按实例和配置文件过滤
- 历史页能看 revision、发布时间、摘要
- 详情页能看返回的 `content`
- diff 页能用前后文本直接渲染对比
- secret 配置按后端脱敏结果展示

### 3.9 项目成员页

完成标准：

- admin 能查看、添加、修改、删除成员
- 能对最后一个 admin 的保护做友好提示
- 非 admin 不展示编辑入口

### 3.10 运维页

完成标准：

- sync records 能按实例、配置、状态过滤
- heartbeats 能按实例、配置文件查看最近状态
- 配置筛选使用 `config_file_id`，展示后端返回的 `config`
- audit logs 能按 action / resource_type / user 过滤

## 4. 页面级权限矩阵

### 4.1 项目与基础资料

- `admin`
  - 可编辑项目
  - 可管理成员
  - 可管理配置文件
  - 可管理部署实例
- `editor`
  - 只读查看项目、配置文件、部署实例
- `viewer`
  - 只读查看项目、配置文件、部署实例

### 4.2 Draft 与发布

- `admin`
  - 可编辑 Current Draft
  - 可单配置 clone
  - 可预览 preview-bundle
  - 可发布
- `editor`
  - 可编辑 Current Draft
  - 可单配置 clone
  - 可预览 preview-bundle
  - 可发布
- `viewer`
  - 不显示 Current Draft 编辑入口
  - 不显示 clone 入口
  - 不显示 preview 入口
  - 不显示发布入口

### 4.3 高风险操作

- `admin`
  - 可 activate / deactivate / reset token
  - 可查看 audit logs
  - 可模板 clone
- `editor`
  - 不显示 reset token
  - 不显示 audit logs
  - 不显示模板 clone 创建实例入口
- `viewer`
  - 不显示以上所有入口

### 4.4 运维查询

- `admin`
  - 可查看 sync records / heartbeats / audit logs
- `editor`
  - 可查看 sync records / heartbeats
  - 不显示 audit logs
- `viewer`
  - 可查看 sync records / heartbeats
  - 不显示 audit logs

## 5. 推荐错误码映射

建议优先接错误码，不要只展示原始 message。

### 5.1 认证类

- `auth_invalid_credentials`
  - 建议文案：用户名或密码错误
- `auth_session_expired`
  - 建议文案：登录状态已过期，请重新登录

### 5.2 权限类

- `project_permission_denied`
  - 建议文案：你当前角色没有执行这个操作的权限

### 5.3 项目与成员

- `project_code_conflict`
  - 建议文案：项目编码已存在
- `project_member_conflict`
  - 建议文案：该用户已经是项目成员
- `project_member_not_found`
  - 建议文案：项目成员不存在或已被移除
- `user_not_found`
  - 建议文案：目标用户不存在或未启用
- `last_project_admin_required`
  - 建议文案：项目至少需要保留一个管理员

### 5.4 配置与实例

- `config_file_code_conflict`
  - 建议文案：配置文件编码已存在
- `deployment_instance_conflict`
  - 建议文案：部署实例标识已存在
- `deployment_key_conflict`
  - 建议文案：部署实例标识已存在

### 5.5 Draft 与发布

- `draft_version_conflict`
  - 建议文案：草稿已被他人更新，请刷新后重试
- `draft_validation_failed`
  - 建议文案：草稿格式或内容校验失败
- `draft_not_found`
  - 建议文案：当前实例下还没有这份草稿
- `required_config_missing`
  - 建议文案：存在必选配置缺失，当前不能发布
- `deployment_instance_template_publish_forbidden`
  - 建议文案：模板实例不能直接发布
- `release_publish_failed`
  - 建议文案：发布失败，请检查当前草稿和实例状态

### 5.6 凭证与开放接口联动

- `deployment_token_reset_failed`
  - 建议文案：凭证重置失败，请稍后重试
- `invalid_token`
  - 这条主要给消费端，不是管理台文案主路径

## 6. 推荐实现阶段

### 阶段 A：壳子与登录态

交付物：

- Vue Router
- 路由守卫
- 会话探测
- 登录页
- 全局错误处理

完成标志：

- 能稳定区分“未登录”和“已登录”

### 阶段 B：项目上下文骨架

交付物：

- 项目列表页
- 项目详情页骨架
- 项目级导航
- 项目上下文 store

完成标志：

- 能从项目列表进入某个项目的子页面

### 阶段 C：配置与实例管理

交付物：

- 配置文件页（首轮已完成，后续只按验收反馈补强）
- 部署实例页
- 模板 clone
- activate / deactivate / token reset

完成标志：

- admin 能完成“建项目 -> 建配置 -> 建实例/模板”

### 阶段 D：Draft 与发布主路径

交付物：

- 配置工作台（Current Draft）
- 单配置 clone
- preview-bundle
- 发布确认
- Release 历史 / 详情 / Diff

完成标志：

- admin / editor 能走通“编辑 -> 预览 -> 发布 -> 查看历史”

### 阶段 E：成员与运维页

交付物：

- 项目成员页
- sync records
- heartbeats
- audit logs

完成标志：

- admin 能做权限管理和审计查看
- admin / editor / viewer 能看运维查询页

## 7. 数据层建议

推荐至少拆这些 store 或 composable：

- `useAuthSession`
- `useProjectContext`
- `useProjectPermissions`
- `useConfigFiles`
- `useDeploymentInstances`
- `useDraftEditor`
- `useSavedVersions`
- `useReleases`
- `useOperationsFilters`

推荐原则：

- 登录态和项目上下文不要混在一个大 store 里
- 过滤条件尽量和 URL query 同步
- Draft 编辑器的脏状态、版本号和保存状态单独管理

## 8. 对前端最容易踩坑的点

- 不要把“发布”理解成整实例发布。
- 不要把历史 Saved Version 直接当成可编辑 Draft；编辑器只对应 Current Draft。
- 不要自己拼 preview 逻辑，直接信后端 `preview-bundle`。
- 不要把 `404` 都当成真正不存在；在项目资源里它也可能表示“非成员不可见”。
- 不要自己实现 secret 字段脱敏算法；直接显示后端返回内容。
- 不要假设 reset token 后旧 token 还能用一段时间。
- 不要假设心跳页天然有在线/离线结论；当前接口只给最近状态。
- 不要为 deployment 实现 archived 状态；未启用和已停用都显示为 inactive。
- 不要再新增 `process_key`；客户端配置标识统一使用 `config`，管理端筛选使用 `config_file_id`。

## 9. 建议的首版验收清单

- admin 能完整走通：登录、建项目、建配置、建实例、激活实例、编辑 Current Draft、发布、看历史、管理成员、停用和重置 token
- editor 能完整走通：查看项目、编辑 Current Draft、预览、发布、看 sync records / heartbeats
- viewer 能完整走通：查看项目、看 release、看 sync records / heartbeats，但不能做写操作
- secret 配置在 release 详情和 diff 页不会显示明文
- 必选配置缺失时，发布页能给出明确阻塞提示
- Draft 冲突时，编辑页能提示刷新而不是静默覆盖
- 如果后续补 Saved Versions，用户能从历史保存版本恢复到 Current Draft 再继续发布
