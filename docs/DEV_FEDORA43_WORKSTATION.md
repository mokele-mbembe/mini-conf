# Fedora 43 开发环境与本地 Agent 约定

## 1. 文档目的

这份文档面向当前这台 Fedora 43 Workstation + GNOME 开发机。

目标有两个：

- 把 `mini-conf` 的开发工具链一次装齐
- 约束本地 agent 在保持 Default permissions 的前提下，尽量贴近正常工程实践，不因为沙箱机制造成项目内异常堆积构建缓存和临时目录

## 2. 最短执行清单

如果你现在只想先把开发底座装起来，按下面顺序复制执行即可。

创建共享缓存和项目构建目录：

```bash
sudo mkdir -p /var/cache/codex/shared/{cargo-home,rustup,corepack,npm,xdg,pnpm-store,playwright,sccache}
sudo mkdir -p /var/cache/codex/build/mini-conf/{cargo-target}
sudo chown -R "$USER":"$USER" /var/cache/codex
mkdir -p "$HOME/.config/mini-conf" "$HOME/.local/share/pnpm"
```

写激活脚本：

```bash
cat > "$HOME/.config/mini-conf/activate-fedora43.sh" <<'EOF'
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
EOF
source "$HOME/.config/mini-conf/activate-fedora43.sh"
```

安装系统依赖：

```bash
sudo dnf upgrade --refresh -y
sudo dnf install -y \
  git curl jq ripgrep just \
  gcc gcc-c++ make pkgconf-pkg-config openssl-devel \
  nodejs rustup sccache \
  postgresql16 postgresql16-server postgresql16-contrib
```

初始化 Rust：

```bash
rustup-init -y --default-toolchain stable
source "$CARGO_HOME/env"
rustup component add rustfmt clippy llvm-tools-preview
cargo install --locked cargo-nextest
cargo install --locked cargo-llvm-cov
cargo install --locked sqlx-cli --no-default-features --features postgres,rustls
```

初始化 pnpm：

```bash
source "$HOME/.config/mini-conf/activate-fedora43.sh"
corepack enable
corepack prepare pnpm@9.12.3 --activate
```

如果这里提示 `corepack: command not found`，执行这个回退方案：

```bash
npm install -g corepack --prefix "$HOME/.local"
source "$HOME/.config/mini-conf/activate-fedora43.sh"
corepack --version
corepack enable
corepack prepare pnpm@9.12.3 --activate
```

初始化 PostgreSQL：

```bash
printf '%s' "$(openssl rand -base64 36 | tr -d '\n')" | secret-tool store --label='mini-conf postgres superuser password' service mini-conf env dev role postgres-admin
printf '%s' "$(openssl rand -base64 36 | tr -d '\n')" | secret-tool store --label='mini-conf app database password' service mini-conf env dev role app-db user mini_conf
sudo /usr/bin/postgresql-setup --initdb
sudo systemctl enable --now postgresql
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
sudo systemctl reload postgresql
sudo -u postgres psql -c "ALTER USER postgres WITH PASSWORD '$(secret-tool lookup service mini-conf env dev role postgres-admin)';"
sudo -u postgres psql -c "CREATE USER mini_conf WITH PASSWORD '$(secret-tool lookup service mini-conf env dev role app-db user mini_conf)';"
sudo -u postgres psql -c "CREATE DATABASE mini_conf OWNER mini_conf;"
```

初始化仓库：

```bash
cd /home/zjj/Projects/mini-conf
source "$HOME/.config/mini-conf/activate-fedora43.sh"
pnpm install
pnpm dlx lefthook install
```

最后验收：

```bash
rustc --version
sccache --version
cargo nextest --version
cargo llvm-cov --version
sqlx --version
node --version
pnpm --version
psql --version
just --version
PGPASSWORD="$(secret-tool lookup service mini-conf env dev role app-db user mini_conf)" psql -h 127.0.0.1 -p 5432 -U mini_conf -d mini_conf -c "select current_database(), current_user;"
pnpm format:check
just bootstrap-dev
```

## 3. 基于现有计划得出的开发约束

从当前仓库文档看，`mini-conf` 的真实开发方向已经比较明确：

