# 0007 配置标识 / 客户端组件 / 心跳澄清

## 背景

咖啡中间件演示案例讨论中发现，当前模型里同时出现了：

- `ConfigFile.code`
- open API 的 `config`
- sync records / heartbeats 中的 `process_key`

如果这些字段在业务上都指向“某个客户端组件需要的一份配置”，就会造成不必要的心智负担。MVP 应避免一个对象多个名称的设计风格。

## Q1: `process_key` 是否是配置中心主路径所需概念？

不是。

MVP 主路径不再引入独立 `process_key` 概念。

配置中心需要围绕下面这组字段定位客户端配置行为：

```text
project + environment + deployment_key + config
```

其中：

- `deployment_key` 表示哪个部署实例
- `config` 表示该部署实例下哪份配置文件
- `config` 对应配置中心的 `ConfigFile.code`

## Q2: 客户端组件标识和配置文件标识是什么关系？

MVP 固定为同一个标识。

```text
ConfigFile.code = 客户端配置标识 = 同步记录和心跳的业务组件标识
```

例如：

```text
coffee-main
ad-screen
vision-service
```

这些值既是配置中心里的配置文件标识，也是客户端拉取、上报同步记录和上报心跳时使用的标识。

## Q3: 业务后台 bootstrap 应返回什么？

业务后台只需要告诉客户端要接入哪些配置：

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

不再返回：

```json
{
  "process_key": "coffee-middleware"
}
```

## Q4: sync records 如何调整？

当前 `deployment_sync_records` 已经有 `config_file_id`，因此 MVP 应移除或废弃 `process_key`。

同步记录应绑定到：

```text
deployment_instance_id + config_file_id
```

而不是：

```text
deployment_instance_id + process_key
```

## Q5: heartbeats 如何调整？

心跳也应绑定到配置文件标识。

推荐请求体从：

```json
{
  "project": "coffee-middleware",
  "environment": "prod",
  "deployment_key": "a-prod-store-001",
  "process_key": "coffee-middleware"
}
```

调整为：

```json
{
  "project": "coffee-middleware",
  "environment": "prod",
  "deployment_key": "a-prod-store-001",
  "config": "coffee-main"
}
```

数据库建议从：

```text
deployment_heartbeats.process_key
unique(deployment_instance_id, process_key)
```

调整为：

```text
deployment_heartbeats.config_file_id
unique(deployment_instance_id, config_file_id)
```

## Q6: 如果未来一个进程需要多份配置怎么办？

MVP 暂不为这个场景引入新对象。

未来如果真的出现“一个进程拉多份配置”或“一份配置被多个进程共享但需要分开观测”的需求，再单独设计客户端组件模型，例如：

```text
ClientComponent
```

但在当前业务中，首阶段只迁移咖啡中间件，一个组件对应一份主配置。后续广告屏和视觉服务也可以自然表达为不同 `ConfigFile.code`。

## 当前结论

- MVP 不再引入独立 `process_key`
- `ConfigFile.code` 是唯一的客户端配置标识
- open API、sync records、heartbeats 都应围绕 `config` 统一表达
- 后续后端和前端调整时，应移除或废弃 `process_key` 字段
