# 管理端 API 草案

## 1. 文档目标

这份文档定义 `mini-conf` 管理端 API 的首版契约边界。

目标是：

- 支持管理后台页面开发
- 支持项目、成员、配置文件、部署实例、Draft、Release 的核心操作
- 提前固定请求体、响应体、错误码和分页语义
- 为后续 OpenAPI 生成和集成测试提供稳定基础

这份文档只覆盖 MVP。

## 2. 设计原则

- 管理端 API 与开放消费端 API 分离
- 管理端默认面向浏览器和受信任内部调用
- 所有写操作默认要求登录
- 所有权限判断以项目级成员关系为主
- 当前所有列表接口统一返回 `{ "items": [...] }`
- 所有错误响应使用统一错误格式

## 3. 基础约定

基础路径：

- `/api`

请求头：

- `Content-Type: application/json`
- `Accept: application/json`

时间格式：

- 统一使用 RFC 3339

## 4. 统一响应格式

成功响应建议保持“资源直出”为主，不额外包一层 `data`，减少前后端样板代码。

列表响应统一格式：

```json
{
  "items": []
}
```

错误响应统一格式：

```json
{
  "code": "deployment_not_found",
  "message": "deployment instance not found",
  "request_id": "01HR..."
}
```

## 5. 列表与筛选约定

当前首版列表接口不做分页统一约定。

- 统一返回 `{ "items": [...] }`
- 按需支持查询参数过滤
- 如果后续需要分页，再单独做协议升级

## 6. 管理端认证接口

首版支持两种模式的设计入口：

- Session Cookie
- JWT

但 MVP 只完整实现：

- Session Cookie

### `POST /api/auth/login`

成功响应：

```json
{
  "user": {
    "id": 1,
    "username": "admin"
  },
  "auth_mode": "session"
}
```

说明：

- MVP 通过 HttpOnly Session Cookie 维持登录态
- JWT 方案只预留扩展点，不在首版完整交付

### `POST /api/auth/logout`

### `GET /api/auth/me`

## 7. Project API

### `GET /api/projects`

说明：

- 仅返回当前登录用户参与的项目
- 非项目成员不会在列表中看到该项目

### `POST /api/projects`

说明：

- 任意已登录用户都可以创建项目
- 创建成功后，创建者自动成为该项目 `admin`

### `GET /api/projects/:id`

### `PUT /api/projects/:id`

说明：

- `GET` 需要当前用户是该项目成员
- `PUT` 仅项目 `admin` 可调用

## 8. Project Member API

### `GET /api/projects/:id/members`

成功响应示例：

```json
{
  "items": [
    {
      "id": 12,
      "project_id": 7,
      "user_id": 9,
      "username": "alice",
      "role": "editor",
      "created_at": "2026-04-10T12:00:00Z"
    }
  ]
}
```

### `POST /api/projects/:id/members`

请求体：

```json
{
  "username": "alice",
  "role": "viewer"
}
```

### `PUT /api/projects/:id/members/:memberId`

请求体：

```json
{
  "role": "admin"
}
```

### `DELETE /api/projects/:id/members/:memberId`

说明：

- 首版角色只保留 `admin`、`editor`、`viewer`
- 权限以项目为边界，不做复杂细粒度 RBAC
- 目标用户必须已存在且 `status = active`
- 重复成员返回 `409 project_member_conflict`
- 不允许删除或降级最后一个项目 `admin`

## 9. Config File API

### `GET /api/config-files`

### `POST /api/config-files`

请求体建议：

```json
{
  "project_id": 1,
  "code": "main",
  "name": "Main Config",
  "format": "yaml",
  "sensitivity": "secret",
  "secret_paths": ["$.wifi.password", "$.third_party.api_key"],
  "is_required": true
}
```

### `GET /api/config-files/:id`

### `PUT /api/config-files/:id`

说明：

- `code` 在中文语义上更接近“配置标识”，不是字符编码
- 当前配置文件主路径支持 `yaml / json / toml`，`text` 不在当前范围内
- `sensitivity` 首版可支持 `normal` 和 `secret`
- `secret_paths` 用于前端脱敏展示和日志裁剪
- `is_required` 是项目级规则，用于约束实例发布前是否必须已具备该配置
- `GET` 需要项目成员身份
- `POST / PUT` 仅项目 `admin` 可调用

## 10. Deployment Instance API

### `GET /api/deployment-instances`

查询参数：

- `project_id`
- `environment`
- `page`
- `page_size`
- `keyword`
- `status`

响应：

```json
{
  "items": [],
  "total": 0,
  "page": 1,
  "page_size": 20
}
```

说明：

