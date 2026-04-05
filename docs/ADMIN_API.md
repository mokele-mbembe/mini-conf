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
- 所有列表接口使用统一分页结构
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
  "items": [],
  "page": 1,
  "page_size": 20,
  "total": 0
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

## 5. 分页与筛选约定

列表接口统一使用：

- `page`
- `page_size`

默认值建议：

- `page=1`
- `page_size=20`

上限建议：

- `page_size <= 100`

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

### `POST /api/projects`

### `GET /api/projects/:id`

### `PUT /api/projects/:id`

## 8. Project Member API

### `GET /api/projects/:id/members`

### `POST /api/projects/:id/members`

### `PUT /api/projects/:id/members/:memberId`

### `DELETE /api/projects/:id/members/:memberId`

说明：

- 首版角色只保留 `admin`、`editor`、`viewer`
- 权限以项目为边界，不做复杂细粒度 RBAC

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
  "schema_name": "coffee-main",
  "schema_version": "v1",
  "sensitivity": "secret",
  "secret_paths": [
    "$.wifi.password",
    "$.third_party.api_key"
  ]
}
```

### `GET /api/config-files/:id`

### `PUT /api/config-files/:id`

说明：

- `sensitivity` 首版可支持 `normal` 和 `secret`
- `secret_paths` 用于前端脱敏展示和日志裁剪

## 10. Deployment Instance API

### `GET /api/deployment-instances`

查询参数：

- `project_id`
- `environment`
- `page`
- `page_size`
- `keyword`
- `status`

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

- `clone_source` 首版可支持 `draft` 或 `latest_release`
- 克隆完成后与模板不联动

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
- 保存时立即做格式和 schema 校验
- 校验失败返回 `422`
- `base_version` 与服务端当前版本不一致时返回 `409`

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

### `GET /api/releases`

### `GET /api/releases/:id`

### `GET /api/releases/:id/diff`

## 13. Deployment Credential API

### `POST /api/deployment-instances/:id/token/reset`

用途：

- 重置部署实例级访问凭证

成功响应建议：

```json
{
  "deployment_instance_id": 8,
  "token_preview": "mc_live_***",
  "token": "mc_live_xxxxxxxxx"
}
```

## 14. Deployment Sync Record API

### `GET /api/deployment-sync-records`

查询参数：

- `project_id`
- `deployment_instance_id`
- `config_file_id`
- `process_key`
- `action`
- `status`
- `page`
- `page_size`

## 15. 状态码建议

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

## 16. 首版错误码建议

- `auth_invalid_credentials`
- `auth_session_expired`
- `project_code_conflict`
- `project_member_conflict`
- `config_file_code_conflict`
- `deployment_instance_conflict`
- `draft_validation_failed`
- `draft_version_conflict`
- `draft_not_found`
- `release_publish_failed`
- `release_not_found`
- `deployment_not_found`
- `deployment_token_reset_failed`

## 17. 实现建议

- 后端直接基于这些结构生成 OpenAPI
- 在 CI 中检查导出的 OpenAPI 产物是否与仓库内版本一致
- 前端尽量不要自行拼装接口语义
- 为登录、部署实例克隆、Draft 保存、Release 发布、Diff 查询补集成测试
- 为列表接口固定分页结构，避免后续前端大改