- Linux first，真正开发和测试环境以 Linux 为准
- 后端为 Rust + Axum + SQLx + PostgreSQL
- 前端为 Vue 3 + Vite + TypeScript + pnpm
- 命令入口统一收敛到 `justfile`
- 质量基线依赖 `cargo-nextest`、`cargo-llvm-cov`、`sqlx-cli`、`pnpm`、`lefthook`

这意味着本机至少要准备好：

- Rust stable toolchain
- Node.js + Corepack + pnpm
- PostgreSQL 16
- `sccache`
- `just`、`git`、`curl`、`jq`、`ripgrep`
- 用于 Rust 原生依赖编译的系统开发包

## 4. 本机约定

### 3.1 权限策略

- 保持 Codex 的 Default permissions
- 不额外放宽项目目录写权限
- 如需系统级安装，手工复制本文里的 `sudo` 命令执行

### 3.2 缓存策略

本项目本地开发允许这些内容出现在仓库内：

- `node_modules/`
- 后续真实源码目录

本项目本地开发不希望这些内容优先出现在仓库内：

- Rust `target/`
- `.pnpm-store/`
- `.cache/`
- `.tmp/`
- `tmp/`
- Playwright 浏览器下载目录
- 其他大体积临时构建产物

默认策略：

- 可复用的下载缓存、工具缓存、浏览器缓存使用系统级共享目录
- 项目相关构建产物放在仓库外，但按项目隔离
- 如果某个工具默认会把缓存写进仓库，先改环境变量
- 如果某个工具无法避免在仓库内生成大缓存目录，先停下来再处理，不要直接落地

### 3.3 目录组织原则

这份文档采用更接近正常工程实践的两层目录：

共享缓存目录：

```bash
/var/cache/codex/shared
```

项目构建目录：

```bash
/var/cache/codex/build/mini-conf
```

这样划分的原因是：

- `cargo` registry、`rustup`、Corepack、npm、pnpm store、Playwright 浏览器、`sccache` 这类内容适合跨项目共享
- Rust `target/` 这类工作区构建产物虽然不该写进仓库，但也不适合多个项目直接共用

如果你后面决定改路径，只要同步修改下面的激活脚本即可。

## 5. 步骤 0：准备共享缓存目录和激活脚本

先创建共享缓存目录和项目构建目录：

```bash
sudo mkdir -p /var/cache/codex/shared/{cargo-home,rustup,corepack,npm,xdg,pnpm-store,playwright,sccache}
sudo mkdir -p /var/cache/codex/build/mini-conf/{cargo-target}
sudo chown -R "$USER":"$USER" /var/cache/codex
mkdir -p "$HOME/.config/mini-conf"
mkdir -p "$HOME/.local/share/pnpm"
```

再写一个只给 `mini-conf` 使用的环境激活脚本：

```bash
cat > "$HOME/.config/mini-conf/activate-fedora43.sh" <<'EOF'
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
EOF
```

加载它：

```bash
source "$HOME/.config/mini-conf/activate-fedora43.sh"
env | grep -E 'CODEX_SHARED_CACHE_ROOT|MINI_CONF_BUILD_ROOT|CARGO_HOME|RUSTUP_HOME|CARGO_TARGET_DIR|XDG_CACHE_HOME|COREPACK_HOME|NPM_CONFIG_CACHE|PNPM_STORE_DIR|PLAYWRIGHT_BROWSERS_PATH|SCCACHE_DIR|RUSTC_WRAPPER|PNPM_HOME'
```

说明：

- 这里没有把脚本自动写进 `~/.bashrc`
- 推荐只在你准备开发 `mini-conf` 或启动本地 agent 的 shell 里手动 `source`
- `RUSTC_WRAPPER=sccache` 只有在系统里已经安装了 `sccache` 时才会生效，所以下一步会一起安装
- `CARGO_HOME/bin` 已经放进 PATH，`cargo-nextest`、`cargo-llvm-cov`、`sqlx` 这类 cargo 安装出来的工具会直接可用

## 6. 步骤 1：安装 Fedora 基础开发工具

先刷新系统：

```bash
sudo dnf upgrade --refresh -y
```

安装基础工具和本地编译依赖：

```bash
sudo dnf install -y \
  git \
  curl \
  jq \
  ripgrep \
  just \
  gcc \
  gcc-c++ \
  make \
  pkgconf-pkg-config \
  openssl-devel \
  nodejs \
  rustup \
  sccache \
  postgresql16 \
  postgresql16-server \
  postgresql16-contrib
```

