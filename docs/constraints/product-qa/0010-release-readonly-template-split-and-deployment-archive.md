# 0010 Release Readonly / Template Split / Deployment Archive / Delete 澄清

## Q1: Draft / preview-bundle / publish 主路径现在是否闭环？

当前代码已经基本打通配置中心核心管理闭环：

- 在实例配置下编辑 Current Draft
- 通过 preview-bundle 查看整实例配置包
- 发布 Current Draft 为 Release
- 保存 Current Draft 时生成 Saved Version
- 从 Saved Version、最新 Release 或其他实例来源恢复到 Current Draft

当前剩余问题主要不是“发布链路不闭环”，而是页面导航和历史回看体验仍偏工程化。

## Q2: 为什么还需要 Release 只读回看页？

当前后端已经提供：

- `GET /api/releases`
- `GET /api/releases/:id`
- `GET /api/releases/:id/diff`

前端已有 Release 列表，但 Release 详情 / Diff 路由仍是占位页。因此用户缺少一个直接打开历史发布内容的只读页面。

Release 回看页应是只读页面，不允许直接编辑 Release 内容。

建议首版能力：

- 展示 `revision`
- 展示 `deployment_instance`
- 展示 `config_file`
- 展示 `published_at`
- 显眼展示 `published_by` / `published_by_username`
- 展示 `change_summary`
- 用不可编辑文本框展示后端返回的 `content`
- secret 配置直接显示后端脱敏后的内容与 redaction 标记
- 提供“恢复到 Current Draft”入口，恢复后用户再进入编辑器继续修改
- 提供“查看 Diff”入口

## Q3: 模板和普通部署实例是否应该继续混在同一个列表？

不建议继续混排。

当前模板通过 `deployment_instances.is_template` 区分，本质上仍是部署实例记录；但使用场景不同：

- 模板用于复制配置，不能 activate、deactivate、reset token、publish
- 普通实例用于真实消费配置，支持 activate、deactivate、token reset、preview、publish

因此前端首版应把同一个接口返回的数据分成两个区块展示：

1. **模板**
   - 只展示模板实例
   - 主操作是“创建实例”
   - 可进入详情查看和维护 Draft
2. **部署实例**
   - 只展示普通实例
   - 主操作是详情、激活、停用、token reset、预览、发布

首版可以不新增后端接口，直接基于 `is_template` 在前端分组；如果后续模板数量和普通实例数量都变大，再考虑后端增加 `is_template` 查询参数或专用列表接口。

## Q4: Deployment archive / delete 应该如何建模？

当前代码明确把 deployment 运行态收口为：

```text
inactive
active
```

这部分不建议改成三态 `active / inactive / archived`。

推荐把三类含义拆开：

```text
status = active | inactive     # 运行态
is_archived = true | false      # 可恢复隐藏
deleted_at is null | not null   # 产品层删除，释放 deployment_key
```

这样用户心智更清晰：

- `inactive`：未启用或已停用，仍在日常管理范围内
- `active`：正在对 Open API 服务
- `archived`：收起来，可恢复，不释放 `deployment_key`
- `deleted`：从当前资源中删除，不可恢复，释放 `deployment_key`

## Q5: 为什么需要 `deployment_uid`？

当前 Open API 请求仍要求客户端携带 `project + environment + deployment_key + config`，并同时携带 Bearer token。服务端用 token 找到凭证所属的 `deployment_instance_id`，再校验请求中的 `deployment_key` 是否指向同一实例。

因此 `deployment_key` 既是人类可读标识，也是客户端接入参数。为了允许删除后复用 `deployment_key`，不能再把 `deployment_key` 当成历史审计的唯一身份。

建议为部署实例增加系统生成、不可复用的内部身份：

```text
deployment_uid UUID NOT NULL
```

语义：

- `deployment_uid` 是实体真实身份
- `deployment_key` 是用户可读、可复用的当前标识
- audit log、Release 历史、sync/heartbeat 历史应记录或能关联到 `deployment_uid`
- UI 默认不展示裸 UID，只在需要区分同名历史实例时显示“已删除于 ...”等业务文案

