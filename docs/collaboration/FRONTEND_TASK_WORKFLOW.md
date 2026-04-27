# 前端任务执行入口

这份文档是当前前端任务的唯一执行入口。

原来的 `FRONTEND_WORKSPACE.md`、`FRONTEND_PAGE_TESTING.md`、`FRONTEND_IMPLEMENTATION_PLAN.md` 已经归档到 [docs/archive/collaboration/](../archive/collaboration/)，其中内容被压缩到本文。业务语义仍以 [FRONTEND_HANDOFF.md](./FRONTEND_HANDOFF.md)、[FRONTEND_MVP_BLUEPRINT.md](../constraints/FRONTEND_MVP_BLUEPRINT.md)、`product-qa/*` 和 OpenAPI 为准。

## 1. 当前前端状态

当前 `apps/web` 已不是 scaffold：

- Vue 3、Vite、TypeScript、Pinia、Vue Router、Element Plus 已落地。
- 已有登录、setup、首次改密、平台用户管理、平台项目列表和创建。
- 项目内主链路已覆盖项目列表、配置文件、环境、部署实例、Draft、Saved Versions、preview-bundle、publish、Release detail/diff。
- 已有 projects / config_files 删除入口、引用冲突提示和页面级 E2E 覆盖。
- 已有项目成员列表、添加成员、角色调整、删除成员和最后 admin 保护提示。
- 已有 sync records 列表、实例 / 配置 / action / status 筛选和页面级 E2E 覆盖。
- 部署实例页已支持模板/普通实例分区、分页、搜索、激活、停用、token reset、归档、恢复、永久删除。
- Playwright E2E 已覆盖 setup、admin project create、resource lifecycle、must-change-password、Saved Versions、clone、Release detail/diff、deployment archive/delete 等主路径。
- 仍未完成真实页面的是：heartbeats、audit logs。

## 2. 任务流程

前端任务默认按这个顺序推进：

1. 先输出任务规格：页面范围、接口映射、状态矩阵、权限规则、验收标准。
2. 再输出执行计划：准备修改的文件、数据获取方式、错误处理、测试命令。
3. 本地实现。
4. 自查和验收：类型、构建、lint、必要 E2E 或手工联调。

低风险列表页可以直接按现有模式实现；高业务密度页面必须先对齐业务语义。

## 3. 必读文件

前端任务开始前至少读：

- [docs/agents/AGENT_START_HERE.md](../agents/AGENT_START_HERE.md)
- [FRONTEND_TASK_WORKFLOW.md](./FRONTEND_TASK_WORKFLOW.md)
- [FRONTEND_HANDOFF.md](./FRONTEND_HANDOFF.md)
- [FRONTEND_MVP_BLUEPRINT.md](../constraints/FRONTEND_MVP_BLUEPRINT.md)
- [ADMIN_API.md](../constraints/ADMIN_API.md)
- [docs/artifacts/openapi.json](../artifacts/openapi.json)

涉及部署实例或客户端上报，再读：

- [DEMO_SCENARIO_COFFEE_MIDDLEWARE.md](../constraints/DEMO_SCENARIO_COFFEE_MIDDLEWARE.md)
- [0007-config-identity-and-heartbeats.md](../constraints/product-qa/0007-config-identity-and-heartbeats.md)

涉及后续编辑器升级，再读：

- [FRONTEND_CONFIG_WORKSPACE_PLAN.md](./FRONTEND_CONFIG_WORKSPACE_PLAN.md)
- [0011-merge-workspace-and-visual-config-editor.md](../constraints/product-qa/0011-merge-workspace-and-visual-config-editor.md)

## 4. 本地运行

安装依赖：

```bash
pnpm install
```

本地联调：

```bash
just dev-db-prepare-local
just run-server-local
just dev-web
```

默认地址：

- 前端：`http://127.0.0.1:5173`
- 后端：`http://127.0.0.1:8080`

如果后端端口不同：

```bash
VITE_API_TARGET=http://127.0.0.1:9090 pnpm --dir apps/web dev
```

## 5. 页面排查顺序

不要一上来只看浏览器。固定按这个顺序排查：

