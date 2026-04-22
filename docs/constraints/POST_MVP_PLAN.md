# MVP 之后的版本规划

## 1. 文档目标

这份文档记录 `mini-conf` 在 MVP 完成之后优先考虑的增强能力。

它的作用是：

- 防止 MVP 阶段把后续想法遗忘
- 让后续执行开发工作的 AI agent 更准确理解产品演进方向
- 帮助区分“当前必须做”和“后续值得做”

## 2. P1 方向

### 1. 管理端 JWT 认证

当前状态：

- 设计上支持
- MVP 只完整实现 Session Cookie

后续目标：

- 完成管理端 JWT 认证实现
- 保持与 Session 模式可切换

### 2. OAuth 2.0 接入

后续目标：

- 支持更标准的第三方认证接入
- 为企业内部统一登录场景做准备

### 3. 创建项目时选择配置组织模型

当前状态：

- MVP 默认采用 `DeploymentInstance` 模型

后续目标：

- 在创建项目时支持选择配置组织模型
- 不为了差异化而限制项目适用范围

可选模型示例：

- `deployment_instance`
- `application_namespace`
- `scope_match`

约束建议：

- 项目创建完成后，`config_model` 默认不可直接修改
- 如需切换，应通过显式迁移工具完成

### 4. 模板同步更新

这是你明确提出的重点方向。

目标能力：

- 模板部署实例后续发生变化时
- 管理后台或 API 提供一个显式执行的同步更新操作
- 使用 Diff 预览差异
- 允许批量替换到一批部署实例

原则：

- 必须是显式操作
- 不做隐式联动覆盖
- 要先看 Diff，再执行应用

补充设计方向：

- 首版不应直接做“批量自动覆盖”，而应先落地单实例、单配置文件的 **Merge Workspace**
- Merge Workspace 采用三方合并模型：`base / source / target / result`
- 交互形态参考 JetBrains / VS Code 的 merge editor，而不是简化的 GitHub Web conflict editor
- 结果始终写回 Current Draft，保存前自动生成一条 Saved Version 作为回退点
- 详细方案见 `docs/constraints/product-qa/0011-merge-workspace-and-visual-config-editor.md`

### 5. 批量替换与批量发布

后续目标：

- 对一批部署实例执行批量配置替换
- 对一批部署实例执行批量发布
- 提供预览、确认和审计

### 6. 敏感配置字段加密存储

当前状态：

- MVP 只先实现敏感配置脱敏展示与日志裁剪

后续目标：

- 支持字段级加密存储
- 预留外部 KMS 或密钥轮换集成点

## 3. P2 方向

### 1. Scope / labels 动态匹配

当前状态：

- MVP 不作为主路径能力

后续目标：

- 在 `DeploymentInstance` 之上扩展 `Scope` / `labels`
- 用于动态分群、灰度和自动匹配

### 2. 配置包增量拉取

后续目标：

- 对整部署实例配置包支持增量更新
- 降低重复传输成本

### 3. 灰度发布能力

后续目标：

- 支持更灵活的分批发布
- 与 Scope 能力结合

### 4. 配置内容语法高亮与彩色 Diff

当前状态：

- Release 只读详情页和 Diff 页已经具备基础文本展示能力
- 前端当前依赖只有 Vue / Element Plus / Pinia / Router，没有引入专门的语法高亮或 Diff 渲染库
- 早期安全约束中曾以 Monaco 作为编辑器示例提到“编辑内容不直接作为 HTML 渲染”，但 MVP 前端实际未安装 Monaco，也未把 Monaco 作为当前交付验收项
- 现有 Diff 页先用两列只读文本展示上一版 / 当前版本，并显示行数摘要

后续目标：

- 在 Release 详情页、Release Diff 页、preview-bundle 和 Draft 编辑页的只读预览区域支持 YAML / TOML 语法高亮
- 在 Diff 页支持新增 / 删除 / 修改行的颜色区分，必要时支持行号和折叠未变更上下文
- 敏感配置被脱敏时仍保持高亮和 Diff 结构稳定，不因为脱敏占位符破坏阅读体验
- 把 Draft 编辑、Release 只读查看、Diff 和 Merge 收束为统一的 Config Workspace 视觉体系

实施建议：

- 优先把编辑、只读查看、Diff 和 Merge 统一到一套代码工作区方案中，而不是继续把只读和编辑拆成不同栈
- 当前更推荐优先评估 CodeMirror 6：它已有官方 merge view，可支撑只读 Diff 和后续 Merge Workspace
- 语法高亮应与 Merge Workspace 方案一起规划，避免先做一套只读高亮、再做另一套 merge 工作台
- 如果后续 Draft 编辑器确实需要更重的 IDE 体验，再单独评估 Monaco；但当前不建议先上 Monaco 再自行拼 merge editor
- 加入 E2E/组件层检查：确认 YAML / TOML 内容可见、secret redaction 可见、首个发布版本不显示误导性删除行、普通变更能展示新增/删除颜色

## 4. 实际业务背景

这个路线图主要来自你的 `coffee-legacy` 场景：

- 一个项目下存在多个进程
- 一台机器上多个进程共享同一份凭证
- 新机器或新门店上线时，需要快速复制一整套配置
- 模板后续变化时，希望显式查看 Diff 并批量同步到多个部署实例

这也是为什么“模板同步更新 + Diff 批量替换”被记录为 MVP 后的重要方向。
