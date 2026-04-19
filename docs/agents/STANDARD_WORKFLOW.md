# 标准 Linux 开发与部署工作流

## 1. 文档定位

这份文档是 `mini-conf` 当前唯一的工程工作流规范。

目标是把本地开发、隔离数据库测试、共享黑盒环境和生产部署收口成同一套规则，避免：

- 不同开发机各跑各的命令
- 测试环境变量约定分叉
- 本地通过但 CI / staging / production 的数据库契约不一致
- 把开发期本地脚本误当成长期部署标准

平台安装步骤、故障排查和机型差异，继续记录在各自的环境文档里；但工作流规范以本文为准。

## 2. 基本原则

- Linux-first；不承诺 Windows / PowerShell 作为主工作流
- 仓库命令入口统一使用 `just`
- MVP 发布前允许采用 `single-maintainer main-first` 工作模式
- 运行时、迁移和 CI 一律以显式 DSN 为准
- 本机环境入口统一使用 `~/.config/mini-conf/dev-env.sh`
- `secret-tool` 只作为本机兼容选项，不是标准前提
- OpenAPI 生成物必须和功能改动一起提交
- 数据库测试代码只读取 `TEST_DATABASE_URL`
- 运行时服务和迁移命令只读取 `DATABASE_URL`
- 默认采用 `database-per-instance`
- 允许同一个 PostgreSQL server 承载多套 `mini-conf` database
- 不把数据库名绑定到产品名；数据库名由部署者按场景自定义

`single-maintainer main-first` 的边界：

- 仅适用于当前单人开发、尚未上线、业务设计仍可能推翻的阶段
- 默认开发分支就是 `main`
- 不要求所有功能先走 PR / 长期特性分支
- 允许为了排查 CI、验证高风险改动或导出临时 patch 开短命分支
- 临时分支完成后优先使用线性历史并回 `main`，不保留长期并行开发线

## 3. 五层工作流模型

### 3.1 Core

所有 Linux 协作者都必须支持：

- `just lint`
- `just test`
- `just openapi-check`
- `just ci-local`

`Core` 是最小一致性基线，用于保证任何 Linux 环境都能完成非数据库前提下的代码质量与契约校验。

### 3.2 Isolated DB

承担数据库主路径开发、后端真实 PostgreSQL 集成测试和 PR 级 DB 校验的环境，额外必须支持：

- `just db-migrate-up`
- `just test-backend-db`
- `just ci-local-db` 或等价组合
- `just test-e2e-local`，用于需要浏览器和真实 HTTP 服务的前端 smoke

约束：

- 使用显式 `DATABASE_URL` 和 `TEST_DATABASE_URL`
- `TEST_DATABASE_URL` 不能由 `DATABASE_URL` 隐式派生
- 数据库集成测试必须使用隔离 schema
- Web E2E 必须从 `TEST_DATABASE_URL` 派生临时 schema，并由脚本自动启动临时后端和前端

### 3.3 Local Preview / UI Dev

承担前端开发前的本地联调、页面状态观察和手工 API 联调的环境，额外建议支持：

- 一套独立的本机运行库 `DATABASE_URL`
- `just db-migrate-up-local`
- `just dev-seed-demo-local`
- `just dev-db-prepare-local`
- `just run-server-local`
- `just dev-web`

约束：

- 这是本机长期保留的运行库，不是测试隔离 schema
- 推荐和 `TEST_DATABASE_URL` 分离，避免页面联调数据与测试数据互相污染
- 允许插入稳定 demo 数据用于观察页面效果
- 这层只服务本机开发体验，不搬上 CI
- demo 数据应尽量幂等，可重复执行刷新

### 3.4 Blackbox / Staging

承担共享黑盒验证、前端联调和未来管理端 / 消费端 E2E 的环境，至少要支持：

- 显式 `APP_ENV=staging`
- 显式 `DATABASE_URL`
- 独立迁移步骤
- 部署后 HTTP 级黑盒验证

约束：

- 这是长期存在的共享环境，不是开发机脚本的延伸
- 测试以 HTTP 行为为主，不依赖直接读数据库判定通过

### 3.5 Production

生产部署环境必须支持：

- 显式 `APP_ENV=prod`
- 显式 `DATABASE_URL`
- 启动前独立执行迁移
- 部署后健康检查和回滚策略

约束：

- `staging` 和 `prod` 都不允许 `INIT_DB_ON_BOOT=true`
- 不依赖 `~/.config/mini-conf/dev-env.sh`
- 不依赖 `secret-tool`

## 4. 环境变量契约

### 4.1 运行时 / 部署契约

这些变量是可移植的正式契约：

