# 0008 Current Draft / Saved Versions / Release Workspace 澄清

## 背景

围绕 Draft 的长期使用体验，当前前端已经暴露出一个问题：

- 用户可以编辑当前 Draft
- 可以从其他实例或最新 Release 恢复内容
- 可以发布当前 Draft
- 但缺少“从历史保存内容快速找回继续编辑”的稳定入口

这份文档用于固定下一步产品方向，避免继续沿“多个并列 Draft 候选”这条语义不清晰的路线扩展。

## Q1: 同一个实例下同一个配置文件，是否应该允许多个可直接编辑的 Draft？

不建议。

长期语义应收口为：

- 每个 `deployment_instance + config_file` 只有 1 份 **Current Draft**
- 编辑器永远只编辑这 1 份 Current Draft
- 用户历史保存内容进入 **Saved Versions**
- 已发布内容进入 **Releases**

这样可以避免：

- 不清楚当前主线 Draft 是哪一个
- 发布时不知道到底发的是哪个候选稿
- 历史候选越积越多后语义混乱

## Q2: 更合适的对象命名是什么？

建议统一为三层：

1. **Current Draft**

- 当前工作稿
- 唯一
- 可编辑
- 与文本编辑页一一对应

2. **Saved Versions**

- 当前 Draft 的历史保存版本
- 只读
- 可恢复到 Current Draft
- 不直接编辑

3. **Releases**

- 已发布版本
- 只读
- 可回看
- 可恢复到 Current Draft
- 不直接编辑

不建议继续把 Saved Versions 也命名为 Draft。

## Q3: Saved Versions 的主要用途是什么？

Saved Versions 的目标不是取代 Current Draft，而是提供：

- 改错后快速找回历史保存内容
- 给某次保存增加轻量备注
- 在不引入完整分支系统的前提下保留阶段性工作

用户操作应为：

- 编辑 Current Draft
- 保存 Current Draft
- 生成一条 Saved Version
- 需要回退时，从某条 Saved Version 恢复到 Current Draft

## Q4: Saved Versions 的命名和标识应该怎么做？

建议：

- 系统自动生成基于时间的默认标题，例如 `2026-04-20 18:42`
- 允许用户补充备注 `note`
- UI 展示时同时显示：
  - 自动时间标识
  - 用户备注（如有）
  - 保存人
  - 保存时间

这样既保证零成本保存，也支持后续人工区分。

## Q5: Release 应该怎么回看？

Release 应保持只读，不直接编辑。

回看页和右侧历史面板中，应显眼展示：

- `revision`
- `published_at`
- `published_by`
- `published_by_username`（若后端可提供）
- `change_summary`

如需重用历史 Release 内容，操作应是：

- `Restore to Current Draft`

而不是直接在 Release 对象上编辑。

## Q6: 发布流程应该怎样收口？

应保持单一规则：

- 页面只允许发布 **Current Draft**

如果用户想发布某条 Saved Version 或历史 Release：

1. 先恢复到 Current Draft
2. 再发布 Current Draft

这样可以保持：

- 发布入口单一
- 审计链清晰
- 不需要在发布弹窗里额外选择“发布哪个候选”

## Q7: 这是否等于“多候选 Draft”能力？

不等于。

这套设计提供的是：

- 1 份 Current Draft
- 多份 Saved Versions
- 多份 Releases

它不提供：

- 多个并行可编辑 Draft 分支
- 候选 Draft 之间直接切换主线
- 多人并发审批流

如果后续真的需要并行候选分支，那会是更重的模型，例如：

- `draft_branches`
- `draft_candidates`
- `approval_workflows`

这不属于当前推荐方向。

## Q8: UI 层级应该如何组织？

建议以 **配置工作台** 为主入口，而不是单页“编辑 Draft”。

推荐结构：

1. 左栏：配置文件导航

- 当前实例下所有配置文件
- 展示当前状态、是否缺失、是否有 Current Draft / Latest Release

2. 中栏：Current Draft 编辑区

- 编辑器只对应 Current Draft
- 提供保存、预览、发布按钮

3. 右栏：历史面板

- `Saved Versions`
- `Releases`
- 只读查看、对比、恢复

## Q9: 长期使用是否足够？

对当前配置中心目标，这套模型是足够的，前提是：

- 主要需求是回找历史保存内容
- 发布链路需要清晰和可审计
- 不打算短期内引入审批流或多分支并行编辑

它比“多个 Draft 并存且都可直接编辑/直接发布”更稳，长期维护成本也更低。

## Q10: 当前代码与目标方向的差异是什么？

当前已实现：

- 唯一 Current Draft
- 从其他实例复制到 Current Draft
- 从最新 Release 恢复到 Current Draft
- 发布 Current Draft
- Release 列表
- Saved Versions 后端数据模型与列表 / 详情 / 备注 / 恢复 / 删除接口初版
- Draft 编辑页中的 Saved Versions 历史面板

当前未实现但下一步建议补：

- Release 只读详情页和 Diff 页
- Release 回看中显眼展示发布账号
- 配置工作台页的独立三栏布局；当前能力仍主要承载在 Draft 编辑页中

## 当前结论

- 编辑器只对应唯一 Current Draft
- 历史保存能力应命名为 Saved Versions，而不是多个 Draft
- Release 只读回看，不直接编辑
- 发布入口始终针对 Current Draft
- 下一步前后端设计应围绕“配置工作台 + Saved Versions + Releases”展开
