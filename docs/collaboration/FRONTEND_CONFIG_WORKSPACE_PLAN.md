# 配置工作台详细改造方案

## 1. 目标

这份文档把“Current Draft + Saved Versions + Releases”落到可开发的前端工作台方案。

目标：

- 给实例配置提供稳定、可恢复、可发布的工作入口
- 明确 Current Draft 与历史记录的关系
- 让前端页面层级、交互动作、状态机和接口依赖可直接拆任务实现
- 约束 UI 方向，避免继续扩散成“多个并列可编辑 Draft”

## 2. 视觉与交互基调

### 2.1 Visual Thesis

管理台应采用克制、紧凑、低干扰的运维工作台风格：

- 信息密度高，但层级清楚
- 编辑器是主工作面
- 历史记录是侧边上下文，不抢主线
- 状态与操作优先于装饰

### 2.2 Content Plan

页面结构只保留三个职责：

1. 配置导航
2. Current Draft 编辑
3. 历史回看与恢复

不引入 hero、营销文案或大面积说明卡片。

### 2.3 Interaction Thesis

建议工作台至少具备三种明确的交互反馈：

1. 左栏配置切换时，当前项有稳定的状态标识与选中高亮
2. 右栏版本列表切换时，详情区做轻量内容过渡，不整页闪动
3. Current Draft 保存 / 发布 / 恢复后，页头状态摘要即时刷新

## 3. 术语与对象

统一使用以下命名：

- `Current Draft`
  - 唯一
  - 可编辑
  - 与编辑器一一对应
- `Saved Versions`
  - 历史保存版本
  - 只读
  - 可恢复到 Current Draft
- `Releases`
  - 已发布版本
  - 只读
  - 可恢复到 Current Draft

禁用以下命名：

- 多个 Draft
- Draft 列表
- 可编辑历史 Draft

## 4. 页面结构

建议新增主页面：

- `/projects/:projectId/deployments/:deploymentId/configs/:configFileId/workspace`

当前 [DraftEditorPage.vue](/home/zjj/Projects/mini-conf/apps/web/src/modules/drafts/pages/DraftEditorPage.vue) 的职责应逐步收敛为工作台中栏编辑区，而不是继续作为最终页面形态。

### 4.1 桌面布局

采用三栏布局：

- 左栏：`280px` 左右
  - 配置文件导航
  - 当前实例摘要
  - 进入 preview-bundle 的入口
- 中栏：弹性主列
  - Current Draft 页头
  - 编辑器
  - 底部保存状态和主要操作
- 右栏：`360px` 左右
  - 历史面板
  - 切换 `Saved Versions / Releases`
  - 详情摘要与恢复动作

布局原则：

- 编辑器是视觉中心
- 左右栏使用 plain layout，不做厚重卡片堆叠
- 只有版本详情抽屉、确认弹窗等真正需要边界的地方使用 card / panel

### 4.2 移动布局

移动端改为三段式：

1. 顶部配置切换器
2. Current Draft 编辑区
3. 底部 tabs：`History / Releases`

移动端不保留固定三栏，避免编辑器被挤压。

## 5. 工作台页面分区

### 5.1 页头

页头需要同时承担定位、状态与全局动作。

建议内容：

- 第一行：
  - 实例名
  - 配置标识
  - 环境
  - 是否模板
- 第二行状态摘要：
  - Current Draft 状态
  - 最近保存时间
  - Latest Release revision
  - Latest Release 发布时间
- 右侧动作：
  - `保存 Current Draft`
  - `预览配置包`
  - `发布 Current Draft`

模板实例页头规则：

- 不显示发布按钮
- 可保留编辑与 Saved Versions
- 明示“模板不能直接发布”

### 5.2 左栏：配置文件导航

每个配置项展示：

- 配置名称
- 配置标识 `code`
- 格式
- 状态 badge

状态 badge 建议只保留以下几种：

- `Current Draft`
- `Latest Release`
- `Missing Required`
- `Missing Optional`
- `Unconfigured`

辅助信息：

- 是否存在 Saved Versions
- 是否存在 Releases

交互：

- 点击切换当前 `configFileId`
- dirty 状态下需确认离开
- 提供 keyword 过滤，但不遮挡主操作

### 5.3 中栏：Current Draft 编辑区

结构：

- 编辑器工具条
  - 格式
  - 当前版本号
  - dirty 状态
  - 保存中状态
- 编辑器正文
- 底部状态栏
  - 最后保存时间
  - 最后编辑人
  - 错误提示区域

操作：

