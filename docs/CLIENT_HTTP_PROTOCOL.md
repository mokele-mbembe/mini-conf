# 消费端 HTTP 协议草案

## 1. 设计目标

这份文档定义 `mini-conf` 面向消费端的最小 HTTP 协议。

目标是：

- 不要求接入重量 SDK
- 只靠简单 HTTP 请求即可拉取在线配置
- 支持版本检查、配置获取、结果回传
- 适用于 IoT 设备、服务实例、任务节点、CLI 和桌面程序
- 让同一部署实例上的多个进程可以共享一份凭证访问平台

首版采用：

- HTTP + JSON
- 轮询拉取
- `ETag / If-None-Match`
- Bearer Token

首版不做：

- gRPC
- WebSocket
- SSE
- 长轮询
- 强依赖客户端动态标签匹配

## 2. 基本概念

### Project

配置所属项目，例如：

- `coffee-legacy`
- `billing-service`

### Config

一份配置文件，例如：

- `main`
- `ad-screen`
- `vision`

### Environment

运行环境，例如：

- `dev`
- `test`
- `prod`

### Deployment Instance

项目在某个环境下的一份独立部署实例。

它可以：

- 持有多份配置文件
- 从模板克隆
- 被多个进程共享同一份部署实例级凭证访问

示例：

- `store-001`
- `store-002`
- `template-default-store`

### Process

部署实例上的某个具体进程，可选上报字段，例如：

- `main`
- `ad-screen`
- `vision`

## 3. 鉴权方式

首版默认使用 Bearer Token。

```http
Authorization: Bearer <token>
```

说明：

- token 归属于部署实例，而不是单个进程
- 同一部署实例上的多个进程可以共享同一份 token
- token 默认长期有效
- 可通过管理端手动重置和吊销

## 4. 请求约定

请求统一约定：

- `Content-Type: application/json`
- `Accept: application/json`
- 所有时间字段使用 RFC 3339
- 所有错误响应都返回统一 JSON 结构

通用错误响应建议：

```json
{
  "code": "deployment_not_found",
  "message": "deployment instance not found",
  "request_id": "01HR...."
}
```

## 5. 版本检查与配置解析

### `GET /api/open/configs/resolve`

用途：

- 根据项目、环境、部署实例和配置文件，解析出当前应使用的 Release
- 如果客户端已持有最新版本，服务端可返回 `304 Not Modified`

### 请求参数

查询参数建议：

- `project`
- `environment`
- `deployment_key`
- `config`

可选查询参数：

- `process_key`
- `current_revision`

请求头可选：

- `If-None-Match: "<content_hash>"`

### 请求示例

```http
GET /api/open/configs/resolve?project=coffee-legacy&environment=prod&deployment_key=store-001&config=main&process_key=main HTTP/1.1
Host: conf.example.com
Accept: application/json
Authorization: Bearer <token>
If-None-Match: "7d6b0c..."
```

### 成功响应

```json
{
  "project": "coffee-legacy",
  "environment": "prod",
  "deployment": {
    "key": "store-001",
    "name": "Store 001"
  },
  "config": "main",
  "release": {
    "revision": "20260405.0001",
    "content_hash": "7d6b0c5c9d...",
    "format": "yaml",
    "published_at": "2026-04-05T12:00:00Z",
    "apply_mode": "soft"
  },
  "fetch": {
    "url": "/api/open/releases/20260405.0001"
  }
}
```

### 未变化响应

当 `If-None-Match` 或 `current_revision` 表示客户端已是最新版本时：

- 返回 `304 Not Modified`
- 不重复传输配置正文

### 未命中响应

当部署实例或配置未命中时：

- 明确返回失败
- 不做隐式兜底
- 由客户端决定本地缓存或默认配置策略

建议错误码：

- `deployment_not_found`
- `config_file_not_found`
- `release_not_found`

## 6. 拉取发布内容

### `GET /api/open/releases/:revision`

用途：

- 拉取某个已发布版本的单配置文件内容

### 请求示例

```http
GET /api/open/releases/20260405.0001 HTTP/1.1
Host: conf.example.com
Accept: application/json
Authorization: Bearer <token>
```

### 成功响应

```json
{
  "release": {
    "revision": "20260405.0001",
    "content_hash": "7d6b0c5c9d...",
    "format": "yaml",
    "published_at": "2026-04-05T12:00:00Z",
    "apply_mode": "soft"
  },
  "deployment": {
    "project": "coffee-legacy",
    "environment": "prod",
    "deployment_key": "store-001"
  },
  "config": {
    "name": "main"
  },
  "content": "log_level: info\npoll_interval_sec: 30\n",
  "metadata": {
    "schema_version": "v1",
    "change_summary": "adjust polling interval"
  }
}
```

响应头建议：

- `ETag: "<content_hash>"`
- `Cache-Control: no-cache`

## 7. 拉取整部署实例配置包

### `GET /api/open/deployments/:deploymentKey/config-bundle`

用途：

- 一次性拉取某个部署实例下多份配置文件的当前已发布内容