验收：

```bash
git --version
rg --version
just --version
node --version
npm --version
```

说明：

- Fedora 官方文档建议 Node.js 直接安装 `nodejs`
- Fedora 官方 Rust 文档建议二选一使用 `rust/cargo` 或 `rustup`，这里选 `rustup`
- 不要同时混用 Fedora 打包的 `rust`/`cargo` 和 `rustup` 工具链

## 7. 步骤 2：安装 Rust stable toolchain

初始化 `rustup`：

```bash
rustup-init -y --default-toolchain stable
source "$CARGO_HOME/env"
rustup component add rustfmt clippy llvm-tools-preview
rustup show
sccache --version
```

验收：

```bash
rustc --version
cargo --version
cargo fmt --version
cargo clippy --version
```

## 8. 步骤 3：安装 Rust 扩展工具

安装项目规划里已经点名的工具：

```bash
source "$HOME/.config/mini-conf/activate-fedora43.sh"
cargo install --locked cargo-nextest
cargo install --locked cargo-llvm-cov
cargo install --locked sqlx-cli --no-default-features --features postgres,rustls
```

验收：

```bash
cargo nextest --version
cargo llvm-cov --version
sqlx --version
```

说明：

- 因为已经导出了 `CARGO_HOME` / `RUSTUP_HOME` / `CARGO_TARGET_DIR`，后续会形成“共享依赖缓存 + 项目隔离构建目录”的结构
- 因为已经导出了 `SCCACHE_DIR` 和 `RUSTC_WRAPPER=sccache`，Rust 编译缓存也会进入共享层
- 这比单纯依赖 `.gitignore` 更贴近正常工程实践：不是“忽略 `target/`”，而是“默认就不要在仓库里生成 `target/`”

## 9. 步骤 4：启用 Corepack 并固定 pnpm

当前仓库根级 `package.json` 固定的是：

```text
pnpm@9.12.3
```

所以这里不要直接依赖 Fedora 仓库里的 `pnpm` 版本，而是让 Corepack 按仓库声明拉起版本。

执行：

```bash
source "$HOME/.config/mini-conf/activate-fedora43.sh"
corepack enable
corepack prepare pnpm@9.12.3 --activate
pnpm --version
```

验收：

```bash
corepack --version
pnpm --version
```

如果 `corepack` 命令不存在，再检查一次 Node 安装：

```bash
node --version
npm --version
```

如果 `node` / `npm` 正常，但仍然没有 `corepack`，使用手动安装方案：

```bash
npm install -g corepack --prefix "$HOME/.local"
source "$HOME/.config/mini-conf/activate-fedora43.sh"
corepack --version
corepack enable
corepack prepare pnpm@9.12.3 --activate
pnpm --version
```

说明：

- Node 官方文档说明 Corepack 在 Node 14.19+ 到 24.x 的默认安装中通常随 Node 分发
- 但发行版打包的 Node 并不一定把 `corepack` 二进制一并放进当前 PATH
- Corepack 官方仓库提供的手动安装方式就是 `npm install -g corepack`

## 10. 步骤 5：验证并预热 sccache

如果你希望 Rust 编译更接近多项目开发环境，建议保留 `sccache`。

先确认当前 shell 已经加载激活脚本：

```bash
source "$HOME/.config/mini-conf/activate-fedora43.sh"
echo "$RUSTC_WRAPPER"
echo "$SCCACHE_DIR"
```

查看状态：

```bash
sccache --show-stats
```

后续第一次真正执行 Rust 构建后，再看缓存是否开始命中：

```bash
sccache --show-stats
```

如需手动清理共享编译缓存：

```bash
sccache --zero-stats
sccache --stop-server
rm -rf /var/cache/codex/shared/sccache/*
```

说明：

- `sccache` 共享的是 Rust 编译缓存，不是工作区的 `target/`
- 这比让多个项目硬共享一个 `target/` 更符合实际工程做法
- 如果后面你不想启用它，只要从激活脚本里去掉 `RUSTC_WRAPPER=sccache` 即可

## 11. 步骤 6：初始化 PostgreSQL 16