不建议引入完整 generation / 代次系统。`deployment_uid + deleted_at` 已经足够区分同名不同生命期。

## Q6: 删除是否应该物理删除数据库行？

不建议物理删除 `deployment_instances` 行。

推荐采用 **tombstone delete**：

- 对用户来说，实例已删除，不在默认列表、不在归档列表、不可恢复
- 对系统来说，`deployment_instances` 行仍保留 `deployment_uid / deployment_key / deleted_at / deleted_by`
- `deployment_key` 通过 partial unique index 释放给新实例复用
- 历史 Release、sync records、heartbeats、audit logs 仍能解释旧实体

这不是“只做 archive”，而是产品层删除。区别在于：

- `archive` 可恢复，不释放 key
- `delete` 不可恢复，释放 key，但保留 tombstone 用于审计和历史解释

## Q7: 推荐数据库字段是什么？

建议在 `deployment_instances` 增加：

```text
deployment_uid uuid not null
is_archived boolean not null default false
archived_at timestamptz null
archived_by bigint null
archive_reason text null
deleted_at timestamptz null
deleted_by bigint null
delete_reason text null
```

约束建议：

```sql
CHECK (NOT is_archived OR status = 'inactive')
CHECK (deleted_at IS NULL OR status = 'inactive')
```

唯一键建议从普通唯一约束改成 partial unique：

```sql
CREATE UNIQUE INDEX deployment_instances_live_key_unique
ON deployment_instances (project_id, environment_id, deployment_key)
WHERE deleted_at IS NULL;
```

效果：

- 未删除实例仍不能重 key
- archived 实例仍占用 key
- deleted 实例释放 key
- 新建同 key 实例会得到新的 `deployment_uid`

## Q8: Archive / Restore / Delete API 应该怎样定义？

建议新增专用接口，不复用通用 `PUT` 修改状态。

### Archive

```http
POST /api/deployment-instances/:id/archive
```

规则：

- 仅 `admin / editor`
- 仅允许 `inactive` 实例
- active 实例必须先 deactivate，再 archive
- 设置 `is_archived = true`
- 写入 `archived_at / archived_by / archive_reason`
- 不释放 `deployment_key`
- 写 audit log：`deployment_instance.archived`

### Restore

```http
POST /api/deployment-instances/:id/restore
```

规则：

- 仅 `admin / editor`
- 仅允许 archived 且未 deleted 的实例
- 设置 `is_archived = false`
- 清空或保留 archive metadata 需要实现前定；首版建议清空当前状态字段，历史由 audit log 保留
- `status` 保持或设置为 `inactive`
- 不生成 token
- 写 audit log：`deployment_instance.restored`

### Delete

```http
DELETE /api/deployment-instances/:id
```

规则：

- 仅 `admin / editor`
- 仅允许 archived 且 inactive 的实例
- 不物理删除 `deployment_instances` 行
- 设置 `deleted_at / deleted_by / delete_reason`
- 立即撤销或删除 deployment credentials
- 删除或软删除 Current Draft 和 Saved Versions 等可变工作区数据
- 保留 Release、sync records、heartbeats、audit logs 等历史事实
- 释放 `deployment_key`
- 写 audit log：`deployment_instance.deleted`

## Q9: 列表查询应如何处理？

`GET /api/deployment-instances` 建议新增：

```text
visibility_filter=current | archived | all
```

默认：

```text
visibility_filter=current
```

语义：

- `current`：`deleted_at IS NULL AND is_archived = false`
- `archived`：`deleted_at IS NULL AND is_archived = true`
- `all`：返回未 deleted 的全部实例

deleted 实例不进入日常列表和归档列表。它们只在历史 Release、sync/heartbeat、audit 等上下文里作为历史实体显示。

## Q10: 归档和删除后哪些操作应被禁止？

archived 实例只允许：

