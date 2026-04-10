# 0002 必选配置 / 预览 / Draft 候选澄清

## 背景

围绕 `ConfigFile`、`Draft`、`Release`、实例预览，MVP 需要进一步收紧规则，避免前后端对“是否可发布”“如何预览最终效果”理解不一致。

## Q1: 项目层能不能定义“必选配置文件”？

可以。

本轮收口后的目标语义：

- `ConfigFile` 增加项目级布尔属性 `is_required`
- `is_required = true` 表示该项目下所有可发布实例都必须“存在”这份配置
- 这里的“存在”是发布规则意义上的存在，不要求实例在创建时立刻有内容

## Q2: “实例中存在该配置”具体是什么意思？

当前明确为：

- 该实例下该配置有当前 Draft
- 或该实例下该配置至少已有一条历史 Release

满足其一即可视为“该实例已有这份配置”。

所以：

- 必选配置不要求必须先发过版
- 也不要求必须同时存在 Draft 和 Release
- 只要二者之一存在，就满足发布前置条件

## Q3: 项目里能不能有部分配置文件在某些实例上完全缺失？

可以，但只限非必选配置。

收口后的规则是：

- `is_required = false` 的配置文件，实例可以完全没有 Draft / Release
- `is_required = true` 的配置文件，实例在尝试发布任意单配置前，必须已经具备 Draft 或 Release

这保证了：

- 项目可以维护一组较大的配置文件清单
- 不必强迫所有实例都完整启用每份可选配置
- 但关键配置可通过 `is_required` 固定为发布门槛

## Q4: 当前系统是否支持“一个实例下同一配置文件有多个候选 Draft”？

不支持，且当前不进入 MVP。

当前和后续本轮实现都保持：

- 一个 `deployment_instance + config_file` 只有一份当前 Draft
- 不引入多候选 Draft 池
- 前端如果需要“备选配置”体验，应通过复制 / 覆盖当前 Draft 的方式完成，而不是依赖后端保存多个候选稿

这也是为什么本轮不引入新的 `draft_candidates` 或 `draft_snapshots` 表。

## Q5: 为什么还需要预览接口？

因为前端需要两类能力：

1. 管理端预览某个实例当前“最终会被消费端看到的配置效果”
2. 给开发/测试人员复制一份接近 open consumer 响应的整包 JSON，用于联调

如果没有后端预览接口，前端需要自行组合：

- 项目配置文件清单
- 当前 Draft
- 最新 Release
- 必选配置缺失状态

这会导致前端重复实现发布语义。

## Q6: 预览接口应该怎么决策内容来源？

固定规则：

- 对每个项目配置文件，优先读取当前 Draft
- 没有 Draft 时，回落到该实例该配置的最新 Release
- 非必选且完全缺失的配置可以在明细中标记为 `missing_optional`
- 必选但完全缺失的配置必须在明细中明确标记为 `missing_required`

这样前端能清楚展示：

- 当前哪些内容来自未发布的 Draft
- 哪些内容来自已发布 Release
- 哪些必选配置阻塞了后续发布

## Q7: 预览接口返回什么结构最合适？

本轮目标是同时返回两部分：

1. `items`

每个配置文件一条明细，至少包含：

- `config_file_id`
- `code`
- `name`
- `is_required`
- `source`
- `status`
- `format`
- `content`
- `revision`

2. `open_bundle_preview`

一份与开放接口 `GET /api/open/deployments/:deploymentKey/config-bundle` 响应兼容的整包 JSON，可直接复制到剪贴板，用于前端联调和人工核对。

## Q8: 当前实现和目标语义有哪些差异？

当前已具备：

- 单配置 Draft
- 单配置 Release
- open consumer 侧整包 `config-bundle`
- `ConfigFile.is_required`
- 发布时对必选配置完整性的校验
- 管理端整实例预览接口
- 单配置 clone 接口

当前仍未做的是：

- 多候选 Draft 池
- 由后端提供更高级的发布前修复建议
- 超出 MVP 的批量编排能力

## 当前结论

- `is_required` 已作为项目级发布门槛落地
- preview-bundle 已成为前端预览最终效果的标准真值接口
- 前端不应自行组合 Draft / Release / required 缺失逻辑
