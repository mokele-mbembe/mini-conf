# Linux / WSL2 开发环境实录

## 1. 文档目的

这份文档不再记录“打算怎么做”，而是记录一次已经在本仓库真实跑通的 WSL 环境初始化结果。

本文基于 2026-04-08 在 `Fedora Linux 43 (WSL)` 上的实际搭建过程整理，目标是让后续重新创建 WSL 环境时可以直接复用，而不是再次从建议性方案里试错。

## 2. 当前已验证结果

这次实际跑通的环境状态：

- 系统：`Fedora Linux 43 (WSL)`
- 仓库路径：`/home/zjj/Projects/mini-conf`
- Rust：`rustc 1.94.1`
- `cargo-nextest`：`0.9.132`
- `cargo-llvm-cov`：`0.8.5`
- `sqlx-cli`：`0.8.6`
- Node.js：`v22.22.0`
- pnpm：`9.12.3`
- PostgreSQL：`16.13`

这套环境已经实际通过下面这些命令：

- `pnpm install`
- `pnpm dlx lefthook install`
- `just db-migrate-up`
- `just test-backend-db`

其中 `just test-backend-db` 的实际结果是：

- `134 tests run: 134 passed, 0 skipped`

## 3. 当前仓库在 WSL 下的实际约束

- 仓库应放在 WSL Linux 文件系统里，不要长期在 `/mnt/c/...` 下高频编译和测试。
- WSL 不要把 `secret-tool` 或桌面 keyring 当成前置依赖。
- 本仓库的 Rust `target/`、pnpm store、Corepack 缓存和浏览器缓存应放在仓库外。
- 本仓库已经通过 `scripts/load-dev-env.sh` 和 `scripts/dev-db-env.sh` 支持从 `~/.config/mini-conf/dev-env.sh` 自动加载本地配置。

## 4. 已验证的初始化步骤

### 4.1 准备共享缓存和构建目录

```bash
sudo mkdir -p /var/cache/codex/shared/{cargo-home,rustup,corepack,npm,xdg,pnpm-store,playwright,sccache}
sudo mkdir -p /var/cache/codex/build/mini-conf/cargo-target
sudo chown -R "$USER":"$USER" /var/cache/codex
mkdir -p "$HOME/.config/mini-conf" "$HOME/.local/share/pnpm"
```

这样做的目的是：

- Rust 下载缓存、pnpm store、Corepack 缓存和 `sccache` 不写回仓库
- Rust `target/` 也不落到仓库根目录

### 4.2 写本机环境文件

当前仓库会自动加载：

```bash
~/.config/mini-conf/dev-env.sh
```

这次实际跑通时使用的是下面这种写法：

```bash
cat > ~/.config/mini-conf/dev-env.sh <<'EOF'
export CODEX_SHARED_CACHE_ROOT=/var/cache/codex/shared
export MINI_CONF_BUILD_ROOT=/var/cache/codex/build/mini-conf
export CARGO_HOME="$CODEX_SHARED_CACHE_ROOT/cargo-home"
export RUSTUP_HOME="$CODEX_SHARED_CACHE_ROOT/rustup"
export CARGO_TARGET_DIR="$MINI_CONF_BUILD_ROOT/cargo-target"
export XDG_CACHE_HOME="$CODEX_SHARED_CACHE_ROOT/xdg"
export COREPACK_HOME="$CODEX_SHARED_CACHE_ROOT/corepack"
export NPM_CONFIG_CACHE="$CODEX_SHARED_CACHE_ROOT/npm"
export PNPM_STORE_DIR="$CODEX_SHARED_CACHE_ROOT/pnpm-store"
export PLAYWRIGHT_BROWSERS_PATH="$CODEX_SHARED_CACHE_ROOT/playwright"
export SCCACHE_DIR="$CODEX_SHARED_CACHE_ROOT/sccache"
export RUSTC_WRAPPER=sccache
export PNPM_HOME="$HOME/.local/share/pnpm"
export PATH="$CARGO_HOME/bin:$HOME/.local/bin:$PNPM_HOME:$PATH"

export MINI_CONF_DB_HOST=127.0.0.1
export MINI_CONF_DB_PORT=5432
export MINI_CONF_DB_NAME=mini_conf
export MINI_CONF_DB_USER=mini_conf
export MINI_CONF_DB_PASSWORD='replace-with-a-local-dev-password'
export TEST_DATABASE_URL=''
export INIT_DB_ON_BOOT=true
EOF

chmod 600 ~/.config/mini-conf/dev-env.sh
```