- 查看基础详情
- 查看只读历史
- restore
- delete

deleted 实例只允许在历史上下文里只读展示。

这些接口应拒绝 archived / deleted：

- `POST /api/deployment-instances/:id/activate`
- `POST /api/deployment-instances/:id/deactivate`
- `POST /api/deployment-instances/:id/token/reset`
- `PUT /api/deployment-instances/:id`
- `GET /api/deployment-instances/:id/preview-bundle`
- `POST /api/releases/publish`
- `GET /api/clone-sources`
- `POST /api/drafts/:targetDeploymentId/:configFileId/clone`

Draft 读写建议：

- archived：可读基础详情，工作区写操作拒绝
- deleted：不允许作为工作区目标读取或编辑，只在历史页面透出历史快照

错误码建议：

```text
deployment_instance_archived
deployment_instance_deleted
deployment_instance_delete_requires_archived
deployment_instance_delete_requires_inactive
```

## Q11: Audit log 应该怎么记录？

不要在删除前批量改旧 audit log。

audit log 是事件事实，应采用 append-only 思路：

- 每条事件写入当时的 `deployment_uid`
- 同时写入当时的 `deployment_key` / `deployment_name` / `environment_code` 快照
- 删除时追加 `deployment_instance.deleted` 事件
- UI 根据 `deployment_uid` 和 `deleted_at` 区分同 key 不同实体

示例 detail：

```json
{
  "deployment_instance_id": 123,
  "deployment_uid": "4e8f2d8c-9f2f-4a8d-9f1f-f7b0e8b1c123",
  "deployment_key": "store-001",
  "deployment_name": "Store 001",
  "environment_id": 9,
  "environment_code": "prod"
}
```

UI 文案建议：

```text
store-001
store-001 · 已删除于 2026-04-20 18:42
store-001 · 当前实例
```

不要让用户理解 `gen 1 / gen 2` 这类内部概念。

## Q12: 前端应如何展示？

部署实例页：

- 默认展示未 archived、未 deleted 的实例
- 分成“模板”和“部署实例”两个区块
- 提供“已归档部署实例”入口
- 不展示 deleted 实例

已归档弹窗 / 抽屉：

- 展示模板和普通实例
- 展示 `archived_at / archived_by / archive_reason`
- 操作：
  - 恢复为未激活
  - 删除

删除确认：

- 明确说明删除不可恢复
- 明确说明删除后会释放 `deployment_key`
- 如果旧实例有历史 Release / sync / heartbeat，说明这些历史仍会保留在历史页面和审计里

历史页面：

- 对 deleted 实体显示删除时间
- 如果同一 `deployment_key` 当前已有新实例，提示“当前同名实例是另一实体”

## Q13: 推荐实施顺序是什么？

建议分三批：

1. **Release 只读回看页**
   - 风险最低
   - 后端接口已有
   - 能立即补齐发布历史体验

2. **部署实例列表拆成模板 / 普通实例两个区块**
   - 首版可前端完成
   - 不改后端模型
   - 直接改善查找成本

3. **Deployment archive + tombstone delete 生命周期**
   - 增加 `deployment_uid`
   - 增加 `is_archived / archived_* / deleted_*`
   - 改唯一键为 `deleted_at IS NULL` partial unique
   - 增加 archive / restore / delete API
   - 增加默认过滤和 archived 查询
   - 为 Open API、publish、preview、clone、Draft 写操作补 guard
   - 补 OpenAPI、后端集成测试和前端 E2E

## 当前结论

- Draft / preview-bundle / publish 主链路已经基本闭环。
- Release 仍缺前端只读详情 / Diff 页面。
- 模板和普通实例应在前端拆成两个列表区块。
- Deployment archive 不应建成 `status = archived`。
- 推荐 `deployment_uid + is_archived + deleted_at`：
  - archive 可恢复，不释放 `deployment_key`
  - delete 不可恢复，释放 `deployment_key`
  - tombstone row 保留历史解释能力
