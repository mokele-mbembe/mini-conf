# 0003 Release Diff 澄清

## 背景

围绕 `Release` 历史查看与前端 Diff 页，当前仓库已经有：

- `POST /api/releases/publish`
- `GET /api/releases`
- `GET /api/releases/:id`
- `GET /api/releases/:id/diff`

这份文档用于固定已经落地的 diff 语义，避免前后端继续按“自由 compare”方式理解。

## Q1: `GET /api/releases/:id/diff` 比较什么？

MVP 固定比较：

- 当前 `release`
- 与同一 `deployment_instance_id + config_file_id` 下的上一条 release

不支持：

- 任意两条 release 自由比较
- 通过 `compare_to` 指定另一条基线 release

这样做的原因是：

- 当前产品主路径是“查看某次发布相对上一版改了什么”
- 与 `KICKOFF.md` 中“发布时生成与上一版的 Diff”一致
- 前端 Diff 页和发布历史页都能直接复用这条语义

## Q2: “上一版”如何确定？

当前明确为：

- 只在同一个 `deployment_instance + config_file` 维度内找上一版
- 排序基线按 `published_at DESC, id DESC`
- 对当前 release 来说，取紧邻它之前的一条 release 作为 `base_release`

这保证：

- 不会跨实例比较
- 不会跨配置文件比较
- 即使同一秒内有多次发布，也能通过 `id` 保持稳定顺序

## Q3: 如果这是首个 release 怎么办？

首个 release 仍然允许查询 diff。

此时返回语义为：

- `base_release = null`
- `before_content = null`
- `after_content = 当前 release.content`
- `diff_summary.is_initial = true`

也就是说，首发视为“从空版本到当前内容”的一次初始变更。

## Q4: Diff 做到什么粒度？

MVP 只做文本级、按行比较：

- 不做 YAML / JSON / TOML 的 AST 语义 diff
- 不返回 unified diff / patch 文本
- 只返回前后内容和轻量摘要

轻量摘要固定包含：

- `is_initial`
- `has_changes`
- `added_lines`
- `removed_lines`

这样足以支撑：

- 前端自己用 DiffEditor 展示前后文本
- 管理端列表/详情页显示“是否有变化”和大致变化规模

## Q5: `diff_summary` 什么时候生成？

本轮收口后的目标语义：

- 在 `POST /api/releases/publish` 时就生成并写入 `releases.diff_summary`
- `GET /api/releases/:id` 直接回显已落库的摘要
- `GET /api/releases/:id/diff` 基于该摘要，并补齐 `base_release`、`before_content`、`after_content`

这样做的原因是：

- 发布时就能固定这次变更的轻量摘要
- 不需要每次列表/详情查询都临时计算
- 又保留了 diff 明细接口按需返回完整前后文本的能力

## Q6: 当前实现与目标语义的差异在哪里？

当前实现应保持：

- `GET /api/releases/:id/diff` 已可用
- `diff_summary` 为稳定结构，而不是随意 JSON
- 重复发布相同内容时，仍生成新 release，但 `has_changes = false`

## 当前结论

- 前端 Diff 页应固定渲染“当前 release 与上一版”的比较结果
- release detail 和 diff 都可以直接消费后端返回的摘要与前后文本
