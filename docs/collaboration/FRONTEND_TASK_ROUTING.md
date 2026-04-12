# 前端任务分发与续工手册

## 1. 文档目标

这份文档把前端开发阶段的模型协作方式正式收编到仓库里。

目标：

- 固定 Codex 和 Copilot 在前端阶段的职责边界
- 避免每次开工都重复解释“先出规格，再施工，再验收”
- 让下一次在其他开发主机或新会话里，也能快速恢复这套分发节奏

这份文档替代本地临时文件 `.tmp/CODEX_FRONTEND_TASK_ROUTING.md` 作为长期入口。

## 2. 当前已落地状态

截至当前仓库状态，前端已经不再是“空仓”：

- `apps/web` 已初始化
- 已有登录页、项目列表页、项目详情骨架页
- 已有本地联调说明：`FRONTEND_PAGE_TESTING`
- 已有前端 build check
- 已有最小 Playwright smoke E2E

因此后续前端工作不再是“从 0 起项目”，而是“在已有 scaffold 上继续按模块推进”。

## 3. 开工前必读

前端任务开始前，Codex 和 Copilot 至少都应该先读这些文件：

- [FRONTEND_TASK_ROUTING.md](./FRONTEND_TASK_ROUTING.md)
- [FRONTEND_HANDOFF.md](./FRONTEND_HANDOFF.md)
- [FRONTEND_IMPLEMENTATION_PLAN.md](./FRONTEND_IMPLEMENTATION_PLAN.md)
- [FRONTEND_WORKSPACE.md](./FRONTEND_WORKSPACE.md)
- [FRONTEND_PAGE_TESTING.md](./FRONTEND_PAGE_TESTING.md)
- [FRONTEND_MVP_BLUEPRINT.md](../constraints/FRONTEND_MVP_BLUEPRINT.md)
- [0001-template-publish-and-clone.md](../constraints/product-qa/0001-template-publish-and-clone.md)
- [0002-required-configs-and-preview.md](../constraints/product-qa/0002-required-configs-and-preview.md)
- [0003-release-diff.md](../constraints/product-qa/0003-release-diff.md)
- [0004-token-reset.md](../constraints/product-qa/0004-token-reset.md)
- [0005-project-members-permissions-audit.md](../constraints/product-qa/0005-project-members-permissions-audit.md)
- [ADMIN_API.md](../constraints/ADMIN_API.md)

一句话分工：

- `FRONTEND_TASK_ROUTING`：怎么分配 Codex / Copilot 的职责
- `FRONTEND_HANDOFF`：前端不能只靠接口猜的业务语义
- `FRONTEND_IMPLEMENTATION_PLAN`：按什么顺序推进页面最顺
- `FRONTEND_WORKSPACE`：工程结构、脚本、CI 和本地运行方式
- `FRONTEND_PAGE_TESTING`：怎么联调、怎么查白屏、怎么复现 smoke

## 4. 任务分流总原则

前端开发阶段，默认不要把“让 Codex 直接写页面代码”作为第一反应。

优先采用这个节奏：

1. 先让 Codex 基于当前仓库状态输出任务规范
2. 再让 Codex 把任务规范整理成给 Copilot 的执行 prompt
3. 让 Copilot 完成局部实现
4. 回到 Codex 做 review、验收、风险补洞和必要小修

核心目标：

- 把 Codex 的上下文预算留给全局设计和关键判断
- 把重复实现、样板接线、稳定 CRUD 页面交给 Copilot
- 避免两个模型同时重复做同一段探索

## 5. Codex 与 Copilot 的职责

### 5.1 优先留给 Codex 的任务

- 页面信息架构、路由结构、导航分层
- 跨页面状态模型和数据流设计
- 权限矩阵、错误态矩阵、空态矩阵
- 后端语义到前端交互的映射
- Draft / Preview / Publish / Diff 这类高业务密度流程
- 多页面一致性和抽象边界判断
- 最终验收、风险盘点、回归检查

### 5.2 优先分流给 Copilot 的任务

- 静态页面骨架和布局实现
- 明确字段定义的列表页、详情页、筛选栏
- 表单控件接线和基础校验
- API client 类型接线
- 小范围组件拆分
- 样式细化、响应式修补
- 已有明确输入输出的小组件

