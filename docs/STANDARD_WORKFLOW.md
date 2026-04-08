# 标准 Linux 开发工作流

## 1. 文档定位

这份文档是 `mini-conf` 当前唯一的工程工作流规范。

目标是把不同 Linux 环境中的开发、测试、迁移、OpenAPI 检查和 CI 校验收口成同一套规则，避免：

- 不同开发机各跑各的命令
- 测试环境变量约定分叉
- 本地通过但 GitHub Actions 缺同类验证
- WSL、Fedora 或未来其他 Linux 协作者出现各自维护一套流程

平台安装步骤、故障排查和机型差异，继续记录在各自的环境文档里；但工作流规范以本文为准。

## 2. 基本原则

- Linux-first；不承诺 Windows / PowerShell 作为主工作流
- 仓库命令入口统一使用 `just`
- 本机环境入口统一使用 `~/.config/mini-conf/dev-env.sh`
- `secret-tool` 只作为兼容选项，不是标准前提
- OpenAPI 生成物必须和功能改动一起提交
- 数据库测试代码只读取 `TEST_DATABASE_URL`
- 运行时服务和迁移命令使用 `DATABASE_URL`

## 3. 两层工作流模型

### 3.1 Core

所有 Linux 协作者都必须支持：

- `just lint`
- `just test`
- `just openapi-check`

`Core` 是最小一致性基线。

它用于保证任何 Linux 环境至少都能完成：

- 代码格式与静态检查
- 非数据库前提下的测试
- OpenAPI 产物一致性校验

### 3.2 Full

承担数据库联调、迁移和真实 PostgreSQL 集成测试的环境，额外必须支持：

- `just db-migrate-up`
- `just test-backend-db`
- `just dev-server`

当前约定：

- 你的两台 Fedora 43 环境都按 `Full` 维护
- 未来其他 Linux 协作者至少满足 `Core`
- 需要承担数据库主路径开发时，再升级到 `Full`

## 4. 本机环境约定

唯一推荐的本机入口文件：

```bash
~/.config/mini-conf/dev-env.sh
```

推荐写入：

- 缓存目录和构建目录
- `CARGO_HOME`、`RUSTUP_HOME`、`CARGO_TARGET_DIR`
- `PNPM_STORE_DIR`、`COREPACK_HOME`
- `MINI_CONF_DB_*`
- 可选的 `TEST_DATABASE_URL=''`

约束：

- 不要求手工在 shell 中长期导出完整 `DATABASE_URL`
- 不要求依赖桌面 keyring
- `secret-tool` 可继续使用，但不是标准前提

## 5. 数据库连接与测试契约

数据库相关规则分两层：

### 脚本层

[`scripts/dev-db-env.sh`](../scripts/dev-db-env.sh) 负责：

- 从 `MINI_CONF_DB_*` 变量或显式 `DATABASE_URL` 生成 `DATABASE_URL`
- 当 `TEST_DATABASE_URL` 为空时，把它补成与 `DATABASE_URL` 一致
- 为 `just db-migrate-up`、`just test-backend-db`、`just dev-server` 提供统一入口

### Rust 测试层

数据库集成测试统一复用 [`infra::testing`](../crates/infra/src/testing.rs)：

- `test_database_url(...)`
- `unique_schema_name(...)`
- `with_search_path(...)`

约束：

- 测试代码只读 `TEST_DATABASE_URL`
- 测试代码不自行回退 `DATABASE_URL`
- 每个数据库集成测试使用隔离 schema
- `setup_*` 返回 `Result<Option<...>>`
- `teardown_*` 返回 `Result`

这样可以把“测试应该连哪个库”的决策收口在脚本层，而不是分散在各个 Rust 测试文件里。

## 6. 提交前执行顺序

默认提交前顺序：

1. `just lint`
2. `just test`
3. `just openapi-check`

涉及数据库主路径时，再额外执行：

4. `just db-migrate-up`
5. `just test-backend-db`

说明：

- `just openapi-check` 失败通常表示接口定义已变化，但 `docs/openapi/openapi.json` 没有同步提交
- 不接受“单独补一个 refresh generated spec 空提交”作为标准做法
- OpenAPI 生成物应和功能改动处于同一个提交序列中

## 7. CI 对齐规则

GitHub Actions 与本地工作流一一对应：

- `quality` job 对应 `Core`
- `backend-db` job 对应 `Full`

约束：

- `quality` 必须覆盖 `just lint`、`just sqlx-check`、`just openapi-check`、`just test`、`just perf-smoke`
- `backend-db` 必须提供 PostgreSQL，并执行 `just db-migrate-up`、`just test-backend-db`
- CI 不依赖 `secret-tool`
- CI 使用仓库标准入口，不写绕过 `just` 的旁路命令

## 8. 关联文档

- [WSL 与 Fedora 平台并行对齐说明](./DEV_DUAL_ENV_PARITY.md)
- [Linux / WSL2 开发环境实录](./DEV_LINUX_WSL2.md)
- [Fedora 43 开发环境与本地 Agent 约定](./DEV_FEDORA43_WORKSTATION.md)
- [质量检查与测试收口计划](./QUALITY_CHECK_PLAN.md)
- [仓库初始化清单](./REPO_INIT_CHECKLIST.md)
