# 0004 Deployment Token Reset 澄清

## 背景

`DeploymentInstance` 的开放消费端访问目前依赖：

- `deployment_credentials`
- `Authorization: Bearer <token>`

MVP 已经支持凭证鉴权，但还缺少管理端主动轮换部署实例 token 的接口。

## Q1: `POST /api/deployment-instances/:id/token/reset` 重置的是什么？

MVP 固定重置：

- 指定部署实例
- 默认凭证 `credential_name = 'default'`

不支持：

- 多命名凭证管理
- 一次性重置多个凭证

这样做与当前模型一致：

- 每个实例先只启用一份默认凭证
- 同一实例上的多个进程共享这份凭证

## Q2: 如果该实例还没有默认凭证怎么办？

当前目标语义：

- 如果不存在默认凭证，则创建默认凭证
- 创建后立即返回明文 token
- 新 token 立刻可用于 open API

也就是说，`token/reset` 在 MVP 中同时承担：

- 首次发放默认凭证
- 后续轮换默认凭证

## Q3: 如果默认凭证已存在，如何轮换？

本轮收口后的语义是：

- 原地更新同一条 `deployment_credentials` 记录
- 覆盖 `token_hash`
- 设为 `status = 'active'`
- 清空 `last_used_at`
- 更新 `updated_at`

不采用：

- 把旧记录保留为独立 `revoked` 历史
- 同时保留两条活动凭证

原因：

- MVP 当前只有单默认凭证模型
- 现有唯一约束 `unique (deployment_instance_id, credential_name)` 已适配原地轮换
- 可以先把行为做简单、做稳定

## Q4: reset 后旧 token 与新 token 的联动规则是什么？

固定规则：

- reset 成功后，旧 token 立即失效
- reset 响应里的新 token 立即生效
- 不提供平滑过渡窗口

这意味着：

- 用旧 token 请求 open API，应返回 `401 invalid_token`
- 用新 token 请求 open API，应按正常鉴权流程继续访问

## Q5: 响应应该返回什么？

MVP 响应固定包含：

- `deployment_instance_id`
- `credential_name`
- `token_preview`
- `token`

其中：

- `credential_name` 固定为 `"default"`
- `token` 只在 reset 响应里明文返回一次
- `token_preview` 只返回固定掩码，例如 `mc_live_***`

## Q6: token 格式先做到什么程度？

MVP 先用平台自生成的不透明字符串：

- 前缀固定 `mc_live_`
- 后缀使用随机值

不做：

- JWT
- 可解析结构化字段
- 到期时间内嵌编码

## 当前建议

按下面顺序继续推进：

1. 新增 `POST /api/deployment-instances/:id/token/reset`
2. 为 token reset 补 OpenAPI 与响应模型
3. 用真实 PostgreSQL + HTTP 测试覆盖旧 token 失效 / 新 token 生效