- 保存 Current Draft
- 丢弃 Current Draft
- 从 latest release 恢复
- 从其他实例复制到 Current Draft
- 预览配置包
- 发布 Current Draft

当前不建议在编辑器工具条中直接放“切换历史版本为编辑对象”的下拉。恢复动作应始终从右栏进入，保持来源清楚。

### 5.4 右栏：历史面板

顶部 segment：

- `Saved Versions`
- `Releases`

主体分两段：

1. 列表区
2. 详情区

列表区每项建议字段：

#### Saved Version 项

- 自动时间标题
- 用户备注
- 保存人
- 保存时间
- 来源标识

#### Release 项

- `revision`
- `change_summary`
- 发布人
- 发布时间

详情区建议展示：

- 元数据
- 内容摘要
- 对比入口
- 恢复到 Current Draft

## 6. 关键流程

### 6.1 保存 Current Draft

流程：

1. 用户编辑 Current Draft
2. 点击 `保存 Current Draft`
3. 发送 `PUT /api/drafts/:deploymentId/:configFileId`
4. 保存成功后：
   - 更新 Current Draft
   - 刷新页头状态
   - 刷新 Saved Versions 列表

产品规则建议：

- 若内容与最近一条 Saved Version 完全一致，可不重复生成
- 默认自动生成基于时间的标题
- 备注可采用两种方案：
  - A. 保存后补备注
  - B. 保存时附带可选备注输入

建议首版采用 A，减少保存路径阻力。

### 6.2 从 Saved Version 恢复

流程：

1. 用户在右栏选中某条 Saved Version
2. 查看详情或对比
3. 点击 `恢复到 Current Draft`
4. 二次确认提示将覆盖当前 Current Draft
5. 恢复成功后：
   - 中栏内容更新
   - dirty 状态归零
   - 页头状态刷新

### 6.3 从 Release 恢复

流程与 Saved Version 基本一致，但文案必须强调来源是已发布内容。

按钮文案建议：

- `恢复此发布版本到 Current Draft`

### 6.4 发布 Current Draft

流程：

1. 用户点击 `发布 Current Draft`
2. 打开确认层
3. 展示：
   - 当前实例
   - 当前配置文件
   - Current Draft 版本信息
   - latest release revision
   - 是否缺失 required config
4. 成功后：
   - 刷新 Releases 列表
   - 页头状态刷新
   - 保持留在工作台，不强制跳走

建议：

- 发布成功后给出次级入口：`查看本次 Release`
- 不建议强制跳转发布列表页，避免打断工作流

## 7. 页面状态机

### 7.1 Current Draft

- `loading`
- `new`
- `clean`
- `dirty`
- `saving`
- `conflict`
- `validation_error`
- `restoring`
- `publishing`

### 7.2 Saved Versions

- `idle`
- `loading`
- `empty`
- `ready`
- `restoring`
- `deleting`
- `error`

### 7.3 Releases

- `idle`
- `loading`
- `empty`
- `ready`
- `restoring`
- `error`

### 7.4 全局离开保护

以下动作都必须统一接 dirty guard：

- 切换配置文件
- 返回部署实例详情
- 跳转 preview-bundle
- 顶部项目 tabs
- 浏览器后退
- 刷新 / 关闭标签页

## 8. 接口与数据依赖

### 8.1 当前已可复用接口

- `GET /api/drafts/:deploymentId/:configFileId`
- `PUT /api/drafts/:deploymentId/:configFileId`
- `POST /api/drafts/:targetDeploymentId/:configFileId/clone`
- `GET /api/deployment-instances/:id/preview-bundle`
- `GET /api/releases`
- `GET /api/releases/:id`
- `GET /api/releases/:id/diff`
- `POST /api/releases/publish`

### 8.2 Saved Versions 接口（已落地）

- `GET /api/draft-saved-versions?deployment_id=&config_file_id=`
- `GET /api/draft-saved-versions/:id`
- `POST /api/draft-saved-versions/:id/restore`
- `PATCH /api/draft-saved-versions/:id`
- `DELETE /api/draft-saved-versions/:id`

说明：

- Saved Version 不通过独立 `POST` 手动创建；保存 Current Draft 时后端自动生成历史保存版本
- 如果最新 Saved Version 与当前保存内容完全相同，后端不会重复生成一条历史记录

### 8.3 Saved Version 数据字段

- `id`
- `deployment_instance_id`
- `config_file_id`
- `title`
- `note`
- `content`
- `content_hash`
- `format`
- `source_draft_version`
- `created_by`
- `created_by_username`
- `created_at`
- `deleted_at` 可选