说明：

- `scripts/dev-db-env.sh` 会优先读取 `MINI_CONF_DB_PASSWORD`
- 脚本会自动 URL 编码并生成 `DATABASE_URL`
- 如果 `TEST_DATABASE_URL` 为空，脚本会自动回落到 `DATABASE_URL`

因此：

- `just db-migrate-up`
- `just db-migrate-down`
- `just test-backend-db`
- `just dev-server`

都不需要额外手工拼连接串。

### 4.3 让新 shell 自动加载环境

这次实际环境里，最终采用的是把下面这段加到 `~/.bashrc`：

```bash
# mini-conf development environment
if [ -f "$HOME/.config/mini-conf/dev-env.sh" ]; then
    . "$HOME/.config/mini-conf/dev-env.sh"
fi
if [ -n "${CARGO_HOME:-}" ] && [ -f "$CARGO_HOME/env" ]; then
    . "$CARGO_HOME/env"
elif [ -f "/var/cache/codex/shared/cargo-home/env" ]; then
    . "/var/cache/codex/shared/cargo-home/env"
fi
```

这样新开 shell 时会自动拿到：

- `CARGO_HOME`
- `CARGO_TARGET_DIR`
- `PNPM_STORE_DIR`
- `MINI_CONF_DB_*`

### 4.4 安装 Fedora 43 WSL 系统依赖

这次实际可用的安装命令是：

```bash
sudo dnf upgrade --refresh -y
sudo dnf install -y \
  git curl jq ripgrep just \
  gcc gcc-c++ make pkgconf-pkg-config \
  openssl openssl-devel \
  nodejs rustup sccache \
  postgresql16 postgresql16-server postgresql16-contrib
```

这里有一个实际踩到的坑：

- `openssl-devel` 只提供开发库，不提供 `openssl` 命令行工具
- 如果你需要本机生成随机密码或调试证书，应该同时装 `openssl` 和 `openssl-devel`

### 4.5 初始化 Rust

```bash
source ~/.config/mini-conf/dev-env.sh
rustup-init -y --default-toolchain stable
source "$CARGO_HOME/env"
rustup component add rustfmt clippy llvm-tools-preview

cargo install --locked cargo-nextest
cargo install --locked cargo-llvm-cov
cargo install --locked sqlx-cli --no-default-features --features postgres,rustls
```

### 4.6 初始化 pnpm

先加载环境：

```bash
source ~/.config/mini-conf/dev-env.sh
```

如果当前 `corepack` 是从 Windows PATH 透传进来的，可能会报类似错误：

```text
/bin/sh^M: bad interpreter: No such file or directory
```

这次实际解决方式是直接安装 Linux 本地 `corepack`：

```bash
npm install -g corepack --prefix "$HOME/.local"
corepack --version
corepack enable
corepack prepare pnpm@9.12.3 --activate
```

不要依赖 Windows 那边透传进 WSL 的 `corepack`。

### 4.7 初始化 PostgreSQL

先初始化数据目录：

```bash
if [ ! -f /var/lib/pgsql/data/PG_VERSION ]; then
  sudo /usr/bin/postgresql-setup --initdb
fi
```

如果 WSL 里的 systemd 正常可用：

```bash
sudo systemctl enable --now postgresql
```

如果 systemd 不可用，回退到直接启动：

```bash
sudo -u postgres /usr/bin/pg_ctl \
  -D /var/lib/pgsql/data \
  -l /var/lib/pgsql/data/server.log \
  start
```

然后把 Fedora 默认的 `peer` / `ident` 认证改成密码认证。实际验证下来，这一步是必须的；否则 `mini_conf` 用户无法按仓库脚本通过 TCP 正常连接。

把 `pg_hba.conf` 改成：

