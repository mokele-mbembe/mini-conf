# mini-conf Kickoff

这份文档不再承担“项目启动前规划草案”的职责，而是当前仓库的：

- 未完成工作总索引
- 新会话续工入口
- 文档导航与 prompt 集合页

角色分工：

- [README.md](./README.md)：项目定位、核心模型、当前状态总览
- [DEVELOPMENT_LOG.md](./DEVELOPMENT_LOG.md)：阶段进度、近期完成项、会话交接记录
- `KICKOFF.md`：接下来还要做什么，以及从哪组文档继续

## 1. 当前整体状态

截至当前仓库状态：

- 后端 MVP 主链路已基本完成
- 项目级权限、审计日志、开放接口主路径已落地
- `apps/web` 已初始化，已有登录页、项目列表页、项目详情骨架页和配置文件列表 / 编辑页
- 前端已接入 `lint / format:check / typecheck / build`
- GitHub Actions 已接入前端 build 和最小 Playwright smoke E2E

当前真正还在推进的大项，已经不再是“搭骨架”，而是“在现有骨架上继续完成主路径”。

## 2. 当前未完成的大项

### 2.1 前端管理台主路径

优先级最高，建议继续按模块切片推进：

- [x] 配置文件列表 / 编辑页
- [ ] 部署实例列表 / 详情页
- [ ] 模板创建实例流程
- [ ] Draft 编辑页
- [ ] preview-bundle 预览页
- [ ] release history / diff 页
- [ ] 项目成员页
- [ ] sync records / heartbeats / audit logs 页面
- [ ] 前端单元 / 组件测试基线

### 2.2 质量与 CI 收口

- [ ] 前端单元测试基线
- [ ] 更完整的前端页面级 E2E
- [ ] 覆盖率持续补量
- [ ] `sqlx-check` 恢复为强制检查的时机评估

### 2.3 后续后端 / 工程收口

- [ ] 持续补 alpha 黑盒回归
- [ ] 部署与运行文档继续收口
- [ ] OpenAPI / 文档 / 前端语义持续对齐

## 3. 按工作主题读哪些文档

### 3.1 如果继续前端任务

优先读这些：

- [DEVELOPMENT_LOG.md](./DEVELOPMENT_LOG.md)
- [FRONTEND_TASK_WORKFLOW.md](./docs/collaboration/FRONTEND_TASK_WORKFLOW.md)
- [FRONTEND_HANDOFF.md](./docs/collaboration/FRONTEND_HANDOFF.md)
- [FRONTEND_WORKSPACE.md](./docs/collaboration/FRONTEND_WORKSPACE.md)
- [FRONTEND_PAGE_TESTING.md](./docs/collaboration/FRONTEND_PAGE_TESTING.md)
- [FRONTEND_IMPLEMENTATION_PLAN.md](./docs/collaboration/FRONTEND_IMPLEMENTATION_PLAN.md)
- [FRONTEND_MVP_BLUEPRINT.md](./docs/constraints/FRONTEND_MVP_BLUEPRINT.md)
- [ADMIN_API.md](./docs/constraints/ADMIN_API.md)
- [0001-template-publish-and-clone.md](./docs/constraints/product-qa/0001-template-publish-and-clone.md)
- [0002-required-configs-and-preview.md](./docs/constraints/product-qa/0002-required-configs-and-preview.md)
- [0003-release-diff.md](./docs/constraints/product-qa/0003-release-diff.md)
- [0004-token-reset.md](./docs/constraints/product-qa/0004-token-reset.md)
- [0005-project-members-permissions-audit.md](./docs/constraints/product-qa/0005-project-members-permissions-audit.md)
- [0006-config-file-format-and-ux-alignment.md](./docs/constraints/product-qa/0006-config-file-format-and-ux-alignment.md)

### 3.2 如果继续后端 / 接口 / 黑盒 / 质量工作

优先读这些：

