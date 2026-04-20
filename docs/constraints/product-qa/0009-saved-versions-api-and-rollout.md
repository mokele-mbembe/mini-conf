# 0009 Saved Versions 接口与渐进落地方案

## 背景

在 [0008-current-draft-saved-versions-and-release-workspace.md](./0008-current-draft-saved-versions-and-release-workspace.md) 中，已经明确了长期模型：

- 唯一 `Current Draft`
- 多条 `Saved Versions`
- 多条 `Releases`

接下来的问题不再是命名，而是如何在不打乱现有 Draft / Release 主路径的前提下，把 `Saved Versions` 逐步落地到后端接口、前端工作台和 OpenAPI。

这份文档用于固定：

- `Saved Versions` 的对象边界
- 最小接口集合
- 错误码
- 页面与接口的绑定方式
- 渐进上线顺序

## Q1: `Saved Versions` 是否应该复用现有 `drafts` 或 `releases` 表？

不建议。

原因：

- `drafts` 表示唯一 `Current Draft`
- `releases` 表示已发布真值
- `Saved Versions` 表示历史保存版本

三者职责不同：

- `Current Draft`：唯一、可编辑、会被覆盖
- `Saved Version`：只读、可删除、可恢复
- `Release`：只读、不可变、带发布审计

如果把 `Saved Versions` 混入 `drafts` 或 `releases`，会直接带来：

- 发布语义混乱
- 当前工作稿与历史版本边界不清
- 查询和 UI 文案都变复杂

## Q2: `Saved Versions` 的最小数据模型应该是什么？

建议新增独立表，例如：

```text
draft_saved_versions
```

建议字段：

```text
id
deployment_instance_id
config_file_id
title
note
content
content_hash
format
source_draft_version
created_by
created_at
updated_at
deleted_at nullable
```

补充说明：

- `title`
  - 默认自动生成，例如 `2026-04-20 18:42`
- `note`
  - 用户可选填写
- `source_draft_version`
  - 用于追踪这条历史版本来自哪个 Current Draft 修订号
- `deleted_at`
  - 如果后端希望保留审计，可采用软删除

当前不建议额外引入：

- `published_release_id`
- `branch_name`
- `candidate_status`

这些字段对应的是更重的候选分支语义，不属于当前目标。

## Q3: `Saved Versions` 是“每次保存自动生成”，还是“单独另存为版本”？

建议首版采用：

- 保存 Current Draft 后自动生成一条 Saved Version

但需要两个约束：

1. 若内容与最近一条 Saved Version 完全一致，则不重复生成
2. 允许前端后续补“编辑备注”，而不是把备注输入塞进每次保存流程

这样做的理由：

- 用户不需要额外理解“保存”和“另存为版本”的差异
- 历史找回路径稳定
- 当前主工作流阻力最小

如果后续发现保存频率过高，再考虑增加：

- `保存 Current Draft`
- `另存为版本`

两种显式动作分流。

## Q4: 最小接口集合应该是什么？

建议首版接口如下。

### 1. 列表

`GET /api/draft-saved-versions?deployment_id=:deploymentId&config_file_id=:configFileId`

响应示例：

```json
{
  "items": [
    {
      "id": 301,
      "deployment_instance_id": 18,
      "config_file_id": 7,
      "title": "2026-04-20 18:42",
      "note": "门店调试版",
      "format": "yaml",
      "source_draft_version": 12,
      "created_by": 9,
      "created_by_username": "alice",
      "created_at": "2026-04-20T10:42:00Z"
    }
  ]
}
```

说明：

- 首版不强制分页
- 默认按 `created_at desc`

### 2. 详情

`GET /api/draft-saved-versions/:id`

响应示例：

```json
{
  "saved_version": {
    "id": 301,
    "deployment_instance_id": 18,
    "config_file_id": 7,
    "title": "2026-04-20 18:42",
    "note": "门店调试版",
    "content": "shop:\n  id: store-001\n",
    "format": "yaml",
    "source_draft_version": 12,
    "created_by": 9,
    "created_by_username": "alice",
    "created_at": "2026-04-20T10:42:00Z"
  }
}
```

### 3. 修改备注

`PATCH /api/draft-saved-versions/:id`

请求体：

```json
{
  "note": "晚高峰降载参数"
}
```

说明：

- 首版只允许改 `note`
- `title` 默认不开放用户自由改名，避免 UI 复杂化

### 4. 恢复到 Current Draft

`POST /api/draft-saved-versions/:id/restore`

请求体：

```json
{
  "base_version": 12
}
```

响应示例：

```json
{
  "draft": {
    "deployment_instance_id": 18,
    "config_file_id": 7,
    "content": "shop:\n  id: store-001\n",
    "format": "yaml",
    "version": 13,
    "updated_at": "2026-04-20T10:50:00Z"
  }
}
```

说明：