`mini-conf` 规划里明确是 PostgreSQL 16+。Fedora 43 已提供 `postgresql16` / `postgresql16-server` 包。

如果你希望数据库密码不写进文档或 shell 历史，可以先把密码存进 GNOME Keyring。

命令行方式：

```bash
printf '%s' "$(openssl rand -base64 36 | tr -d '\n')" | secret-tool store --label='mini-conf postgres superuser password' service mini-conf env dev role postgres-admin
printf '%s' "$(openssl rand -base64 36 | tr -d '\n')" | secret-tool store --label='mini-conf app database password' service mini-conf env dev role app-db user mini_conf
```

如果你想用图形界面查看这些密码，可安装：

```bash
sudo dnf install -y seahorse
```

然后启动 `seahorse`，在登录钥匙环里搜索：

- `mini-conf postgres superuser password`
- `mini-conf app database password`

先初始化数据库集群并启动服务：

```bash
sudo /usr/bin/postgresql-setup --initdb
sudo systemctl enable --now postgresql
systemctl --no-pager --full status postgresql
```

Fedora 默认 `pg_hba.conf` 往往是 `peer` / `ident`，而当前这套开发流程用的是密码认证，所以先把本地规则改成 `scram-sha-256`。

对一台刚初始化的本地开发机，可以先备份再整体覆盖：

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
sudo systemctl reload postgresql
```

再创建开发用角色和数据库：

```bash
sudo -u postgres psql -c "ALTER USER postgres WITH PASSWORD '$(secret-tool lookup service mini-conf env dev role postgres-admin)';"
sudo -u postgres psql -c "CREATE USER mini_conf WITH PASSWORD '$(secret-tool lookup service mini-conf env dev role app-db user mini_conf)';"
sudo -u postgres psql -c "CREATE DATABASE mini_conf OWNER mini_conf;"
```

验收：

```bash
PGPASSWORD="$(secret-tool lookup service mini-conf env dev role app-db user mini_conf)" \
psql -h 127.0.0.1 -p 5432 -U mini_conf -d mini_conf \
  -c "select current_database(), current_user;"
```

说明：

- 这里的 `postgresql-setup` / `postgresql` service 用法是基于 Fedora 官方 PostgreSQL 快速开始和 Fedora 43 的 PostgreSQL 16 包命名推断出来的
- 如果你使用随机强密码，不要把明文直接塞进 `postgres://...` URI 里做 CLI 验证；密码里可能含有 URL 保留字符，`psql` 会解析失败
- 如果你的机器提示 unit 名不对，先执行下面这条确认本机实际服务名：

```bash
systemctl list-unit-files | grep postgres
```

## 12. 步骤 7：可选安装 Docker/Compose

如果你后面想把 PostgreSQL 或联调依赖切到容器里，再执行这一步。

安装：

```bash
sudo dnf install -y moby-engine docker-compose
sudo systemctl enable --now docker
sudo usermod -aG docker "$USER"
```

重新登录 shell 后验收：

```bash
docker --version
docker compose version || docker-compose version
```

说明：

- Fedora 43 官方包名是 `moby-engine`
- 这一步不是当前文档必须项，先把本机原生 PostgreSQL 跑起来就够开发 MVP

## 13. 步骤 8：初始化仓库级 Node 依赖和 Git hooks

进入仓库：

```bash
cd /home/zjj/Projects/mini-conf
source "$HOME/.config/mini-conf/activate-fedora43.sh"
```

安装根级 Node 依赖：

```bash
pnpm install
```

安装 Git hooks：

```bash
pnpm dlx lefthook install
```

基础验收：

```bash
just --list
pnpm format:check
just bootstrap-dev
```

说明：

- 当前仓库前后端源码还没真正初始化，所以很多 `just` 任务会显示 `Skipping ...`
- 这是正常状态，说明脚手架入口已经就位，不代表环境有问题

## 14. 步骤 9：建议保留的本地环境变量模板

后续后端开始编码时，可在仓库里手工创建一个仅本地使用的环境文件，例如 `.env.local`：