- `APP_ENV=dev|test|staging|prod`
- `DATABASE_URL`
- `HTTP_ADDR`
- `STATIC_DIR`
- `OPENAPI_EXPORT_PATH`
- `INIT_DB_ON_BOOT`
- `INIT_ADMIN_USERNAME`
- `INIT_ADMIN_PASSWORD`

### 4.2 测试契约

- `TEST_DATABASE_URL`

约束：

- Rust 数据库测试代码只读 `TEST_DATABASE_URL`
- 测试代码不自行回退 `DATABASE_URL`

### 4.3 本机开发便利变量

唯一推荐的本机入口文件：

```bash
~/.config/mini-conf/dev-env.sh
```

本机便利变量推荐使用：

- `MINI_CONF_LOCAL_DATABASE_URL`
- `MINI_CONF_LOCAL_DB_*`
- `MINI_CONF_LOCAL_TEST_DATABASE_URL`
- `MINI_CONF_LOCAL_TEST_DB_*`
- `MINI_CONF_LOCAL_TEST_USE_RUNTIME_DB=true` 仅在你明确接受本机运行库与本机测试库共用时启用

兼容说明：

- 旧的 `MINI_CONF_DB_*` 仍暂时兼容
- 但它们不再是推荐的长期命名

前端联调建议：

- 为 `Local Preview / UI Dev` 单独配置 `MINI_CONF_LOCAL_DB_*`
- 不要把页面联调长期绑定在 `MINI_CONF_LOCAL_TEST_DB_*` 上
- 推荐使用单独 database 名，例如 `mini_conf_ui_dev`、`mini_conf_test`、`mini_conf_ci`、`mini_conf_staging`
- 如果当前本机 DB 用户没有 `CREATEDB` 权限，可先退回到“同一 database 下的显式 schema + `search_path`”方案
- 当前本机若使用 `mini_conf?options=-csearch_path%3Dmini_conf_ui_dev`，则应把 `mini_conf.mini_conf_ui_dev` 视为前端联调常驻 schema
- 当前 schema 方案只作为前端开发期的便利形态；前端主路径完成后，应迁移回独立 database：`mini_conf_ui_dev` 作为 runtime DB，`mini_conf_test` 作为测试 base DB
- `public` 不应承载 mini-conf 应用数据；如果发现 `mini_conf.public` 有业务表，通常是历史默认连接串或误用无 `search_path` 连接留下的数据
- `mini_conf_<test-prefix>_<数字时间戳>` 这类 schema 是数据库集成测试临时 schema；确认没有本地测试运行时，可用 `just db-clean-test-schemas-local` 清理

## 5. 命令分层

### 5.1 Portable 命令

这些命令不加载本机环境文件，只读取当前进程环境：

- `just db-migrate-up`
- `just db-migrate-down`
- `just test-backend-db`
- `just run-server`

适用场景：

- CI
- staging
- production
- 明确导出环境变量后的本地 shell

### 5.2 Local wrapper 命令

这些命令会先加载 `~/.config/mini-conf/dev-env.sh`，再解析本机便利变量：

- `just db-migrate-up-local`
- `just db-migrate-down-local`
- `just test-backend-db-local`
- `just dev-seed-demo-local`
- `just dev-db-prepare-local`
- `just run-server-local`
- `just ci-local-db`
- `just test-e2e-local`
- `just ci-local-full`

兼容别名：

- `just dev-server` 等价于 `just run-server-local`

### 5.3 当前阶段建议

当前进入前端管理台主路径前，本机建议同时具备两套能力：

- `Isolated DB`
- `Local Preview / UI Dev`

此时推荐：

- 测试库继续通过 `MINI_CONF_LOCAL_TEST_DB_*` 驱动
- 页面联调库通过 `MINI_CONF_LOCAL_DB_*` 单独维护
- 先执行 `just dev-db-prepare-local` 准备 runtime DB 和 demo 数据
- 再分别运行 `just run-server-local` 与 `just dev-web`
- 页面生命周期或联调相关改动提交前执行 `just test-e2e-local`，不要直接把 E2E 打到长期 runtime DB

如果当前仍只做后端数据库主路径，本机最小适配优先恢复：

- `just test-backend-db-local`
- `just ci-local-db`

此时推荐：

- 只设置 `MINI_CONF_LOCAL_TEST_DB_*`
- 继续通过 `secret-tool` 解析本机测试库密码
- 暂不要求同时补齐 `MINI_CONF_LOCAL_DB_*`
- `just run-server-local` 只在需要手工联调时再启用

## 6. 数据库连接与测试契约

### 6.1 脚本层

[`scripts/local-db-env.sh`](../../scripts/local-db-env.sh) 负责：

