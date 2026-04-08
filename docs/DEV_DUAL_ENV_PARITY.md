# WSL 与 Fedora 双环境并列开发清单

## 1. 文档目的

你接下来会在两个 Linux 环境并列开发：

- Laptop 上的 WSL
- 当前 Fedora 43 工作站

这份文档的目标不是重复所有安装步骤，而是把两边必须对齐的工具和验证命令收敛成一份清单，避免出现：

- 一边能编译，一边不能
- 一边能跑 CI 对应检查，一边缺工具
- 一边能连本地数据库，一边脚本失效

## 2. 最低一致性要求

两个环境都至少需要满足：

- 都能运行 `cargo fmt`、`cargo clippy`、`cargo nextest`
- 都能运行 `bash scripts/export-openapi.sh`
- 都能运行 `just lint`、`just test`、`just openapi-check`
- 至少有一个环境能稳定运行 `just test-backend-db`

如果两个环境都要承担完整开发职责，建议两个环境都装齐数据库和 `sqlx-cli`。

截至 2026-04-08，当前这套 `Fedora Linux 43 (WSL)` 已经实际跑通：

- `just db-migrate-up`
- `just test-backend-db`

因此 WSL 不再只是“轻量写码环境”，它已经可以承担完整后端开发和数据库集成测试。

## 3. 必装工具

### 3.1 通用命令行工具

- `git`
- `curl`
- `jq`
- `ripgrep`
- `just`

### 3.2 Rust 工具链

- `rustup`
- stable toolchain
- `rustfmt`
- `clippy`
- `llvm-tools-preview`
- `cargo-nextest`
- `cargo-llvm-cov`
- `sqlx-cli`

### 3.3 Rust 原生编译依赖

- `gcc`
- `g++`
- `make`
- `pkg-config`
- OpenSSL 开发包

Ubuntu / Debian 系：

- `build-essential`
- `pkg-config`
- `libssl-dev`

Fedora 系：

- `gcc`
- `gcc-c++`
- `make`
- `pkgconf-pkg-config`
- `openssl`
- `openssl-devel`

### 3.4 Node 工具链

- Node.js 20+
- Corepack
- pnpm 9.x

### 3.5 PostgreSQL

如果该环境需要承担真实数据库集成测试或本地联调，还需要：

- PostgreSQL 16+
- `psql`
- 可选：`secret-tool`

## 4. 当前仓库最重要的本地命令

两个环境最少都要能跑通这些命令：

```bash
just lint
just test
just openapi-check
bash scripts/export-openapi.sh
```

承担真实数据库验证的环境，还要能跑：

```bash
just test-backend-db
just db-migrate-up
just dev-server
```

## 5. 当前项目对工具的实际依赖

### 必需

- Rust stable
- `cargo-nextest`
- `sqlx-cli`
- Node 20+
- pnpm
- `just`

### 推荐但不是每次都必须

- `cargo-llvm-cov`
- `sccache`
- PostgreSQL server
- `secret-tool`

## 6. WSL 特别说明

WSL 下最容易出现的不是编译问题，而是“桌面集成工具不可用”。

当前仓库里，下面这些能力在 WSL 下最容易失效：

- `secret-tool`
- keyring / session bus
- 从 Windows PATH 透传进来的 `corepack`

如果你的 WSL 环境里 `secret-tool` 不稳定，建议直接显式设置：

```bash
export MINI_CONF_DB_PASSWORD='...'
export DATABASE_URL=...
export TEST_DATABASE_URL=...
export INIT_ADMIN_USERNAME=...
export INIT_ADMIN_PASSWORD=...
```

更推荐的落点是：

```bash
~/.config/mini-conf/dev-env.sh
```

这样现有 `scripts/load-dev-env.sh` 和 `scripts/dev-db-env.sh` 会自动接上。

不要把 WSL 环境是否有桌面 keyring 当成阻塞项。
如果这台 WSL 会长期承担开发，建议把 `dev-env.sh` 和 `$CARGO_HOME/env` 的自动加载片段写进 `~/.bashrc`。

这次实际验证还确认了两个额外注意点：

- Fedora WSL 默认生成的 `pg_hba.conf` 是 `peer` / `ident`，如果不改成 `scram-sha-256`，`just db-migrate-up` 和 `just test-backend-db` 无法按项目脚本正常连接数据库。
- 如果 `corepack --version` 报 `/bin/sh^M: bad interpreter`，说明当前拿到的是 Windows 侧脚本；应直接在 WSL 内安装本地 `corepack` 并重新 `corepack prepare pnpm@9.12.3 --activate`。

## 7. Fedora 特别说明

Fedora 43 这边更适合作为“全量环境”，但现在也建议和 WSL 采用同一套本地环境布局：

- 安装完整 PostgreSQL
- 使用 `~/.config/mini-conf/dev-env.sh`
- 视个人偏好决定是否继续使用 `secret-tool`
- 在长期开发 shell 里自动加载 `dev-env.sh` 和 `$CARGO_HOME/env`
- 跑 `just test-backend-db`
- 跑 `just openapi-check`
- 跑完整本地联调

如果你只想维护一个能完整跑 DB 集成测试的环境，优先保留已经实际跑通的那套环境作为基准。

## 8. 版本对齐建议

两边至少对齐下面这些版本大类：

- Rust stable 大版本
- `cargo-nextest`
- `sqlx-cli`
- Node 主版本
- pnpm 主版本
- PostgreSQL 主版本

建议定期执行：

```bash
rustc --version
cargo nextest --version
sqlx --version
node --version
pnpm --version
psql --version
just --version
```

如果两边某条命令版本差太多，先对齐工具再排查仓库问题。

## 9. 推荐职责分工

如果你想减少维护成本，当前比较合理的分工是：

- WSL：写代码、跑 `just lint`、跑 `just test`、跑 `just db-migrate-up`、跑 `just test-backend-db`
- Fedora Workstation：作为第二套全量环境，承担本地联调、复现 CI 和性能 smoke

等后面前端和 E2E 成熟后，再决定是否让两个环境都承担全量职责。

## 10. 关联文档

- [Linux / WSL2 开发环境实录](./DEV_LINUX_WSL2.md)
- [Fedora 43 开发环境与本地 Agent 约定](./DEV_FEDORA43_WORKSTATION.md)
- [质量检查与测试收口计划](./QUALITY_CHECK_PLAN.md)