- 语义是“用该 Saved Version 覆盖 Current Draft”
- 恢复成功后，返回最新 Current Draft
- 需要并发保护，因此请求体带 `base_version`

### 5. 删除

`DELETE /api/draft-saved-versions/:id`

说明：

- 删除只影响历史版本列表
- 不影响 Current Draft
- 不影响 Releases

### 6. 手动创建

首版不建议提供独立 `POST /api/draft-saved-versions`

理由：

- 首版默认由 “保存 Current Draft” 自动生成
- 可以减少一个歧义：用户是否在“保存”还是“另存为”

如后续确实需要“只打标签不改 Draft”，再补该接口。

## Q5: 恢复接口为什么不直接复用现有 Draft clone？

不建议直接复用现有单配置 clone 接口。

原因：

- 当前 clone 语义是“从其他实例 / 其他来源复制到目标 Draft”
- `Saved Versions restore` 语义是“从同实例历史版本恢复到 Current Draft”
- 前端文案、审计、错误码都应区分

可以在后端内部复用同一段写 Draft 逻辑，但外部接口应独立。

## Q6: 错误码建议是什么？

建议新增：

- `saved_version_not_found`
  - 历史版本不存在或当前用户不可见
- `saved_version_restore_conflict`
  - 恢复时 `base_version` 冲突
- `saved_version_note_too_long`
  - 备注超长
- `saved_version_restore_failed`
  - 恢复失败

同时复用已有：

- `project_permission_denied`
- `draft_version_conflict`
- `draft_validation_failed`
- `deployment_instance_not_found`
- `config_file_not_found`

其中：

- `saved_version_restore_conflict` 与 `draft_version_conflict` 二选一即可
- 若后端不想新增恢复专属错误码，可统一继续使用 `draft_version_conflict`

## Q7: 权限应该如何收口？

建议与 Current Draft 保持一致：

- `admin`
  - 可查看 / 恢复 / 删除 / 改备注
- `editor`
  - 可查看 / 恢复 / 删除 / 改备注
- `viewer`
  - 不显示 Saved Versions 面板主操作
  - 最多只读查看 Releases

首版不建议开放 `viewer` 查看 Saved Versions。

原因：

- Saved Versions 属于工作过程数据，不是稳定发布真值
- 把它暴露给纯只读角色会增加解释成本

## Q8: 前端工作台如何绑定这些接口？

建议页面行为如下：

### Current Draft 保存

- 调用 `PUT /api/drafts/:deploymentId/:configFileId`
- 成功后：
  - 更新 Current Draft
  - 刷新 Saved Versions 列表

### Saved Version 详情

- 点击列表项时调用 `GET /api/draft-saved-versions/:id`

### Saved Version 恢复

- 调用 `POST /api/draft-saved-versions/:id/restore`
- 成功后：
  - 用返回 Draft 覆盖编辑器内容
  - 重置 dirty 状态
  - 刷新页头状态

### Saved Version 删除

- 调用 `DELETE /api/draft-saved-versions/:id`
- 成功后刷新列表

### Saved Version 备注

- 调用 `PATCH /api/draft-saved-versions/:id`
- 成功后局部刷新详情与列表项摘要

## Q9: 首版是否要求 OpenAPI 一次性完整覆盖？

建议分两步。

### 第一步

先补：

- Saved Versions 列表
- 详情
- 恢复
- 删除
- 备注更新

并更新：

- `openapi.json`
- 管理端集成测试
- 前端 client 类型

### 第二步

前端历史面板已接入，后续再考虑：

- `另存为版本`
- 版本对比
- 版本筛选

## Q10: 渐进落地顺序应如何安排？

当前阶段 1 和阶段 2 已完成，后续只保留体验优化项。

### 阶段 1：后端模型与接口（已完成）

- 增加 `draft_saved_versions` 表
- 在 Current Draft 保存链路中自动写入 Saved Version
- 补列表 / 详情 / 恢复 / 删除 / 改备注接口
- 补 OpenAPI
- 补集成测试

验收：

- 保存 Current Draft 后数据库中有 Saved Version
- 恢复不会影响 Release 历史
- 删除不会影响 Current Draft

### 阶段 2：前端历史面板（已完成）

- 接入 Saved Versions 列表
- 接入详情抽屉或详情区
- 接入恢复与删除
- 接入备注编辑

验收：

- 用户能从历史保存版本恢复继续编辑
- dirty guard 与恢复流程配合正确

### 阶段 3：体验优化（待定）

- 显示“与当前 Draft 相同”的弱提示
- 版本对比
- 更好的空状态文案

## 当前结论

- `Saved Versions` 应使用独立表和独立接口
- 首版以“保存 Current Draft 自动生成历史版本”为主
- 恢复必须覆盖 Current Draft，并做并发保护
- 发布仍只针对 Current Draft
- 前后端可按“后端模型 -> 工作台右栏 -> 体验优化”三阶段推进