```env
APP_ENV=dev
HTTP_ADDR=0.0.0.0:8080
DATABASE_URL=postgres://mini_conf:<url-encoded-password>@127.0.0.1:5432/mini_conf
DATABASE_ADMIN_URL=postgres://postgres:<url-encoded-password>@127.0.0.1:5432/postgres
INIT_DB_ON_BOOT=true
INIT_ADMIN_USERNAME=admin
INIT_ADMIN_PASSWORD=admin123456
STATIC_DIR=apps/web/dist
ADMIN_AUTH_MODE=session
JWT_ENABLED=false
OPENAPI_EXPORT_PATH=docs/openapi/openapi.json
```

当前代码还没落地，所以这一步先不用急着创建。

## 15. 启动本地 agent 前的固定动作

每次准备在这个仓库里工作前，先执行：

```bash
cd /home/zjj/Projects/mini-conf
source "$HOME/.config/mini-conf/activate-fedora43.sh"
```

再从这个 shell 启动你的 IDE、终端会话或本地 agent。

## 16. 本地 agent 行为约定

下面这些是给本地 agent 的明确行为规则：

1. 保持 Default permissions，不主动请求更宽泛的本地目录写权限。
2. 共享下载缓存、工具缓存、浏览器缓存优先落到 `/var/cache/codex/shared`，不要在仓库根下新建大体积缓存目录。
3. 项目相关构建产物优先落到 `/var/cache/codex/build/mini-conf`，默认不在仓库里生成 `target/`。
4. Rust 相关工具必须继承 `CARGO_HOME`、`RUSTUP_HOME`、`CARGO_TARGET_DIR`，形成“共享依赖缓存 + 项目隔离构建目录”的结构。
5. 如果 `sccache` 已安装，Rust 编译默认继承 `RUSTC_WRAPPER=sccache` 与 `SCCACHE_DIR`，共享的是编译缓存而不是 `target/`。
6. Node / Corepack / pnpm / Playwright 必须优先使用仓库外共享缓存，不要在仓库里生成 `.pnpm-store/`、`.cache/`、Playwright 浏览器缓存。
7. 如果某个工具只能把大缓存写进仓库，先停止并提示，不要直接执行。
8. 小体积、必要的工作树内容可以存在仓库内，例如 `node_modules/`、真实源码文件、测试快照、迁移文件。
9. 不要自行创建项目内的“临时构建目录”作为旁路方案，例如 `.tmp-build/`、`build-cache/`、`scratch/` 之类目录。
10. 默认优先使用 `just`、仓库已有脚本和系统共享缓存，不要发明新的本地构建目录约定。

## 17. 完成标准

执行完本文后，最少应满足：

- `rustc --version`
- `sccache --version`
- `cargo nextest --version`
- `cargo llvm-cov --version`
- `sqlx --version`
- `node --version`
- `pnpm --version`
- `psql --version`
- `just --version`
- `PGPASSWORD="$(secret-tool lookup service mini-conf env dev role app-db user mini_conf)" psql -h 127.0.0.1 -p 5432 -U mini_conf -d mini_conf -c "select current_database(), current_user;"`
- `pnpm format:check`
- `just bootstrap-dev`

如果这些都正常，说明 Fedora 43 的 `mini-conf` 开发底座已经够用了。

## 18. 参考来源

仓库内文档：

- `README.md`
- `KICKOFF.md`
- `docs/BOOTSTRAP.md`
- `docs/DEV_LINUX_WSL2.md`
- `docs/REPO_INIT_CHECKLIST.md`
- `docs/FRONTEND_WORKSPACE.md`

外部资料：

- Fedora Developer Portal: Rust installation
  https://developer.fedoraproject.org/tech/languages/rust/rust-installation.html
- Fedora Developer Portal: Node.js
  https://developer.fedoraproject.org/tech/languages/nodejs/nodejs.html
- Fedora Developer Portal: PostgreSQL
  https://developer.fedoraproject.org/tech/database/postgresql/about.html
- Fedora Packages: `pnpm`
  https://packages.fedoraproject.org/pkgs/nodejs-pnpm/pnpm/
- Fedora Packages: `just`
  https://packages.fedoraproject.org/pkgs/rust-just/just/
- Fedora Packages: `sccache`
  https://packages.fedoraproject.org/pkgs/rust-sccache/sccache/
- Fedora Packages: `postgresql16-server`
  https://packages.fedoraproject.org/pkgs/postgresql16/postgresql-server/
- Fedora Packages: `moby-engine`
  https://packages.fedoraproject.org/pkgs/moby-engine/moby-engine/
