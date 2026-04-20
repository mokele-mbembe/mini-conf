# Saved Versions 后端实施清单

## 1. 目标

这份清单用于把 `Saved Versions` 从设计稿推进到后端可交付实现。

范围只覆盖首版：

- 唯一 `Current Draft`
- 自动生成 `Saved Versions`
- `Saved Versions` 列表 / 详情 / 恢复 / 删除 / 备注更新
- OpenAPI、集成测试、前端 client 可消费

不包含：

- 多候选 Draft 分支
- 版本对比
- 审批流
- 另存为版本的独立入口

## 2. 现有落点

当前相关后端文件：

- 路由聚合：[api.rs](/home/zjj/Projects/mini-conf/apps/server/src/http/api.rs)
- Draft API：[drafts.rs](/home/zjj/Projects/mini-conf/apps/server/src/http/api/drafts.rs)
- Release API：[releases.rs](/home/zjj/Projects/mini-conf/apps/server/src/http/api/releases.rs)
- OpenAPI 汇总：[openapi.rs](/home/zjj/Projects/mini-conf/apps/server/src/openapi.rs)
- Draft schema：[draft.rs](/home/zjj/Projects/mini-conf/crates/schema/src/draft.rs)
- Release schema：[release.rs](/home/zjj/Projects/mini-conf/crates/schema/src/release.rs)
- Draft 集成测试：[drafts.rs](/home/zjj/Projects/mini-conf/apps/server/tests/drafts.rs)
- Release 集成测试：[releases.rs](/home/zjj/Projects/mini-conf/apps/server/tests/releases.rs)
- 数据库迁移目录：[migrations](/home/zjj/Projects/mini-conf/migrations)

## 3. 执行顺序

建议固定按这个顺序推进：

1. 数据库迁移
2. schema crate
3. API handler 与路由
4. OpenAPI
5. 集成测试
6. OpenAPI 导出与本地 CI

不要先改前端，再倒推后端对象。

## 4. 数据库迁移

### 4.1 新增 migration

新增：

- `0014_saved_versions.up.sql`
- `0014_saved_versions.down.sql`

建议表名：

- `draft_saved_versions`

建议字段：

```text
id bigserial primary key
deployment_instance_id bigint not null references deployment_instances(id) on delete cascade
config_file_id bigint not null references config_files(id) on delete cascade
title text not null
note text null
content text not null
content_hash text not null
format text not null
source_draft_version bigint not null
created_by bigint not null references users(id)
created_at timestamptz not null default now()
updated_at timestamptz not null default now()
deleted_at timestamptz null
```

建议索引：

- `(deployment_instance_id, config_file_id, created_at desc)`
- 如果使用软删除，再考虑部分索引：
  - `where deleted_at is null`

### 4.2 迁移验收

- migration 可在空库上完整执行
- rollback 可完整回滚
- 不影响现有 `drafts` / `releases`

## 5. Schema crate

### 5.1 新增 schema 文件

建议新增：

- `crates/schema/src/saved_version.rs`

建议暴露类型：

- `SavedVersionSummary`
- `SavedVersionListResponse`
- `SavedVersionDetail`
- `SavedVersionDetailResponse`
- `SavedVersionRestoreResponse`

建议字段：

### `SavedVersionSummary`

- `id`
- `deployment_instance_id`
- `config_file_id`
- `title`
- `note`
- `format`
- `source_draft_version`
- `created_by`
- `created_by_username`
- `created_at`

### `SavedVersionDetail`

在 summary 基础上增加：

- `content`

### `SavedVersionRestoreResponse`

- `draft`
  - 直接复用现有 `DraftResponse`

### 5.2 导出

更新：

- `crates/schema/src/lib.rs`

确保新的 schema 可被 server 和 OpenAPI 使用。

### 5.3 Schema 单测

给新 response 类型补序列化形状测试，风格对齐 [draft.rs](/home/zjj/Projects/mini-conf/crates/schema/src/draft.rs) 和 [release.rs](/home/zjj/Projects/mini-conf/crates/schema/src/release.rs)。

## 6. API handler 与路由

### 6.1 新增 handler 文件

建议新增：

- `apps/server/src/http/api/saved_versions.rs`

建议路由：

- `GET /api/draft-saved-versions`
- `GET /api/draft-saved-versions/:id`
- `PATCH /api/draft-saved-versions/:id`
- `POST /api/draft-saved-versions/:id/restore`
- `DELETE /api/draft-saved-versions/:id`

### 6.2 路由注册

更新：

- [api.rs](/home/zjj/Projects/mini-conf/apps/server/src/http/api.rs)

需要：

- `pub(crate) mod saved_versions;`
- `.merge(saved_versions::router())`

### 6.3 请求体定义

建议在 `saved_versions.rs` 内定义并校验：

- `UpdateSavedVersionRequest`
  - `note: Option<String>`
- `RestoreSavedVersionRequest`
  - `base_version: Option<i64>`

建议加长度约束：

- `note` 去首尾空白后可为空
- 长度上限固定，例如 `200` 或 `500`

### 6.4 复用逻辑

后端内部可以抽共享函数，但外部接口语义不要复用 `clone_draft`。

建议抽成内部 helper：

- 根据 `deployment_instance_id + config_file_id` 加载 Current Draft
- 用一段统一逻辑写入 / 覆盖 Current Draft
- 自动计算 `content_hash`
- 统一写审计日志

### 6.5 保存链路自动生成 Saved Version

更新：

- [drafts.rs](/home/zjj/Projects/mini-conf/apps/server/src/http/api/drafts.rs)

要求：