说明：

- 这是 MVP 可顺手加入的能力
- 适合一台机器上多个进程一起启动或统一预热配置

### 请求示例

```http
GET /api/open/deployments/store-001/config-bundle?project=coffee-legacy&environment=prod HTTP/1.1
Host: conf.example.com
Accept: application/json
Authorization: Bearer <token>
```

### 成功响应

```json
{
  "project": "coffee-legacy",
  "environment": "prod",
  "deployment": {
    "key": "store-001",
    "name": "Store 001"
  },
  "configs": [
    {
      "config": "main",
      "revision": "20260405.0001",
      "content_hash": "aaa",
      "format": "yaml",
      "content": "log_level: info\n"
    },
    {
      "config": "ad-screen",
      "revision": "20260405.0003",
      "content_hash": "bbb",
      "format": "yaml",
      "content": "screen_timeout: 15\n"
    },
    {
      "config": "vision",
      "revision": "20260405.0002",
      "content_hash": "ccc",
      "format": "yaml",
      "content": "camera_enabled: true\n"
    }
  ]
}
```

## 8. 上报同步结果

### `POST /api/open/deployment-sync-records`

用途：

- 上报部署实例中某个进程的版本检查、拉取、应用结果
- 为管理端审计和问题排查提供依据

### 请求体建议

```json
{
  "project": "coffee-legacy",
  "environment": "prod",
  "deployment_key": "store-001",
  "config": "main",
  "process_key": "main",
  "action": "apply",
  "revision": "20260405.0001",
  "status": "success",
  "message": "config applied",
  "detail": {
    "duration_ms": 87
  },
  "reported_at": "2026-04-05T12:05:00Z"
}
```

### 响应建议

```json
{
  "ok": true
}
```

### action 枚举建议

- `version_check`
- `fetch`
- `apply`
- `heartbeat`

### status 枚举建议

- `success`
- `noop`
- `failed`

## 9. 心跳上报

### `POST /api/open/heartbeats`

用途：

- 上报部署实例在线状态
- 更新最近活跃时间

### 请求体示例

```json
{
  "project": "coffee-legacy",
  "environment": "prod",
  "deployment_key": "store-001",
  "process_key": "vision",
  "metadata": {
    "ip": "10.0.0.8",
    "version": "1.0.3"
  },
  "reported_at": "2026-04-05T12:05:00Z"
}
```

## 10. 推荐轮询流程

消费端推荐按以下顺序工作：

1. 调用 `/api/open/configs/resolve` 检查某份配置当前应使用的版本
2. 若命中 `304`，保持当前配置
3. 若返回新 `revision`，再调用 `/api/open/releases/:revision`
4. 应用配置
5. 调用 `/api/open/deployment-sync-records` 回传结果
6. 周期性调用 `/api/open/heartbeats`

如果业务想一次预热整套配置，也可以：

1. 调用 `/api/open/deployments/:deploymentKey/config-bundle`
2. 将多份配置分发给本地各进程

## 11. curl 最小接入示例

### 1. 查询当前版本

```bash
curl -sS \
  -H "Authorization: Bearer ${MINI_CONF_TOKEN}" \
  -H "Accept: application/json" \
  "http://127.0.0.1:8080/api/open/configs/resolve?project=coffee-legacy&environment=prod&deployment_key=store-001&config=main&process_key=main"
```

### 2. 拉取单配置文件正文

```bash
curl -sS \
  -H "Authorization: Bearer ${MINI_CONF_TOKEN}" \
  -H "Accept: application/json" \
  "http://127.0.0.1:8080/api/open/releases/20260405.0001"
```

### 3. 拉取整部署实例配置包

```bash
curl -sS \
  -H "Authorization: Bearer ${MINI_CONF_TOKEN}" \
  -H "Accept: application/json" \
  "http://127.0.0.1:8080/api/open/deployments/store-001/config-bundle?project=coffee-legacy&environment=prod"
```

### 4. 上报应用结果

```bash
curl -sS \
  -X POST \
  -H "Authorization: Bearer ${MINI_CONF_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "project": "coffee-legacy",
    "environment": "prod",
    "deployment_key": "store-001",
    "config": "main",
    "process_key": "main",
    "action": "apply",
    "revision": "20260405.0001",
    "status": "success",
    "message": "config applied"
  }' \
  "http://127.0.0.1:8080/api/open/deployment-sync-records"
```

## 12. 服务端实现建议

为了让协议稳定且适合开源演进，建议：

- 管理端 API 和开放消费端 API 分离
- 消费端接口保持字段少、语义稳、兼容演进
- 所有开放接口纳入 OpenAPI 文档
- 为开放接口补契约测试
- 为 `curl` 示例建立集成测试，避免文档和实现漂移

## 13. 后续演进方向

不进入 MVP，但可以预留：

- JWT 或签名形式的消费端扩展认证
- 部署实例配置包增量拉取
- 长轮询
- SSE
- Webhook
- 命名空间隔离
- 灰度发布策略
- 轻量官方客户端
