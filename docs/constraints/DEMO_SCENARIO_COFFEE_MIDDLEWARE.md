# Coffee Middleware Demo Scenario

## 1. 文档目标

本演示案例用于在 MVP 完工前验证 `mini-conf` 是否贴合真实业务接入场景。

目标不是做一个玩具 demo，而是模拟未来咖啡中间件迁移到在线配置后的真实链路：

- 业务后台仍以设备 SN 识别店铺或工控机
- 客户端只保留旧配置中的业务后台地址和 SN
- 业务后台返回配置中心接入参数
- 客户端向配置中心拉取配置、应用配置、上报同步记录和心跳
- 配置中心管理端可编辑、发布、查看 diff、查看拉取记录和心跳

该 demo 后续应成为真实业务接入开发的参考样板。

## 2. 演示拓扑

演示环境包含：

- 1 个 `mini-conf` 配置中心
- 2 个模拟业务后台，代表两个不同业务平台或后台环境入口
- 每个模拟业务后台包含 3 个环境：`dev`、`staging`、`prod`
- 每个环境包含 2 个部署实例
- 可启动多份模拟客户端，模拟多台工控机或多进程接入

推荐演示数据：

| backend   | env     | sn    | deployment_key   |
| --------- | ------- | ----- | ---------------- |
| backend-a | dev     | SN001 | a-dev-store-001  |
| backend-a | dev     | SN002 | a-dev-store-002  |
| backend-a | staging | SN001 | a-stg-store-001  |
| backend-a | staging | SN002 | a-stg-store-002  |
| backend-a | prod    | SN001 | a-prod-store-001 |
| backend-a | prod    | SN002 | a-prod-store-002 |
| backend-b | dev     | SN001 | b-dev-store-001  |
| backend-b | dev     | SN002 | b-dev-store-002  |
| backend-b | staging | SN001 | b-stg-store-001  |
| backend-b | staging | SN002 | b-stg-store-002  |
| backend-b | prod    | SN001 | b-prod-store-001 |
| backend-b | prod    | SN002 | b-prod-store-002 |

说明：

- `SN001` 可以在不同 backend 或不同业务平台中重复。
- 配置中心不直接以 SN 作为核心层级。
- SN 到部署实例的映射由业务后台维护。
- 配置中心的部署实例抽象对应“一个店铺 / 一台边缘工控机的一套配置集合”。

## 3. 配置中心业务模型

MVP 演示采用以下映射：

```text
Project: coffee-middleware
Environment: dev / staging / prod
DeploymentInstance: 店铺或工控机实例
ConfigFile: coffee-main
```

第一阶段只迁移咖啡中间件配置，因此只要求一个配置文件：

```text
coffee-main
```

后续可扩展：

```text
ad-screen
vision-service
```

但首个演示不要求广告屏和视觉服务完成在线配置迁移。

## 4. 客户端旧配置保留内容

模拟客户端本地旧配置只保留最小启动信息：

```toml
backend_url = "http://127.0.0.1:19001"
sn = "SN001"
```

配置中心不关心客户端 fallback 策略。客户端是否在配置中心不可用时回退旧配置，由业务客户端自行实现。

## 5. 模拟业务后台职责

模拟业务后台负责按 SN 返回配置中心接入信息。

推荐接口：

```http
GET /api/bootstrap/config-center?sn=SN001
```

成功响应：

```json
{
  "config_center_base_url": "http://127.0.0.1:8080",
  "project": "coffee-middleware",
  "environment": "prod",
  "deployment_key": "a-prod-store-001",
  "token": "mc_live_xxx",
  "configs": ["coffee-main"]
}
```

业务规则：

- 同一个 backend 内，`env + sn` 唯一。
- 不同 backend 中 SN 可以重复。
- 业务后台负责把 SN 映射到配置中心部署实例。
- 业务后台不存储真实配置内容，只存储接入参数和绑定关系。
- `configs` 中的值直接对应配置中心的 `ConfigFile.code`。
- MVP 不引入独立的 `process_key`，避免同一个业务组件在不同接口中出现多个名称。

## 6. 模拟客户端流程

模拟客户端启动流程：

```text
1. 读取本地旧配置，获得 backend_url、sn
2. 请求业务后台 bootstrap 接口
3. 拿到配置中心 base_url、project、environment、deployment_key、token、configs
4. 请求配置中心 resolve 或 config-bundle
5. 将拉取到的配置写入本地 effective config
6. 上报 deployment-sync-record
7. 持续上报 heartbeat
8. 下一轮检测到新 revision 后再次拉取并应用
```

推荐支持两种拉取方式：