- `PUT /api/drafts/:deploymentId/:configFileId` 保存成功后，自动插入一条 `draft_saved_versions`
- 若与最近一条历史版本 `content_hash` 相同，则跳过自动生成
- `title` 自动生成为 UTC 或统一时区格式时间串
- `source_draft_version` 记录保存后的 Current Draft 版本号

### 6.6 恢复接口行为

`POST /api/draft-saved-versions/:id/restore` 必须：

- 校验当前用户项目权限
- 校验历史版本所属实例/配置是否可见
- 校验 `base_version`
- 覆盖 Current Draft
- 递增 Current Draft 版本号
- 返回最新 `DraftResponse`
- 写审计日志

## 7. 审计日志

建议新增 action：

- `saved_version.created`
- `saved_version.updated`
- `saved_version.restored`
- `saved_version.deleted`

建议 `resource_type`：

- `saved_version`

建议 detail 至少记录：

- `saved_version_id`
- `deployment_instance_id`
- `config_file_id`
- `source_draft_version`

## 8. 错误码

建议新增：

- `saved_version_not_found`
- `saved_version_note_too_long`

恢复冲突建议直接复用：

- `draft_version_conflict`

这样前端已有 Current Draft 冲突处理可以直接复用。

如果实现中需要更细粒度区分，再考虑新增：

- `saved_version_restore_conflict`

## 9. OpenAPI

### 9.1 更新 OpenAPI paths

更新：

- [openapi.rs](/home/zjj/Projects/mini-conf/apps/server/src/openapi.rs)

新增 path 引用：

- `crate::http::api::saved_versions::list_saved_versions`
- `crate::http::api::saved_versions::get_saved_version`
- `crate::http::api::saved_versions::update_saved_version`
- `crate::http::api::saved_versions::restore_saved_version`
- `crate::http::api::saved_versions::delete_saved_version`

### 9.2 更新 OpenAPI components

新增 schema：

- `SavedVersionSummary`
- `SavedVersionListResponse`
- `SavedVersionDetail`
- `SavedVersionDetailResponse`
- `SavedVersionRestoreResponse`
- `UpdateSavedVersionRequestBody`
- `RestoreSavedVersionRequestBody`
- `ListSavedVersionsParams`

### 9.3 导出产物

执行：

- `cargo run -p server --bin openapi-export`

确认更新：

- `docs/artifacts/openapi.json`

## 10. 集成测试

### 10.1 新增测试文件

建议新增：

- `apps/server/tests/saved_versions.rs`

不要把所有 Saved Versions 测试都塞进现有 [drafts.rs](/home/zjj/Projects/mini-conf/apps/server/tests/drafts.rs)，否则文件会继续膨胀。

### 10.2 测试基线

按现有测试约定实现：

- 使用隔离 schema
- `type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>`
- setup / teardown 返回 `Result`
- 避免 `unwrap()` / `expect()` 出现在共享 helper

### 10.3 必测用例

最少覆盖：

1. `GET /api/draft-saved-versions` 返回空列表
2. 保存 Current Draft 后自动生成 Saved Version
3. 相同内容重复保存不重复生成 Saved Version
4. `GET /api/draft-saved-versions/:id` 返回详情
5. `PATCH /api/draft-saved-versions/:id` 可修改备注
6. `PATCH` 备注过长返回业务错误
7. `POST /api/draft-saved-versions/:id/restore` 成功覆盖 Current Draft
8. `restore` 在 `base_version` 冲突时返回 `409`
9. `DELETE /api/draft-saved-versions/:id` 删除后列表不再返回该版本
10. 删除 Saved Version 不影响 Current Draft
11. 恢复 Saved Version 不影响 Release 历史
12. 非成员访问返回资源级 `404`
13. `viewer` 权限不能恢复或删除
14. 审计日志被正确写入

### 10.4 Draft 相关回归测试

同步补回归到 [drafts.rs](/home/zjj/Projects/mini-conf/apps/server/tests/drafts.rs)：

- 保存 Draft 后会生成 Saved Version
- 删除 Current Draft 不会删除 Saved Versions

## 11. SQL 与实现细节检查点

### 11.1 列表查询

列表接口需要：

- 只返回未删除记录
- `order by created_at desc, id desc`
- 通过 `deployment_id + config_file_id` 过滤

### 11.2 详情查询

详情接口需要：

- 严格校验所属项目权限
- 返回 `created_by_username`

### 11.3 自动标题

不要把标题格式散落在前端和后端两边各自生成。

建议后端统一生成，前端直接显示。

### 11.4 软删除或硬删除

首版建议：

- 对外表现为删除
- 底层可用软删除

理由：

- 后续如果需要审计与恢复空间，更稳
- 对前端语义没有额外复杂度

## 12. 本地执行清单

建议执行顺序：

1. 写 migration
2. 写 schema crate
3. 写 `saved_versions.rs` handler
4. 改 `drafts.rs` 自动生成逻辑
5. 改 `api.rs`
6. 改 `openapi.rs`
7. 写 `apps/server/tests/saved_versions.rs`
8. 跑 targeted tests
9. 导出 OpenAPI
10. 跑全量本地 CI

## 13. 建议命令

开发期最少跑：

```bash
cargo test -p server --test saved_versions
cargo test -p server --test drafts
cargo test -p server --test releases
cargo run -p server --bin openapi-export
```

收口时再跑：

```bash
just ci-local
just ci-local-db
```

## 14. 完成标准

后端侧完成应满足：

- Current Draft 保存会自动产生 Saved Version
- Saved Version 可列出、查看、改备注、删除、恢复
- 恢复会覆盖 Current Draft 并遵守并发保护
- Release 历史不受影响
- OpenAPI 与导出产物同步更新
- 集成测试覆盖主流程与权限边界
- 本地 CI 通过