```bash
sudo cp /var/lib/pgsql/data/pg_hba.conf /var/lib/pgsql/data/pg_hba.conf.bak
sudo tee /var/lib/pgsql/data/pg_hba.conf >/dev/null <<'EOF'
# TYPE  DATABASE        USER            ADDRESS                 METHOD

# local connections
local   all             all                                     scram-sha-256
host    all             all             127.0.0.1/32            scram-sha-256
host    all             all             ::1/128                 scram-sha-256

# replication connections
local   replication     all                                     scram-sha-256
host    replication     all             127.0.0.1/32            scram-sha-256
host    replication     all             ::1/128                 scram-sha-256
EOF

sudo -u postgres /usr/bin/pg_ctl -D /var/lib/pgsql/data reload
```

创建或更新本项目数据库用户：

```bash
source ~/.config/mini-conf/dev-env.sh

sudo -u postgres psql postgres <<SQL
DO \$\$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'mini_conf') THEN
    CREATE ROLE mini_conf LOGIN PASSWORD '${MINI_CONF_DB_PASSWORD}';
  ELSE
    ALTER ROLE mini_conf WITH LOGIN PASSWORD '${MINI_CONF_DB_PASSWORD}';
  END IF;
END
\$\$;
SQL
```

创建数据库：

```bash
sudo -u postgres psql -tAc "SELECT 1 FROM pg_database WHERE datname='mini_conf'" | grep -q 1 \
  || sudo -u postgres createdb -O mini_conf mini_conf
```

### 4.8 初始化仓库依赖

```bash
source ~/.bashrc
cd /home/zjj/Projects/mini-conf
pnpm install
pnpm dlx lefthook install
```

当前仓库里 `apps/web` 还没有初始化，所以这里的 `pnpm install` 只会安装根目录依赖，目前主要是 `prettier` 和相关工具。

### 4.9 最终验收

这次真实跑通时使用的是：

```bash
source ~/.bashrc
cd /home/zjj/Projects/mini-conf

rustc --version
cargo nextest --version
cargo llvm-cov --version
sqlx --version
node --version
pnpm --version
psql --version
just --version

just db-migrate-up
just test-backend-db
```

实际通过结果：

- `just db-migrate-up` 成功应用了 8 个 migration
- `just test-backend-db` 成功跑完 134 个测试

## 5. 当前仓库脚本在 WSL 下的实际行为

### `scripts/load-dev-env.sh`

优先顺序：

1. `MINI_CONF_DEV_ENV_FILE`
2. `~/.config/mini-conf/dev-env.sh`
3. `~/.config/mini-conf/activate-fedora43.sh`

### `scripts/dev-db-env.sh`

行为如下：

- 如果 `DATABASE_URL` 未设置，就根据 `MINI_CONF_DB_*` 变量拼接
- 如果 `MINI_CONF_DB_PASSWORD` 未设置，才尝试 `MINI_CONF_DB_PASSWORD_FILE`
- 以上都没有时，才会回退到 `secret-tool`
- `TEST_DATABASE_URL` 默认为 `DATABASE_URL`
- `INIT_DB_ON_BOOT` 默认为 `true`

### `justfile`

当前和数据库有关的实际入口是：

- `just db-migrate-up`
- `just db-migrate-down`
- `just test-backend-db`
- `just dev-server`

这些命令都会先 `source scripts/dev-db-env.sh`，所以只要 `~/.config/mini-conf/dev-env.sh` 正确，就不需要每次手工导出连接串。

## 6. 这次实际搭建里踩到的坑

- 不要把 WSL 是否有 `secret-tool` 当成前置条件；本仓库已经支持本地环境文件。
- 不要依赖 Windows 透传进来的 `corepack`；在 WSL 里它可能因为 CRLF 直接不可执行。
- Fedora 默认初始化出来的 `pg_hba.conf` 是 `peer` / `ident`，不改成 `scram-sha-256` 就无法按项目脚本完成数据库连接。
- `openssl-devel` 不等于 `openssl` 命令行工具，两者都要装。

## 7. 对 Ubuntu / Debian 的迁移原则

如果后续切回 Ubuntu / Debian，保留下面这些原则即可：

- 继续使用 `~/.config/mini-conf/dev-env.sh`
- 继续把 `CARGO_TARGET_DIR`、pnpm store 和缓存放到仓库外
- 继续让 PostgreSQL 使用密码认证，而不是依赖桌面 keyring
- 继续使用 `just db-migrate-up` 和 `just test-backend-db` 作为最终验收

也就是说，真正需要替换的主要只是系统包管理命令，而不是这套目录布局、环境变量布局和测试入口。
