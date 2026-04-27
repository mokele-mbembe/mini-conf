# mini-conf Kickoff

这份文档是根目录的续工索引，只回答“当前做到哪、下一步做什么、从哪里读起”。

面向 AI agent 的完整入口已经收口到：

- [docs/agents/AGENT_START_HERE.md](./docs/agents/AGENT_START_HERE.md)

不要再从多个 `FRONTEND_*`、handoff 和旧 checklist 中拼启动 prompt。

## 1. 当前整体状态

截至当前仓库真实状态：

- 后端 MVP 配置中心主链路已基本完成。
- 项目级权限、平台级权限、审计日志、开放接口主路径已落地。
- 配置标识已收口为 `ConfigFile.code / open config / config_file_id`，后端主路径不再使用 `process_key`。
- 部署实例运行态仍是 `active / inactive`；归档和删除通过 `is_archived / deleted_at / deployment_uid` 表达。
- `apps/web` 已有真实管理台，不再是 scaffold：
  - 登录、setup、首次改密
  - 平台用户管理、平台项目列表和创建
  - 项目列表、配置文件、环境、部署实例
  - Draft、Saved Versions、preview-bundle、publish
  - Release history / detail / diff
  - deployment archive / restore / permanent delete
- 前端已接入 lint、format check、typecheck、build 和 Playwright E2E。
- 当前最大的剩余风险已经从“主链路缺页面”转为“上线运营骨架、文档和 runbook 收口”。

## 2. 当前未完成大项

按当前优先级排序：

1. 文档同步与入口压缩：保持 README / KICKOFF / DEVELOPMENT_LOG / constraints 与当前实现一致。
2. 上线实施方案：
   - Linux binary 发布包已提供 `just release-package`
   - 外部 PostgreSQL 16+ 作为必备基础设施
   - `config-center.mycompany.com` 独立入口域名 + 反向代理/TLS runbook 已成文
   - GitHub Actions `Release Package` workflow 已可生成 artifact，下一步是真实 staging 试部署
3. 资源生命周期与文案收口：
   - projects 删除能力和引用检查
   - config_files 删除能力和引用检查
   - 状态词、错误码、前端文案统一
4. 低风险运营页面：
   - 项目成员页
   - sync records 页面
   - heartbeats 页面
   - audit logs 页面
5. 质量补量：
   - 前端单元 / 组件测试基线
   - 更完整的页面级 E2E
   - 覆盖率持续补量
   - `sqlx-check` 恢复为强制检查的时机评估
6. Config Workspace 统一升级：
   - Draft 编辑统一升级
   - Release 只读代码视图
   - Diff 统一升级
   - Merge Workspace

## 3. 读文档顺序

通用续工：

1. [README.md](./README.md)
2. [DEVELOPMENT_LOG.md](./DEVELOPMENT_LOG.md)
3. [MAIN_DEV_CHECKLIST.md](./MAIN_DEV_CHECKLIST.md)
4. [docs/agents/AGENT_START_HERE.md](./docs/agents/AGENT_START_HERE.md)

继续前端任务：

- [docs/collaboration/FRONTEND_TASK_WORKFLOW.md](./docs/collaboration/FRONTEND_TASK_WORKFLOW.md)
- [docs/collaboration/FRONTEND_HANDOFF.md](./docs/collaboration/FRONTEND_HANDOFF.md)
- [docs/constraints/FRONTEND_MVP_BLUEPRINT.md](./docs/constraints/FRONTEND_MVP_BLUEPRINT.md)
- [docs/constraints/ADMIN_API.md](./docs/constraints/ADMIN_API.md)

继续接口、权限、数据库或安全任务：

- [docs/constraints/ADMIN_API.md](./docs/constraints/ADMIN_API.md)
- [docs/constraints/AUTH_AND_SECURITY.md](./docs/constraints/AUTH_AND_SECURITY.md)
- [docs/constraints/DB_SCHEMA.md](./docs/constraints/DB_SCHEMA.md)
- [docs/constraints/product-qa/0012-mvp-launch-operability-and-admin-model.md](./docs/constraints/product-qa/0012-mvp-launch-operability-and-admin-model.md)
- [docs/artifacts/openapi.json](./docs/artifacts/openapi.json)

继续部署实例、客户端上报或 demo 任务：

- [docs/public/CLIENT_HTTP_PROTOCOL.md](./docs/public/CLIENT_HTTP_PROTOCOL.md)
- [docs/constraints/DEMO_SCENARIO_COFFEE_MIDDLEWARE.md](./docs/constraints/DEMO_SCENARIO_COFFEE_MIDDLEWARE.md)
- [docs/constraints/product-qa/0007-config-identity-and-heartbeats.md](./docs/constraints/product-qa/0007-config-identity-and-heartbeats.md)

## 4. 常用恢复命令

只恢复上下文：

```bash
git status --short --branch
```

非 DB 基线：

```bash
just ci-local
```

数据库主路径：

```bash
just ci-local-db
```

完整本机收口：

```bash
just ci-local-full
```

前端联调：

```bash
just dev-db-prepare-local
just run-server-local
just dev-web
```

隔离 Web E2E：

```bash
just test-e2e-local
```

## 5. 使用规则

- 当前单人 MVP 前开发默认遵守 `main-first`，细则见 [MAIN_DEV_CHECKLIST.md](./MAIN_DEV_CHECKLIST.md)。
- 如果产品语义要改，先更新 `docs/constraints/` 或 `docs/constraints/product-qa/*`，再改代码。
- 如果接口或 schema 改动影响 OpenAPI，必须同步 `docs/artifacts/openapi.json`。
- 如果继续前端任务，默认先按 [FRONTEND_TASK_WORKFLOW.md](./docs/collaboration/FRONTEND_TASK_WORKFLOW.md) 输出任务规范和验收标准，再实现。
- 如果前端白屏或联调异常，不要只看 `/api/healthz`；至少验证 `/api/auth/me`、登录链路和浏览器 Console。
- `docs/archive/` 中的文档只保留历史上下文，不作为当前执行入口。