### 8.4 Release 详情展示字段

Release 详情和右栏详情卡至少要有：

- `id`
- `revision`
- `change_summary`
- `content`
- `published_at`
- `published_by`
- `published_by_username`

## 9. 组件拆分建议

建议至少拆出以下组件：

- `ConfigWorkspacePage`
- `ConfigFileNavList`
- `CurrentDraftHeader`
- `CurrentDraftEditor`
- `WorkspaceHistoryPanel`
- `SavedVersionList`
- `ReleaseList`
- `SavedVersionDetail`
- `ReleaseDetail`
- `RestoreToDraftDialog`
- `PublishDraftDialog`

建议 composable：

- `useConfigWorkspace`
- `useCurrentDraft`
- `useSavedVersions`
- `useReleaseHistory`
- `useWorkspaceDirtyGuard`

## 10. 入口改造建议

### 10.1 实例详情页

当前入口问题：

- “编辑 Draft”语义太窄
- 用户无法预判是否有历史可恢复内容

建议改为：

- 主按钮：`打开工作台`
- 辅助信息列：
  - Current Draft：有 / 无
  - Saved Versions：数量
  - Latest Release：revision / 无

### 10.2 Preview 页

Preview 页每行建议提供：

- `编辑 Current Draft`
- `查看 Releases`
- `恢复 latest release 到 Current Draft`

不在 preview 页承载 Saved Versions 主列表，避免页面职责膨胀。

## 11. 权限建议

- `admin`
  - 可编辑 Current Draft
  - 可查看 Saved Versions / Releases
  - 可恢复历史内容
  - 可发布
- `editor`
  - 可编辑 Current Draft
  - 可查看 Saved Versions / Releases
  - 可恢复历史内容
  - 可发布
- `viewer`
  - 只读查看 Releases
  - 不显示 Current Draft 编辑区主操作
  - 不显示恢复与发布入口

## 12. 文案建议

推荐直接采用以下操作文案：

- `打开工作台`
- `保存 Current Draft`
- `丢弃 Current Draft`
- `恢复到 Current Draft`
- `恢复此发布版本到 Current Draft`
- `发布 Current Draft`
- `查看 Release`
- `编辑备注`
- `删除历史版本`

避免含糊文案：

- `编辑 Draft`
- `使用这个版本`
- `继续`
- `提交`

## 13. 实施顺序

### 阶段 1（已完成）

- 现有 Draft 编辑页继续承载 Current Draft 编辑、配置切换、Saved Versions 历史面板和发布入口
- 补 Release 只读详情页与 Diff 页
- Release 详情页提供不可编辑内容框、发布账号、恢复到 Current Draft

### 阶段 2（已完成）

- 将部署实例列表拆成“模板”和“部署实例”两个区块
- 模板区主操作聚焦“创建实例”
- 普通实例区主操作聚焦详情、激活、停用、token reset、预览和发布

### 阶段 3（已完成）

- 已引入 deployment archive + tombstone delete 生命周期
- 新增 `deployment_uid` 作为内部不可复用实体身份
- 默认列表排除 archived 和 deleted
- 通过已归档入口查看和恢复，恢复后状态为 `inactive`
- delete 后不可恢复并释放 `deployment_key`；历史页面通过 tombstone row 和 `deployment_uid` 区分同 key 旧实体
- 补后端 API、OpenAPI、前端交互和 E2E / 集成测试

### 下一阶段建议

- 继续收敛 `DraftEditorPage` 页面层职责，把 API 状态和副作用逐步抽入 `useDraftWorkspace`、`useSavedVersionsPanel`、`useCloneDraftSource` 等 composable
- 已完成统一代码工作区底座第一步：Draft 编辑页切到 CodeMirror 6，并拆出 workspace shell、配置导航和 Saved Versions 面板
- Release 详情 / Diff 升级为复用同一代码视图底座的只读语法高亮和差异视图
- 为高状态密度组件补前端单元 / 组件测试

## 14. 验收标准

- 用户能从实例详情页明确进入配置工作台
- 用户能在工作台内切换配置文件，不丢未保存编辑
- 用户能保存 Current Draft，并在历史面板看到 Saved Version
- 用户能从某条 Saved Version 恢复后继续编辑
- 用户能从某条 Release 回看发布人并恢复到 Current Draft
- 用户能始终明确“当前正在发布的是 Current Draft”
- 页面没有“多个可编辑 Draft 并存”的误导入口