- 单配置：`GET /api/open/configs/resolve`
- 整实例包：`GET /api/open/deployments/{deployment_key}/config-bundle`

首个 demo 优先使用整实例包，便于后续多配置文件扩展。

客户端上报同步记录和心跳时，也使用同一个配置标识：

```text
config = coffee-main
```

也就是说，MVP 中统一采用：

```text
ConfigFile.code = 客户端配置标识 = 同步记录和心跳的业务组件标识
```

## 7. 咖啡中间件配置样例

首版 `coffee-main` 建议覆盖真实业务常见字段：

```toml
[server]
api_base_url = "https://api.example.com"
mqtt_host = "mqtt.example.com"
mqtt_port = 1883

[device]
machine_sn = "SN001"
device_name = "store-001-coffee"
serial_port = "COM3"
baud_rate = 9600

[feature]
enable_hot_reload = true
poll_interval_seconds = 30
```

后续可加入敏感字段，用于验证脱敏和审计安全：

```toml
[secret]
mqtt_username = "demo-user"
mqtt_password = "demo-password"
```

## 8. 配置中心需要先调整的规则

为贴合演示案例和真实生产接入，部署实例规则需要调整为：

- 部署实例创建后默认 `inactive`
- `inactive` 实例不能被 open API 拉取
- 激活实例时强制生成默认 token
- `token/reset` 仅允许作用于 `active` 实例
- 部署实例创建后不允许修改 `project_id`
- 部署实例创建后不允许修改 `is_template`
- 普通实例和模板之间只能通过复制内容创建新记录
- 模板实例不能发布 Release
- 部署实例列表需要支持分页、环境筛选、状态筛选和 keyword 搜索

推荐状态：

```text
inactive
active
```

不再为部署实例引入 `archived`；停用和未启用统一使用 `inactive`，历史动作由审计日志记录。

## 9. 最小前端范围

为了跑通 demo，前端优先补齐：

1. 部署实例列表 / 详情
2. 创建 inactive 部署实例
3. 激活实例并展示一次性 token
4. Draft 编辑页
5. Preview bundle
6. 单配置发布
7. Release 历史与 diff
8. Sync records 查询
9. Heartbeats 查询

前端第一阶段不追求完整使用体验，优先保证真实业务链路可操作、可观察、可复现。

## 10. CLI / HTTP 等价验收入口

demo 应提供页面操作的等价命令或 HTTP 流程。

推荐最小流程：

```text
1. 创建项目 coffee-middleware
2. 创建配置文件 coffee-main
3. 创建 dev/staging/prod 部署实例
4. 激活部署实例并拿到 token
5. 业务后台绑定 SN -> deployment_key/token
6. 客户端通过 SN 请求业务后台
7. 客户端拉取配置中心配置
8. 客户端上报同步记录
9. 客户端持续上报 heartbeat
10. 修改 Draft 并发布新 Release
11. 客户端检测并拉取新版本
12. 管理端查看 diff、sync records、heartbeats
```

## 11. MVP 验收标准

演示案例通过的标准：

- 客户端只知道业务后台地址和 SN，不直接内置配置中心参数。
- 业务后台能返回配置中心接入参数。
- 配置中心能按部署实例提供配置包。
- 修改 Draft 并发布后，客户端能拉到新 revision。
- 客户端能上报配置拉取 / 应用结果。
- 客户端能周期上报 heartbeat。
- 管理端能看到发布历史和 diff。
- 管理端能看到 sync records 和 heartbeat。
- `inactive` 实例无法被 open API 使用。
- 激活实例时能生成 token。
- reset token 后旧 token 立即失效。
- 模板不能发布。
- 实例不能移动到另一个项目。
- 实例不能在普通实例和模板之间原地转换。

## 12. 非目标

首个演示不做：

- 广告屏和视觉服务的完整迁移
- 多租户隔离
- 复杂审批流
- 客户端 fallback 策略
- 客户端真实热加载实现
- 大规模压测
- 多凭证管理
- 任意两版 diff
- 实例跨项目迁移

## 13. 后续实施顺序

建议按以下顺序推进：

1. 固化本文档
2. 调整部署实例生命周期与接口约束
3. 更新 DB_SCHEMA / ADMIN_API / FRONTEND_MVP_BLUEPRINT
4. 实现部署实例 `inactive / activate / pagination`
5. 补最小部署实例前端
6. 实现模拟业务后台
7. 实现模拟客户端
8. 补 Draft / Publish / Diff 最小前端
9. 补 Sync records / Heartbeats 最小前端
10. 用 demo 反向修正配置中心模型和接入规范
