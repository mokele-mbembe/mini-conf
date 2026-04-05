# Linux / WSL2 开发与部署草案

## 1. 文档目的

这份文档用于明确 `mini-conf` 的真实开发与运行环境约束。

当前仓库虽然位于 Windows 文件系统中，但现阶段 Windows 仅用于：

- 文档设计
- 产品规划
- 方案沉淀

真正的编码、调试、联调、测试、CI 和部署应以 Linux 或 WSL2 为准。

## 2. 为什么采用 Linux / WSL2 优先

原因主要有这些：

- 生产环境未来运行在 Linux
- Rust、PostgreSQL、Node 工具链在 Linux 下更贴近目标环境
- shell 脚本、CI 和容器工作流更容易与生产一致
- 可以减少 Windows 专属路径、权限、换行符和脚本兼容问题

## 3. 开发环境原则

- 以 Linux 路径和 shell 命令为主
- 脚本优先使用 `bash` 和 `just`
- PowerShell 只作为补充，不作为主入口
- 所有本地检查命令都要能在 Linux / WSL2 中直接执行
- CI 以 Linux runner 为基准

## 4. 推荐开发方式

推荐优先级：

1. 独立 Linux 主机
2. 本机 WSL2 Ubuntu 实例
3. Windows 仅做文档和非运行态编辑

如果使用 WSL2，建议：

- 把真正要执行构建和测试的仓库放在 WSL2 Linux 文件系统中
- 尽量避免直接在 `/mnt/c/...` 下进行高频编译和测试

推荐路径示例：

```bash
~/workspace/mini-conf
```

不推荐长期作为主开发目录的路径：

```bash
/mnt/c/Users/zhaoj/Projects/mini-conf
```

## 5. 建议安装的软件

后端相关：

- Rust stable toolchain
- `cargo-nextest`
- `cargo-llvm-cov`
- `sqlx-cli`
- PostgreSQL 16+

前端相关：

- Node.js 20+
- `pnpm`

通用工具：

- `git`
- `just`
- `docker`
- `docker compose`
- `ripgrep`
- `jq`
- `curl`

## 6. WSL2 初始化建议

以 Ubuntu 为例：

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  pkg-config \
  libssl-dev \
  postgresql-client \
  jq \
  curl \
  git \
  ripgrep
```

安装 Rust：

```bash
curl https://sh.rustup.rs -sSf | sh
source "$HOME/.cargo/env"
rustup default stable
rustup component add rustfmt clippy
```

安装 cargo 扩展：

```bash
cargo install cargo-nextest
cargo install cargo-llvm-cov
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

安装 Node 与 pnpm：

```bash
corepack enable
corepack prepare pnpm@latest --activate
```

## 7. 仓库约束

仓库中应遵守这些约束：

- 脚本优先放在 `scripts/*.sh`
- 命令统一收敛到 `justfile`
- 不依赖 Windows 专属批处理脚本作为唯一入口
- 文档中的示例命令优先使用 bash
- 所有配置文件统一使用 LF

建议补充 `.gitattributes`：

```gitattributes
* text=auto eol=lf
*.sh text eol=lf
*.rs text eol=lf
*.ts text eol=lf
*.vue text eol=lf
*.sql text eol=lf
```

## 8. 本地运行建议

建议开发期最小依赖：

- PostgreSQL
- 后端服务
- 前端服务

推荐方式：

- PostgreSQL 用本机服务或 Docker
- 后端在 Linux / WSL2 中运行
- 前端在 Linux / WSL2 中运行

环境变量示例：

```env
APP_ENV=dev
HTTP_ADDR=0.0.0.0:8080
DATABASE_URL=postgres://mini_conf:secret@127.0.0.1:5432/mini_conf
DATABASE_ADMIN_URL=postgres://postgres:secret@127.0.0.1:5432/postgres
INIT_DB_ON_BOOT=true
INIT_ADMIN_USERNAME=admin
INIT_ADMIN_PASSWORD=admin123456
STATIC_DIR=apps/web/dist
```

## 9. 推荐命令入口

建议统一通过 `just` 暴露这些命令：

- `just bootstrap-dev`
- `just dev-server`
- `just dev-web`
- `just lint`
- `just test`
- `just test-e2e`
- `just ci-local`
- `just db-reset-dev`

命令职责建议：

- `bootstrap-dev` 安装并检查开发依赖
- `lint` 跑前后端静态检查
- `test` 跑前后端单测和集成测试
- `ci-local` 尽量模拟 CI 的完整检查过程

## 10. 代码质量基线

后端：

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo nextest run --workspace`
- `cargo llvm-cov --workspace`
- `cargo sqlx prepare --check`

前端：

- `pnpm lint`
- `pnpm format:check`
- `pnpm typecheck`
- `pnpm test`
- `pnpm test:e2e`

提交前建议：

- 使用 `lefthook` 或 `pre-commit`
- 至少执行格式化、lint 和快速测试

## 11. TDD 工作流建议

为了避免后续 vibe coding 导致代码质量快速下滑，建议明确采用 TDD / 测试先行：

1. 先为领域逻辑写失败测试
2. 用最小代码让测试通过
3. 再做重构
4. 对 bug 修复先写回归测试
5. 对开放消费端协议补契约测试

重点必须有测试的模块：

- Scope 匹配
- Release 发布
- 版本解析
- 鉴权
- 开放接口兼容性

## 12. CI 建议

建议使用 GitHub Actions，并以 Linux runner 为准。

最小 CI 任务建议：

- `lint-backend`
- `test-backend`
- `lint-frontend`
- `test-frontend`
- `build`

后续可补：

- 覆盖率上传
- OpenAPI 变更检查
- SQL migration 校验

## 13. 部署方向

生产部署方向建议：

- Linux 单机部署作为首选
- 使用 systemd 管理后端服务
- PostgreSQL 独立部署
- Caddy 或 Nginx 作为反向代理

首版尽量避免：

- 强依赖 Kubernetes
- 强依赖外部注册中心
- 复杂分布式依赖

## 14. 当前阶段的实际建议

结合你当前的计划，最现实的做法是：

- 继续在当前 Windows 仓库里维护产品与设计文档
- 等文档和模型稳定后，在 Linux 主机或 WSL2 中重新拉取仓库
- 再开始初始化工程、数据库迁移、测试基线和 CI

这样可以把“设计阶段”和“实现阶段”的环境分离清楚，减少后期返工。
