# Agent Start Here

这份文档是当前仓库面向 AI agent 和本地自动化协作者的唯一续工入口。

如果新会话只读一个入口，先读这里；再按本文件里的任务类型跳到具体真值文档。不要再从多个 `FRONTEND_*`、handoff 或旧 checklist 中拼启动上下文。

## 1. 当前仓库状态

- 当前采用 MVP 前单人 `main-first` 开发策略，规则见根目录 [MAIN_DEV_CHECKLIST.md](../../MAIN_DEV_CHECKLIST.md)。
- 后端配置中心主链路已基本完成：项目、项目成员、环境、配置文件、部署实例、Draft、Saved Versions、Release、preview-bundle、open API、sync records、heartbeats、audit logs。
- 前端管理台主链路已打通：登录、setup、首次改密、平台用户管理、平台项目创建、项目列表、配置文件、环境、部署实例、Draft、Saved Versions、preview、publish、Release detail/diff、实例归档/恢复/永久删除、项目成员管理。
- 平台级权限模型已落地：`platform_admin` 与项目角色 `admin / editor / viewer` 分层；平台管理员默认不自动获得项目业务可见性。
- Setup 核心链路已落地：`system_settings`、setup status、setup gate、setup complete、前端 setup 页。
- 安全基线已覆盖管理端和 Open API：HttpOnly session cookie、CSRF、CSP/HSTS 等安全响应头、登录失败节流、Open API 基础限流、Open API 失败事件审计、密码强度、强制改密、禁用用户撤销 session、审计脱敏。
- 上线交付主路径已收口为 Linux binary 发布包：`just release-package` 生成 `dist/mini-conf-linux-x86_64.tar.gz`，`just release-package-check` 做部署前包体自检，生产运行模型见 [PRODUCTION_BINARY.md](../runbooks/PRODUCTION_BINARY.md)。
- 当前仍缺：真实 staging 试部署反馈、sync records/heartbeats/audit logs 的真实前端页面、前端单元/组件测试基线、文档继续压缩。

## 2. 必读顺序

任何任务开始前，先读：

1. [README.md](../../README.md)
2. [KICKOFF.md](../../KICKOFF.md)
3. [DEVELOPMENT_LOG.md](../../DEVELOPMENT_LOG.md)
4. [MAIN_DEV_CHECKLIST.md](../../MAIN_DEV_CHECKLIST.md)
5. 本文件

如果任务涉及接口、权限、数据库或安全，再读：

- [ADMIN_API.md](../constraints/ADMIN_API.md)
- [AUTH_AND_SECURITY.md](../constraints/AUTH_AND_SECURITY.md)
- [DB_SCHEMA.md](../constraints/DB_SCHEMA.md)
- [0012 MVP 上线运营闭环](../constraints/product-qa/0012-mvp-launch-operability-and-admin-model.md)
- [docs/artifacts/openapi.json](../artifacts/openapi.json)

如果任务涉及前端，再读：

- [FRONTEND_TASK_WORKFLOW.md](../collaboration/FRONTEND_TASK_WORKFLOW.md)
- [FRONTEND_HANDOFF.md](../collaboration/FRONTEND_HANDOFF.md)
- [FRONTEND_MVP_BLUEPRINT.md](../constraints/FRONTEND_MVP_BLUEPRINT.md)

如果任务涉及部署实例、客户端上报或 demo，再读：

- [CLIENT_HTTP_PROTOCOL.md](../public/CLIENT_HTTP_PROTOCOL.md)
- [DEMO_SCENARIO_COFFEE_MIDDLEWARE.md](../constraints/DEMO_SCENARIO_COFFEE_MIDDLEWARE.md)
- [0007 配置标识与心跳](../constraints/product-qa/0007-config-identity-and-heartbeats.md)

## 3. 当前优先级

按下面顺序推进，避免回到旧的“先补任意页面”模式：

1. 文档同步和入口压缩：保持 README / KICKOFF / DEVELOPMENT_LOG / constraints 与真实实现一致。
2. 上线实施交付：运行 `just staging-smoke` 收集真实 staging 试部署反馈、外部 PostgreSQL、`config-center.example.com` 示例入口域名、反向代理/TLS、初始化和生产变量清单。
3. 资源生命周期收口：projects / config_files 删除能力、引用检查、文案和错误码统一。
4. 低风险运营页：sync records、heartbeats、audit logs。
5. 测试补量：前端单元/组件测试基线、更完整页面级 E2E、覆盖率持续补量。
6. Config Workspace：Draft / Release / Diff / Merge 的统一编辑和阅读体验。

## 4. 执行规则

- 不要先写代码再反推语义。涉及产品语义时先更新 `docs/constraints/` 或对应 product-qa 文档。
- 接口或 schema 改动必须同步 `docs/artifacts/openapi.json`。
- 数据库集成测试使用隔离 schema；Rust 测试只读取 `TEST_DATABASE_URL`。
- 本地开发入口按 [STANDARD_WORKFLOW.md](./STANDARD_WORKFLOW.md) 分层，不把开发机脚本当生产部署方案。
- 前端任务按 [FRONTEND_TASK_WORKFLOW.md](../collaboration/FRONTEND_TASK_WORKFLOW.md) 先规格、再实现、再验收。
- 当前 MVP 前不要为了保留过时设计而做无意义的渐进 workaround；按 `MAIN_DEV_CHECKLIST.md` 及时转向更清晰的设计。

## 5. 常用验证

非 DB 文档/前端/后端基线：

```bash
just ci-local
```

涉及迁移、SQL、权限、发布、初始化：

```bash
just ci-local-db
```

需要本机完整收口：

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

## 6. 当前真值优先级

当文档互相冲突时，按这个顺序判断：

1. 当前代码与数据库迁移
2. 后端集成测试和 Playwright E2E
3. `docs/artifacts/openapi.json`
4. `docs/constraints/` 与 `docs/constraints/product-qa/`
5. `KICKOFF.md` 与 `DEVELOPMENT_LOG.md`
6. `docs/archive/` 中的历史文档