- `page` 默认 `1`
- `page_size` 默认 `20`，最大 `100`
- `status` 仅支持 `active`、`inactive`

### `POST /api/deployment-instances`

请求体：

```json
{
  "project_id": 1,
  "environment": "prod",
  "deployment_key": "store-001",
  "name": "Store 001",
  "description": "hangzhou store 001",
  "is_template": false
}
```

### `GET /api/deployment-instances/:id`

### `PUT /api/deployment-instances/:id`

请求体：

```json
{
  "environment": "prod",
  "deployment_key": "store-001",
  "name": "Store 001",
  "description": "hangzhou store 001"
}
```

说明：

- 只允许修改 `environment`、`deployment_key`、`name`、`description`
- `project_id`、`is_template`、`status` 创建后不可通过 `PUT` 修改
- 部署实例创建后默认 `inactive`

### `POST /api/deployment-instances/:id/clone`

用途：

- 从模板部署实例克隆出一个新部署实例

请求体：

```json
{
  "deployment_key": "store-002",
  "name": "Store 002",
  "environment": "prod",
  "clone_source": "draft"
}
```

说明：

- `clone_source` 首版只支持 `draft`
- 克隆完成后与模板不联动
- Template 本身不可发布，只用于创建实例
- 克隆出的普通实例默认 `inactive`
- `GET` 需要项目成员身份
- `POST / PUT / clone` 仅项目 `admin` 可调用

## 11. Draft API

### `GET /api/drafts/:deploymentId/:configFileId`

成功响应建议包含：

```json
{
  "deployment_instance_id": 8,
  "config_file_id": 3,
  "format": "yaml",
  "content": "poll_interval_ms: 5000",
  "version": 4,
  "updated_at": "2026-04-05T12:00:00Z"
}
```

### `PUT /api/drafts/:deploymentId/:configFileId`

请求体建议：

```json
{
  "content": "poll_interval_ms: 8000",
  "format": "yaml",
  "base_version": 4
}
```

行为约定：

- 如果 Draft 不存在，则创建
- 如果 Draft 已存在，则按乐观锁更新
- 保存时立即做基础格式合法性校验
- 校验失败返回 `422`
- `base_version` 与服务端当前版本不一致时返回 `409`

### `POST /api/drafts/:targetDeploymentId/:configFileId/clone`

用途：

- 将同项目内其他实例的单个配置文件复制到目标实例 Draft

请求体建议：

```json
{
  "source_deployment_instance_id": 9,
  "source_kind": "latest_release"
}
```

行为约定：

- `source_kind` 支持 `draft` 或 `latest_release`
- 来源和目标必须在同一项目内
- 如果目标 Draft 已存在，则覆盖内容并递增 `version`
- 前端通过多次调用这个接口完成批量 clone
- `GET / PUT / clone` 仅项目 `admin` 和 `editor` 可调用

## 12. Release API

### `POST /api/releases/publish`

请求体：

```json
{
  "project_id": 1,
  "deployment_instance_id": 8,
  "config_file_id": 3,
  "change_summary": "increase polling interval"
}
```

行为约定：

- 只允许基于当前 Draft 发布
- 发布成功后 Release 不可变
- 即使当前 Draft 内容与上一版相同，重复发布仍生成新 revision
- `is_template = true` 的实例禁止发布
- 如果目标实例缺少任一必选配置，则本次发布被阻止
- `POST /api/releases/publish` 仅项目 `admin` 和 `editor` 可调用

### `GET /api/deployment-instances/:id/preview-bundle`

用途：

- 预览某个实例当前“最终会被消费端看到的整包配置效果”

响应语义：

- `items` 中逐项展示每份配置来自 Draft 还是最新 Release
- 必选配置缺失时要明确标记
- 同时返回一份可直接复制的 `open_bundle_preview`
- 仅项目 `admin` 和 `editor` 可调用

### `GET /api/releases`

### `GET /api/releases/:id`

### `GET /api/releases/:id/diff`

用途：

- 查看某次发布相对上一版 release 的文本差异

响应语义：

- `base_release` 固定表示同一 `deployment_instance + config_file` 下的上一条 release
- 如果当前 release 是首发，则 `base_release = null`
- `before_content` / `after_content` 供前端 DiffEditor 直接使用
- `diff_summary` 只返回轻量摘要，不返回 unified diff / patch 文本
- `GET /api/releases*` 需要项目成员身份

## 13. Deployment Credential API

### `POST /api/deployment-instances/:id/activate`

用途：

- 激活普通部署实例，并生成或覆盖默认 token

成功响应与 token reset 相同，`token` 明文只返回一次。

行为约定：

- 仅项目 `admin` 可调用
- 仅普通实例可激活，模板返回 `409 deployment_instance_template_activate_forbidden`
- 仅允许 `inactive -> active`
- 激活时生成或覆盖默认凭证