- 从本机便利变量生成 `DATABASE_URL`
- 从独立的本机测试变量生成 `TEST_DATABASE_URL`
- 仅在显式设置 `MINI_CONF_LOCAL_TEST_USE_RUNTIME_DB=true` 时，允许本机测试库复用本机运行库

[`scripts/dev-db-env.sh`](../../scripts/dev-db-env.sh) 仅保留为旧脚本名的兼容壳。

### 6.2 Rust 测试层

数据库集成测试统一复用 [`infra::testing`](../../crates/infra/src/testing.rs)：

- `test_database_url(...)`
- `unique_schema_name(...)`
- `with_search_path(...)`

约束：

- 每个数据库集成测试使用隔离 schema
- `setup_*` 返回 `Result<Option<...>>`
- `teardown_*` 返回 `Result`
- 当前 Rust 后端集成测试通过 in-process router 调用执行，不绑定真实 TCP 端口
- `HTTP_ADDR` 只影响显式运行的 server 进程，不影响现有 Rust 集成测试

### 6.3 端口与多进程共存

- `run-server` / `run-server-local` 只通过 `HTTP_ADDR` 绑定监听地址
- 默认端口是 `0.0.0.0:8080`
- 当前没有自动探测空闲端口，也没有端口冲突回退
- 如果需要同机并存多个后端进程，必须为每个进程显式设置不同 `HTTP_ADDR`
- 推荐使用 `127.0.0.1:18080`、`127.0.0.1:18081` 这类高位端口

## 7. 提交前执行顺序

默认提交前顺序：

1. `just lint`
2. `just test`
3. `just openapi-check`
4. 可选：`just ci-local`

涉及数据库主路径时，再额外执行：

5. `just db-migrate-up` 或 `just db-migrate-up-local`
6. `just test-backend-db` 或 `just test-backend-db-local`
7. 优先使用 `just ci-local-db`，或直接跑 `just ci-local-full`

涉及前端页面生命周期、登录、代理、真实 HTTP 或管理端主链路时，额外执行：

8. `just test-e2e-local`；如果同时涉及数据库主路径，直接执行 `just ci-local-full`

说明：

- `just openapi-check` 失败通常表示接口定义已变化，但 `docs/artifacts/openapi.json` 没有同步提交
- 不接受“单独补一个 refresh generated spec 空提交”作为标准做法
- OpenAPI 生成物应和功能改动处于同一个提交序列中

### 7.1 MVP 前单人 `main` 提交约定

在 MVP 发布前，默认按下面的简化纪律使用 `main`：

1. 日常开发直接在 `main` 上进行
2. 每次提交尽量只覆盖一个主题，避免把 schema、接口、前端脚手架和文档大杂烩塞进同一提交
3. 非 DB 改动 push 前至少执行 `just ci-local`
4. 涉及迁移、SQL、权限、发布链路、运行库初始化或前端主链路时，额外执行 `just ci-local-db` / `just test-e2e-local`；需要时直接执行 `just ci-local-full`
5. push 后以 GitHub Actions 结果为最终兜底；若 CI 失败，优先立即修复，不在 `main` 上继续叠加新主题
6. 临时分支只用于隔离高风险实验或修复 CI；回收时优先 `git merge --ff-only`

补充约束：

- 不为了省 Actions 分钟而关闭缓存、覆盖率或关键回归检查
- 可以为了减少重复执行而少开分支，但不能跳过本地最小门禁
- 当前没有多人协作前，不强制 PR 审核；但一旦进入多人协作或出现 staging / production 使用者，应重新切回分支与 PR 为默认入口

## 8. CI 与长期环境对齐

GitHub Actions 与工作流层级对应关系：

- `quality` job 对应 `Core`
- `backend-db` job 对应 `Isolated DB`

约束：

- `quality` 必须覆盖 `just lint`、`just sqlx-check`、`just openapi-check`、`just test`、`just perf-smoke`
- `backend-db` 必须提供 PostgreSQL，并通过显式 DSN 执行 `just db-migrate-up`、`just test-backend-db`
- CI 不依赖 `secret-tool`
- CI 不使用本机 local wrapper 命令

后续新增时：

- `staging-blackbox` 应对应 `Blackbox / Staging`
- 生产发布工作流应独立建模，不复用开发机脚本

## 9. 关联文档

- [WSL 与 Fedora 平台并行对齐说明](./DEV_DUAL_ENV_PARITY.md)
- [Linux / WSL2 开发环境实录](./DEV_LINUX_WSL2.md)
- [Fedora 43 开发环境与本地 Agent 约定](./DEV_FEDORA43_WORKSTATION.md)
- [质量检查与测试收口计划](../collaboration/QUALITY_CHECK_PLAN.md)
- [仓库初始化清单](../collaboration/REPO_INIT_CHECKLIST.md)