### 5.3 不适合直接分流的任务

- 目标本身还没想清楚
- 涉及核心业务语义澄清
- 可能影响多个页面一致性的基础抽象
- 需要结合后端真实行为判断对错
- “先看看怎么设计”这一类开放式问题

## 6. 标准工作流

### 阶段 A：Codex 输出任务规范

要求 Codex 产出：

- 范围
- 不做什么
- 路由建议
- 接口映射
- 页面状态清单
- 权限规则
- 组件拆分建议
- 实现顺序
- 验收标准

### 阶段 B：Codex 输出给 Copilot 的实现 prompt

prompt 至少要包含：

- 必读文件
- 目标页面或模块
- 允许修改的文件范围
- 数据获取方式
- 权限和状态分支要求
- 不允许改动的边界
- 自测要求

### 阶段 C：Copilot 执行实现

Copilot 负责：

- 页面编码
- 小组件抽取
- 基础联调
- 基础测试
- 样式落地

### 阶段 D：Codex 回收验收

Codex 负责：

- 代码审阅
- 语义一致性检查
- 状态分支补漏
- 权限处理检查
- 风险提示
- 必要的小修

## 7. 下次开工的正确姿势

不要只把“实现某个页面”丢给模型。

下次开工建议顺序：

1. 明确让 Codex 先读本文件和 `docs/collaboration/*`
2. 让 Codex 输出任务规范，不直接写代码
3. 让 Codex 再生成给 Copilot 的 prompt
4. Copilot 实现后，把结果贴回给 Codex 做验收

## 8. 当前阶段最适合的下一批任务

基于当前 scaffold 和页面现状，后续继续推进时，建议优先顺序是：

1. 配置文件列表页
2. 部署实例列表页
3. 项目成员页
4. Draft 编辑页
5. Preview / Publish / Release 历史 / Diff

继续分流时仍然要遵守：

- 低风险 CRUD 和列表页优先交给 Copilot
- 高业务密度页面先让 Codex 出规格

## 9. 下次续工的最短 checklist

如果下次是在新会话或其他开发机继续：

1. 拉取最新仓库
2. 执行 `pnpm install`
3. 执行 `just dev-db-prepare-local`
4. 执行 `just run-server-local`
5. 执行 `just dev-web`
6. 先读本文件、`FRONTEND_HANDOFF`、`FRONTEND_IMPLEMENTATION_PLAN`、`FRONTEND_PAGE_TESTING`
7. 确认当前要继续的是哪个页面或模块
8. 先让 Codex出规格，再发给 Copilot

## 10. 给 Codex 的推荐 kickoff prompt

```text
请按以下文件协作：
- docs/collaboration/FRONTEND_TASK_ROUTING.md
- docs/collaboration/FRONTEND_HANDOFF.md
- docs/collaboration/FRONTEND_IMPLEMENTATION_PLAN.md
- docs/collaboration/FRONTEND_WORKSPACE.md
- docs/collaboration/FRONTEND_PAGE_TESTING.md
- docs/constraints/FRONTEND_MVP_BLUEPRINT.md
- docs/constraints/product-qa/0001-template-publish-and-clone.md
- docs/constraints/product-qa/0002-required-configs-and-preview.md
- docs/constraints/product-qa/0003-release-diff.md
- docs/constraints/product-qa/0004-token-reset.md
- docs/constraints/product-qa/0005-project-members-permissions-audit.md
- docs/constraints/ADMIN_API.md

你先不要直接写前端代码。

这轮请把自己当成前端总控和验收负责人，而不是第一时间亲自施工的人。优先做这些事：
1. 基于现有仓库状态，输出本轮任务的页面范围、接口映射、状态矩阵、权限规则和验收标准
2. 再把任务整理成可交给 Copilot 执行的详细 prompt
3. 等我贴回 Copilot 的实现结果后，你再负责 review、验收、查漏补缺和必要的小修

要求：
- 以当前仓库真实实现和文档为准
- 明确指出哪些业务逻辑不是接口设计本身能表达出来的
- 不要先直接进入写页面代码

本轮任务是：
[把这里替换成具体页面或模块]
```

## 11. 一句话策略

把 Codex 当成总设计师和验收官，把 Copilot 当成按规范施工的执行者。