1. 确认 runtime DB 可用。
2. 确认后端服务监听成功。
3. 确认 Vite dev server 启动。
4. 用 `curl` 验证 `/api/auth/me`、登录、项目列表。
5. 再看浏览器页面和 Console。

最小 API 验证：

```bash
curl --noproxy '*' -i http://127.0.0.1:8080/api/auth/me
```

未登录时预期返回 `401`。

登录需要先取 CSRF cookie：

```bash
tmpdir=$(mktemp -d)
cookiejar="$tmpdir/cookies.txt"

curl --noproxy '*' -sS -c "$cookiejar" \
  http://127.0.0.1:8080/api/auth/csrf >/dev/null

csrf=$(awk '/mini_conf_csrf/ { print $7 }' "$cookiejar")

curl --noproxy '*' -sS -b "$cookiejar" -c "$cookiejar" \
  -H 'Content-Type: application/json' \
  -H "X-CSRF-Token: $csrf" \
  -d '{"username":"admin","password":"admin123456"}' \
  http://127.0.0.1:8080/api/auth/login

curl --noproxy '*' -sS -b "$cookiejar" \
  http://127.0.0.1:8080/api/projects

rm -rf "$tmpdir"
```

## 6. 自测命令

前端静态检查：

```bash
pnpm --dir apps/web typecheck
pnpm --dir apps/web lint
pnpm --dir apps/web build
```

隔离 E2E：

```bash
just test-e2e-local
```

完整本机收口：

```bash
just ci-local-full
```

裸 `pnpm --dir apps/web test:e2e` 默认拒绝连接共享服务；调试已有服务时必须显式设置：

```bash
E2E_ALLOW_SHARED_SERVER=1 PLAYWRIGHT_BASE_URL=http://127.0.0.1:5173 pnpm --dir apps/web test:e2e
```

## 7. 权限规则

- `platform_admin` 可进入平台控制台、管理用户、创建项目并指定首个项目 admin。
- `platform_admin` 默认不自动看到未加入的业务项目。
- 项目 `admin` 可管理项目、成员、配置文件、部署实例、token、audit logs。
- 项目 `editor` 可编辑 Draft、clone 单配置、preview、publish、查看 release/sync/heartbeat。
- 项目 `viewer` 只读查看项目、配置文件、部署实例、release、sync、heartbeat。
- 后端是权限真值，前端隐藏按钮只改善体验。

## 8. 仍未完成的前端批次

当前下一批前端优先级：

1. heartbeats 页面：按实例、配置查看最近上报；不要自行定义在线真值。
2. audit logs 页面：按 action / resource_type / user 过滤，仅 admin 入口。
3. 前端单元 / 组件测试基线：优先覆盖高状态密度组件和权限交互。
4. Config Workspace：等平台、上线、安全、低风险页收口后再统一升级。

## 9. 常见坑

- 不要把 `publish` 理解成整实例发布；当前是单配置发布。
- 不要把 Saved Versions 当成并行可编辑 Draft；编辑器只对应 Current Draft。
- 不要再新增 `process_key`；客户端配置标识统一为 `config`，管理端筛选使用 `config_file_id`。
- 不要把 deployment archived 加回 `status`；归档用 `is_archived`，删除用 `deleted_at`。
- token reset / deactivate 没有灰度窗口，旧 token 立即失效。
- secret 内容展示和 diff 以后端脱敏结果为准，前端不要重新实现脱敏算法。
- 前端白屏时不要只查 `/api/healthz`；必须同时查 `/api/auth/me`、登录链路和 Console。

## 10. 统一 kickoff prompt

```text
请先阅读：
- docs/agents/AGENT_START_HERE.md
- docs/collaboration/FRONTEND_TASK_WORKFLOW.md
- docs/collaboration/FRONTEND_HANDOFF.md
- docs/constraints/FRONTEND_MVP_BLUEPRINT.md
- docs/constraints/ADMIN_API.md

你先不要直接写前端代码。

本轮请先基于当前仓库真实状态，输出：
1. 页面范围
2. 接口映射
3. 状态矩阵
4. 权限规则
5. 验收标准
6. 本地执行计划

本轮任务是：
[替换成具体页面或模块]
```
