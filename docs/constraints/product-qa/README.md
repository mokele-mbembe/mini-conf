# Product Q&A

这个目录用于记录 MVP 阶段的产品细节澄清、规则收敛和当前代码选择。

适用场景：

- 领域概念容易产生歧义
- 当前实现与最初草案之间出现偏差
- 某个能力需要先确定规则，再继续编码

约定：

- 每次澄清新增一篇独立文档
- 建议按时间顺序连续编号
- 文档优先回答：
  - 业务上希望怎样
  - 当前代码实际上怎样
  - 两者差异在哪里
  - 下一步要如何调整
- 相关设计变化先更新这里和计划文档，再做代码

当前条目：

- `0001-template-publish-and-clone.md`
- `0002-required-configs-and-preview.md`
- `0003-release-diff.md`
- `0004-token-reset.md`
- `0005-project-members-permissions-audit.md`
- `0006-config-file-format-and-ux-alignment.md`
- `0007-config-identity-and-heartbeats.md`
- `0008-current-draft-saved-versions-and-release-workspace.md`
- `0009-saved-versions-api-and-rollout.md`
- `0010-release-readonly-template-split-and-deployment-archive.md`
- `0011-merge-workspace-and-visual-config-editor.md`
- `0012-mvp-launch-operability-and-admin-model.md`

覆盖关系：

- `0012` 覆盖 `0005` 中“任意已登录用户可以创建项目”的旧项目创建语义。
- 当前真值是：只有 `platform_admin` 可以创建项目，创建时必须指定首个项目 `admin`，平台管理员默认不自动拥有项目业务可见性。
