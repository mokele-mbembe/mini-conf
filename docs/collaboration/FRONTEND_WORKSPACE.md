# 前端 Workspace 与运行方式

## 1. 文档目标

这份文档说明当前仓库里的前端工程已经落地到什么程度，以及本地开发、联调和 CI 现在依赖哪些入口。

目标：

- 让新开发机能快速把 `apps/web` 跑起来
- 说明当前前端工程的技术栈和脚本入口
- 记录本地联调与 CI 现在已经具备的最小能力

## 2. 当前仓库中的前端基线

当前仓库已经同时具备：

- 根级 `package.json`
- 根级 `pnpm-workspace.yaml`
- `apps/web`

这意味着前端已经从“只有 workspace 壳子”进入“已有真实应用 scaffold”的阶段。

## 3. 当前前端工程结构

当前前端应用位于：

- `apps/web`

当前已落地的基础能力包括：

- Vue 3
- Vite
- TypeScript
- Vue Router
- Pinia
- Element Plus
- ESLint
- Prettier
- Playwright 最小 smoke E2E

当前已落地的页面基础包括：

- 登录页
- 项目列表页
- 项目详情骨架页

## 4. 包管理与脚本入口

根级仍然保留统一入口，目的是让 `just`、CI 和本地脚本都不用再分叉处理。

常用入口：

- `pnpm install`
- `pnpm --dir apps/web dev`
- `pnpm --dir apps/web build`
- `pnpm --dir apps/web lint`
- `pnpm --dir apps/web typecheck`
- `pnpm --dir apps/web test:e2e`

根级脚本和 `justfile` 仍然是团队推荐入口：

- `just dev-web`
- `just lint`
- `just test-e2e`

## 5. 本地启动顺序

推荐顺序：

```bash
pnpm install
just dev-db-prepare-local
just run-server-local
just dev-web
```

说明：

- `pnpm install`
  - 安装根级和 `apps/web` 依赖
- `just dev-db-prepare-local`
  - 准备 runtime DB
  - 执行 migrations
  - 写入 demo 数据
- `just run-server-local`
  - 启动后端
- `just dev-web`
  - 启动 Vite dev server

## 6. 当前本地联调约定

当前本地联调约定依赖：

- 前端 dev server 默认端口：`5173`
- 后端默认监听端口：`8080`
- Vite `/api` 代理默认指向：`http://127.0.0.1:8080`

如果后端监听端口不是默认值，可以通过环境变量覆盖：

```bash
VITE_API_TARGET=http://127.0.0.1:9090 pnpm --dir apps/web dev
```

更详细的联调与排障方式见：

- [FRONTEND_PAGE_TESTING.md](./FRONTEND_PAGE_TESTING.md)

## 7. 当前 CI 已覆盖的前端能力

当前 GitHub Actions 已覆盖：

- pnpm install
- frontend lint
- frontend format check
- frontend typecheck
- frontend build
- 最小 Playwright smoke E2E

这意味着前端现在已经具备：

- 静态质量门槛
- 构建门槛
- 一条真实浏览器主路径 smoke

但仍未全面展开：

- Vitest 单元测试
- 多浏览器矩阵
- 截图回归
- 大量页面级 E2E

## 8. 当前脚手架阶段最重要的约束

虽然 `apps/web` 已存在，但它仍然处在 scaffold + 第一批页面阶段。

因此后续开发应优先遵守：

- 先看 `FRONTEND_TASK_ROUTING`
- 先让 Codex 出规格，不直接让模型自由发挥写页面
- 复杂页面先做任务拆分，再分流给 Copilot
- 联调或白屏问题优先按 `FRONTEND_PAGE_TESTING` 的顺序排查

## 9. 续工建议

如果下次在新会话或其他开发机继续，建议最少先读：

- [FRONTEND_TASK_ROUTING.md](./FRONTEND_TASK_ROUTING.md)
- [FRONTEND_HANDOFF.md](./FRONTEND_HANDOFF.md)
- [FRONTEND_IMPLEMENTATION_PLAN.md](./FRONTEND_IMPLEMENTATION_PLAN.md)
- [FRONTEND_PAGE_TESTING.md](./FRONTEND_PAGE_TESTING.md)

这样能最快恢复：

- 当前前端已经做到哪里
- 当前怎么跑本地联调
- 当前怎么继续按 Codex / Copilot 分工推进