- [DEVELOPMENT_LOG.md](./DEVELOPMENT_LOG.md)
- [QUALITY_CHECK_PLAN.md](./docs/collaboration/QUALITY_CHECK_PLAN.md)
- [ADMIN_API.md](./docs/constraints/ADMIN_API.md)
- [CLIENT_HTTP_PROTOCOL.md](./docs/public/CLIENT_HTTP_PROTOCOL.md)
- [DB_SCHEMA.md](./docs/constraints/DB_SCHEMA.md)
- [AUTH_AND_SECURITY.md](./docs/constraints/AUTH_AND_SECURITY.md)
- [product-qa/README.md](./docs/constraints/product-qa/README.md)

### 3.3 如果要恢复本地环境或换机器续工

优先读这些：

- [BOOTSTRAP.md](./docs/public/BOOTSTRAP.md)
- [DEV_LINUX_WSL2.md](./docs/agents/DEV_LINUX_WSL2.md)
- [DEV_FEDORA43_WORKSTATION.md](./docs/agents/DEV_FEDORA43_WORKSTATION.md)
- [REPO_INIT_CHECKLIST.md](./docs/collaboration/REPO_INIT_CHECKLIST.md)

## 4. 下次开工建议命令

### 4.1 恢复上下文

```bash
git status --short
cargo test --workspace
bash scripts/export-openapi.sh
just coverage-check
```

### 4.2 继续前端主路径

```bash
pnpm install
just dev-db-prepare-local
just run-server-local
just dev-web
```

### 4.3 按接近 CI 的方式复现前端 smoke

```bash
pnpm --dir apps/web build
PLAYWRIGHT_BASE_URL=http://127.0.0.1:4173 pnpm --dir apps/web test:e2e
```

## 5. 续工 prompt 集合

### 5.1 前端续工主入口

前端续工不要再在多个文档里找 prompt，统一以 [FRONTEND_TASK_WORKFLOW.md](./docs/collaboration/FRONTEND_TASK_WORKFLOW.md) 第 10 节为准。

下次可以直接先贴这一段：

```text
请先阅读 DEVELOPMENT_LOG.md，然后按 docs/collaboration/FRONTEND_TASK_WORKFLOW.md 第 10 节的统一 kickoff prompt 继续。

本轮任务是：
[把这里替换成具体页面或模块]
```

### 5.2 通用续工入口

如果下次不是只做前端，而是先让模型帮你判断“当前最该推进哪一块”，可以直接贴这段：

```text
请先阅读：
- README.md
- KICKOFF.md
- DEVELOPMENT_LOG.md

对仓库的分支使用策略请参照规范:
- MAIN_DEV_CHECKLIST.md

如果本轮涉及前端，再补读：
- docs/collaboration/FRONTEND_TASK_WORKFLOW.md
- docs/collaboration/FRONTEND_HANDOFF.md
- docs/collaboration/FRONTEND_WORKSPACE.md
- docs/collaboration/FRONTEND_PAGE_TESTING.md
- docs/collaboration/QUALITY_CHECK_PLAN.md
- docs/constraints/FRONTEND_MVP_BLUEPRINT.md
- docs/constraints/ADMIN_API.md

你先不要直接写代码。

请先基于当前仓库真实状态，总结：
1. 当前已经完成了什么
2. 还有哪些大项未完成
3. 本轮最值得推进哪一项
4. 如果继续前端，应该先输出什么任务规范和验收标准
```

## 6. 使用规则

- 如果要继续前端任务，默认先走任务规范和执行计划，再由 Codex 本地实现并自验
- 如果产品语义要改，先更新 `docs/constraints/product-qa/*` 或相关约束文档，再改代码
- 如果继续收口配置文件页的格式、状态、schema 痕迹、中文字段命名或后续 i18n 入口，先看 `0006-config-file-format-and-ux-alignment.md`
- 如果前端出现白屏或联调异常，不要只看 `/api/healthz`，至少同时验证 `/api/auth/me`、登录链路和浏览器 Console
- 如果复现前端 smoke，本地和 CI 都要保证后端真正建立 `db_pool`