### `POST /api/deployment-instances/:id/deactivate`

用途：

- 停用普通部署实例，使 Open API 立即不可消费

行为约定：

- 仅项目 `admin` 可调用
- 仅允许 `active -> inactive`
- 同时将默认凭证置为非 active，使旧 token 立即失效

### `POST /api/deployment-instances/:id/token/reset`

用途：

- 重置部署实例级访问凭证

成功响应建议：

```json
{
  "deployment_instance_id": 8,
  "credential_name": "default",
  "token_preview": "mc_live_***",
  "token": "mc_live_xxxxxxxxx"
}
```

行为约定：

- 默认只处理 `credential_name = "default"`
- 如果实例还没有默认凭证，则本次 reset 会创建默认凭证
- 如果实例已经有默认凭证，则原地覆盖 `token_hash`
- reset 成功后旧 token 立即失效，新 token 立即生效
- `token` 明文只在响应里返回一次
- 仅允许 `active` 普通实例调用
- 仅项目 `admin` 可调用

## 14. Deployment Sync Record API

### `GET /api/deployment-sync-records`

查询参数：

- `project_id`
- `deployment_instance_id`
- `config_file_id`
- `action`
- `status`

说明：

- 仅返回当前登录用户可见项目内的同步记录
- 项目 `admin / editor / viewer` 都可查看

成功响应示例：

```json
{
  "items": [
    {
      "id": 88,
      "project_id": 7,
      "deployment_instance_id": 3,
      "config_file_id": 5,
      "config": "main",
      "release_id": 8,
      "revision": "20260410.0001",
      "action": "apply",
      "status": "success",
      "message": "config applied",
      "detail": {
        "duration_ms": 87
      },
      "reported_at": "2026-04-10T12:00:00Z"
    }
  ]
}
```

## 15. Audit Log API

### `GET /api/deployment-heartbeats`

查询参数：

- `project_id`
- `deployment_instance_id`
- `config_file_id`

说明：

- 仅返回当前登录用户可见项目内的最近心跳
- 同一个 `deployment_instance_id + config_file_id` 只保留最近一次
- 项目 `admin / editor / viewer` 都可查看

成功响应示例：

```json
{
  "items": [
    {
      "id": 91,
      "project_id": 7,
      "deployment_instance_id": 3,
      "config_file_id": 5,
      "config": "main",
      "metadata": {
        "version": "1.0.3"
      },
      "reported_at": "2026-04-10T12:01:00Z",
      "updated_at": "2026-04-10T12:01:00Z"
    }
  ]
}
```

## 16. Audit Log API

### `GET /api/audit-logs`

查询参数：

- `project_id`
- `user_id`
- `action`
- `resource_type`

说明：

- 仅项目 `admin` 可查看
- 当传入 `project_id` 时，返回该项目日志
- 当不传 `project_id` 时，返回当前用户具备 `admin` 权限的项目日志，以及该用户自己的全局认证日志

成功响应示例：

```json
{
  "items": [
    {
      "id": 41,
      "project_id": 7,
      "user_id": 1,
      "action": "project_member.created",
      "resource_type": "project_member",
      "resource_id": "17",
      "detail": {
        "username": "alice",
        "role": "viewer"
      },
      "created_at": "2026-04-10T12:00:00Z"
    }
  ]
}
```

## 17. 状态码建议

- `200 OK`
- `201 Created`
- `204 No Content`
- `400 Bad Request`
- `401 Unauthorized`
- `403 Forbidden`
- `404 Not Found`
- `409 Conflict`
- `422 Unprocessable Entity`
- `429 Too Many Requests`

## 17. 首版错误码建议

- `auth_invalid_credentials`
- `auth_session_expired`
- `project_code_conflict`
- `project_member_conflict`
- `project_permission_denied`
- `project_member_not_found`
- `user_not_found`
- `last_project_admin_required`
- `config_file_code_conflict`
- `deployment_instance_conflict`
- `draft_validation_failed`
- `draft_version_conflict`
- `draft_not_found`
- `required_config_missing`
- `deployment_instance_template_publish_forbidden`
- `release_publish_failed`
- `release_not_found`
- `deployment_not_found`
- `deployment_token_reset_failed`

## 18. 实现建议

- 后端直接基于这些结构生成 OpenAPI
- 在 CI 中检查导出的 OpenAPI 产物是否与仓库内版本一致
- 前端尽量不要自行拼装接口语义
- 为登录、部署实例克隆、Draft 保存、Release 发布、Diff 查询补集成测试
- 前端应以当前 `{ "items": [...] }` 返回形状为准
