# 前端 Workspace 最小脚手架

## 1. 文档目标

这份文档说明 `mini-conf` 在前端工程真正初始化之前，为什么先保留根级 `package.json` 和 `pnpm-workspace.yaml`。

目标是：

- 先把 Node / pnpm 工作区基础设施放进仓库
- 让 `just`、CI、格式化和前端脚本有统一入口
- 等后续在 Linux / WSL2 开工时，再把 `apps/web` 真正初始化出来

## 2. 当前仓库中的最小前端基线

当前已经提供：

- 根级 `package.json`
- 根级 `pnpm-workspace.yaml`

作用：

- 固定包管理器版本
- 统一前端脚本入口
- 让 `pnpm exec prettier` 和 CI 基线可以工作

## 3. 为什么现在就放根级 package.json

即使 `apps/web` 还没初始化，根级 `package.json` 仍然有价值：

- `just fmt-frontend` 有可依赖的执行环境
- GitHub Actions 可以提前安装 pnpm 并识别前端工作区
- 后续新增 `apps/web` 时不需要再重构整个前端工具链入口

## 4. pnpm-workspace 结构

当前约定：

```yaml
packages:
  - apps/*
```

这样后续可以自然容纳：

- `apps/web`

如果后面需要独立的前端管理台、文档站或演示应用，也可以继续放在 `apps/` 下。

## 5. 后续在 Linux / WSL2 的实际初始化步骤

建议顺序：

1. 在 Linux / WSL2 中重新拉取仓库
2. 执行 `pnpm install`
3. 初始化 `apps/web`
4. 给 `apps/web/package.json` 补齐 `lint`、`typecheck`、`test`、`test:e2e`
5. 让根级脚本继续作为统一入口

## 6. 后续前端工程建议

`apps/web` 初始化后建议至少具备：

- Vue 3
- Vite
- TypeScript
- Element Plus
- Pinia
- Vue Router
- Monaco Editor
- Vitest
- Playwright
- ESLint
- Prettier

## 7. 与 just / CI 的关系

当前 `justfile` 和 GitHub Actions 已经假设：

- 根级存在 `package.json`
- 后续可能存在 `apps/web/package.json`

所以这份最小脚手架不是占位垃圾，而是为了让后续工程初始化更顺滑。

进入真实前端开发前，建议同步准备一套本机 runtime DB 和 demo 数据：

- `just dev-db-prepare-local`
- `just run-server-local`
- `just dev-web`

如果当前本机 PostgreSQL 账号没有单独建库权限，也可以先用“同一 database 下的独立 schema”承载 runtime 数据，只要保证它和 `TEST_DATABASE_URL` 不共用同一个 `search_path` 即可。

这样前端页面开发默认能看到非空状态、角色差异、发布历史、预览、同步记录和心跳，而不是长期对着空表结构搭页面。
