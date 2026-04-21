# 前端任务执行与续工手册

## 1. 文档目标

这份文档把前端开发阶段的本地 Codex 执行方式正式收编到仓库里。

目标：

- 固定本地 Codex 在前端阶段的工作顺序
- 避免每次开工都重复解释“先出规格，再实现，再验收”
- 让下一次在其他开发主机或新会话里，也能快速恢复这套执行节奏

这份文档是本地 Codex 前端执行工作流的长期入口。

## 2. 当前已落地状态

截至当前仓库状态，前端已经越过初始 scaffold 阶段，管理台核心配置链路已经能闭环运行：

- `apps/web` 已初始化
- 已有登录页、项目列表页、项目详情页、配置文件页、部署实例列表 / 详情页
- 已有 Draft 编辑、Saved Versions、单配置 clone、preview-bundle、publish、Release 列表 / 详情 / Diff
- 部署实例列表已拆分模板和普通实例，归档 / 恢复 / 永久删除主路径已接入
- 已有本地联调说明：`FRONTEND_PAGE_TESTING`
- 已有前端 build check
- 已有覆盖核心管理链路的 Playwright smoke E2E

因此后续前端工作不再是“补核心链路”，而是“围绕 demo、运维可见性、权限管理和体验增强继续补齐”。

## 3. 开工前必读

前端任务开始前，Codex 至少应该先读这些文件：

- [FRONTEND_TASK_WORKFLOW.md](./FRONTEND_TASK_WORKFLOW.md)
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
- [0007-config-identity-and-heartbeats.md](../constraints/product-qa/0007-config-identity-and-heartbeats.md)
- [DEMO_SCENARIO_COFFEE_MIDDLEWARE.md](../constraints/DEMO_SCENARIO_COFFEE_MIDDLEWARE.md)
- [ADMIN_API.md](../constraints/ADMIN_API.md)

一句话定位：

- `FRONTEND_TASK_WORKFLOW`：Codex 怎么按规格、实现、验收的顺序推进
- `FRONTEND_HANDOFF`：前端不能只靠接口猜的业务语义
- `FRONTEND_IMPLEMENTATION_PLAN`：按什么顺序推进页面最顺
- `FRONTEND_WORKSPACE`：工程结构、脚本、CI 和本地运行方式
- `FRONTEND_PAGE_TESTING`：怎么联调、怎么查白屏、怎么复现 smoke

## 4. 任务执行总原则

前端开发阶段，默认由本地 Codex 直接完成代码实现，但不要第一步就进入写页面代码。

优先采用这个节奏：

1. 先让 Codex 基于当前仓库状态输出任务规范
2. 再让 Codex 把任务规范拆成可执行步骤和允许修改范围
3. 由 Codex 在本地实现
4. Codex 自查、跑必要检查、补状态分支和风险点
5. 如果需要浏览器手工测试，再启动服务交给用户验证

核心目标：

- 保留“先对齐业务语义和验收标准”的质量门
- 减少跨模型交接带来的上下文丢失和执行偏差
- 让实现、联调和验收由同一个本地上下文闭环完成

## 5. Codex 的职责

### 5.1 规格与设计

- 页面信息架构、路由结构、导航分层
- 跨页面状态模型和数据流设计
- 权限矩阵、错误态矩阵、空态矩阵
- 后端语义到前端交互的映射
- Draft / Preview / Publish / Diff 这类高业务密度流程
- 多页面一致性和抽象边界判断

### 5.2 本地实现

- 静态页面骨架和布局实现
- 明确字段定义的列表页、详情页、筛选栏
- 表单控件接线和基础校验
- API client 类型接线
- 小范围组件拆分
- 样式细化、响应式修补
- 已有明确输入输出的小组件

### 5.3 验收与收口

- 代码审阅
- 语义一致性检查
- 状态分支补漏
- 权限处理检查
- 风险提示
- 必要的小修
- `typecheck / build / lint / smoke` 等检查

### 5.4 不适合直接开写的任务

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

### 阶段 B：Codex 输出本地执行计划

执行计划至少要包含：

- 必读文件
- 目标页面或模块
- 允许修改的文件范围
- 数据获取方式
- 权限和状态分支要求
- 不允许改动的边界
- 自测要求

### 阶段 C：Codex 本地实现

Codex 负责：

- 页面编码
- 小组件抽取
- 基础联调
- 基础测试
- 样式落地

### 阶段 D：Codex 自验与交付

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
3. 让 Codex 输出本地执行计划和验收标准
4. 让 Codex 按计划实现、运行检查、给出交付说明

## 8. 当前阶段最适合的下一批任务

基于当前 scaffold 和页面现状，后续继续推进时，建议优先顺序是：

1. 咖啡中间件 demo：demo seed、客户端程序或脚本、从模板复制店铺配置的演示路径
2. 项目成员页：成员列表、添加成员、角色调整、最后 admin 保护错误提示
3. sync records / heartbeats 页面：让 demo 中的配置拉取、应用和心跳上报可被管理端观察
4. audit logs 页面：按项目和事件类型查看关键操作，支撑发布、归档、删除等追溯
5. Release 详情 / Diff 体验增强：Monaco 只读语法高亮、彩色 diff、复制和跳转体验
6. 前端单元 / 组件测试基线：覆盖高状态密度组件和错误态分支

继续推进时仍然要遵守：

- 低风险 CRUD 和列表页也由 Codex 直接落地
- 高业务密度页面必须先让 Codex 出规格和验收标准
- 单轮过大的页面要拆成可验证的小批次，不要一次性铺太宽

## 9. 下次续工的最短 checklist

如果下次是在新会话或其他开发机继续：

1. 拉取最新仓库
2. 执行 `pnpm install`
3. 执行 `just dev-db-prepare-local`
4. 执行 `just run-server-local`
5. 执行 `just dev-web`
6. 先读本文件、`FRONTEND_HANDOFF`、`FRONTEND_IMPLEMENTATION_PLAN`、`FRONTEND_PAGE_TESTING`
7. 确认当前要继续的是哪个页面或模块
8. 先让 Codex 出规格和执行计划，再由 Codex 本地实现

## 10. 统一 kickoff prompt

这个 prompt 作为当前仓库里前端续工的主入口模板。

其他文档如果需要引用 kickoff prompt，优先直接指向这里，不再重复维护一份近似版本。

```text
请按以下文件协作：
- docs/collaboration/FRONTEND_TASK_WORKFLOW.md
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
- docs/constraints/product-qa/0007-config-identity-and-heartbeats.md
- docs/constraints/DEMO_SCENARIO_COFFEE_MIDDLEWARE.md
- docs/constraints/ADMIN_API.md

你先不要直接写前端代码。

这轮请按“先规格、再实现、再验收”的本地 Codex 流程推进。优先做这些事：
1. 基于现有仓库状态，输出本轮任务的页面范围、接口映射、状态矩阵、权限规则和验收标准
2. 再输出本地执行计划，说明准备修改哪些文件、按什么顺序实现、跑哪些检查
3. 等我确认或直接要求开始后，你负责本地实现、review、验收、查漏补缺和必要的小修

要求：
- 以当前仓库真实实现和文档为准
- 明确指出哪些业务逻辑不是接口设计本身能表达出来的
- 在输出规格和计划前，不要先直接进入写页面代码

本轮任务是：
[把这里替换成具体页面或模块]
```

## 11. 一句话策略

把 Codex 当成本地闭环负责人：先对齐规格，再直接实现，最后自验和交付。
