# 质量检查与测试收口计划

## 1. 文档目的

这份文档用于回答三件事：

- 当前已经启用哪些本地检查和 CI 检查
- 哪些检查是“先占位、后收紧”的
- 什么时候值得补更重的“带真实数据、带 HTTP 响应断言”的集成测试

目标不是把检查堆满，而是在项目阶段合适的时候引入合适的约束。

工作流分层规范见 [docs/STANDARD_WORKFLOW.md](./STANDARD_WORKFLOW.md)；本文只讨论质量门槛收口节奏。

## 2. 当前已启用的检查

当前工作流分四层：

- `Core`：`just lint`、`just test`、`just openapi-check`
- `Isolated DB`：`just db-migrate-up`、`just test-backend-db`
- `Blackbox / Staging`：长期共享环境上的 HTTP 黑盒验证与未来前端 E2E
- `Production`：显式迁移、启动、健康检查与回滚流程

CI 对应关系：

- `quality` job 对应 `Core`
- `backend-db` job 对应 `Isolated DB`

本地与 CI 当前共同依赖这些入口：

- `just lint`
- `just sqlx-check`
- `just openapi-check`
- `just test`
- `just perf-smoke`

其中实际对应的检查内容如下。

### 2.1 后端静态检查

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`

引入时机：

- Rust workspace 可编译后立即启用

当前状态：

- 已经是稳定基线
- 应长期保持为强制检查

### 2.2 后端测试

- `cargo nextest run --workspace`
- `just test-backend-db`

引入时机：

- 健康检查接口落地后先启用基础测试
- PostgreSQL 和 migration 接入后，再补数据库集成测试

当前状态：

- 已经是稳定基线
- Open API、管理端 CRUD、Draft、Release、Preview 这些主路径都已经开始用真实 PostgreSQL 测
- 这层测试属于 `Isolated DB`，不是共享黑盒环境测试

### 2.3 OpenAPI 检查

- `just openapi-check`

引入时机：

- `utoipa`、`/api/openapi.json`、`/swagger-ui` 和导出脚本落地后启用

当前状态：

- 已经是强制检查
- 任何 handler/schema 变更，只要影响 OpenAPI，都必须同步更新 `docs/openapi/openapi.json`

特别说明：

- 这条检查不是语义比对，而是导出后检查生成文件是否有 git 变更
- 所以只要改了接口定义，就必须重新导出并把产物一起提交

### 2.4 SQLx 检查

- `just sqlx-check`

引入时机：

- 当前仓库开始接入 SQLx 后就预留了入口

当前状态：

- 现在是“条件启用”
- 只有当仓库里出现 `.sqlx/` 离线元数据，或开始使用 `sqlx::query!` / `query_as!` 这类 compile-time 宏时，才真正执行 `cargo sqlx prepare --check --workspace`
- 当前项目大部分查询还是运行时 API，因此这条检查暂时允许跳过

恢复为强制检查的时机：

1. 开始系统性改用 compile-time checked SQLx 查询宏
2. 提交 `.sqlx/` 元数据目录
3. 希望把复杂 SQL 的字段类型错误前移到编译期

### 2.5 性能烟雾检查

- `just perf-smoke`

引入时机：

- Open API 主路径可运行后启用

当前状态：

- 先作为最小性能回归哨兵
- 后续真实压测脚本完善后，再提高约束力度

## 3. 未来仍要补的检查

这些检查已经有计划，但当前还不适合强制。

### 3.1 覆盖率

计划入口：

- `cargo llvm-cov --workspace --lcov --output-path target/lcov.info`

建议正式引入时机：

- `project_members`
- `release diff`
- `audit_logs`
- `token reset`

这些模块落地后，分支和权限路径会明显增多，覆盖率约束才更有价值。

### 3.2 前端 lint / typecheck / test / e2e

当前前端还没进入真实开发阶段，因此 CI 虽然有 pnpm 安装入口，但还不适合把完整前端质量门槛拉满。

建议正式引入时机：

- `apps/web` 开始有真实页面和组件
- 登录页、项目页、Draft 编辑页、预览页至少有一条真实主链路

### 3.3 强制 SQLx prepare

这条检查不应该为了“有检查”而提前开启。

建议正式引入时机：

- 复杂查询开始收口到 compile-time SQLx 宏
- 团队决定将 `.sqlx` 作为正式产物纳入版本控制

## 4. 什么时候补“带数据 + 带 HTTP 结果校验”的测试更合适

这里说的测试，指的是：

- 用真实 PostgreSQL 或隔离 schema seed 数据
- 通过真实 HTTP 路由发送请求
- 同时校验返回体、状态码和最终数据库状态

当前已经值得做、并且部分已经在做的模块：

- Open API 主路径
- `projects` CRUD
- `config-files` CRUD
- `deployment-instances` CRUD / clone / preview
- `POST /api/deployment-instances/:id/token/reset`
- `drafts`
- `releases/publish`
- `GET /api/releases/:id/diff`

接下来最适合继续补这类测试的模块：

1. `project_members`
2. `audit_logs`
3. 管理端鉴权失败路径和 cookie/session 边界

原因：

- 这些模块都带明显的权限、状态转换或对外契约
- 单纯单测不足以覆盖真实行为
- 它们又还没复杂到需要前端 E2E 才能验证

当前不值得优先补重型 HTTP+DB 测试的场景：

- 纯配置解析
- 纯 schema/model 映射
- 简单错误码常量

这些更适合留在单测层。

## 5. 建议的引入节点

可以按下面的阶段收口质量门槛。

### 阶段 A：后端骨架期

- `fmt`
- `clippy`
- `nextest`
- 最小 healthz 集成测试

### 阶段 B：数据库主链路期

- 数据库集成测试
- `test-backend-db`
- Open API 主路径 HTTP 契约测试

### 阶段 C：OpenAPI 与管理端期

- `openapi-check`
- 管理端 CRUD 闭环集成测试
- Preview / clone / publish 规则测试

### 阶段 D：权限与复杂查询期

- `project_members`
- `audit_logs`
- `release diff`
- 视情况恢复强制 `sqlx-check`
- 开始要求覆盖率报告

### 阶段 E：前端主链路期

- `pnpm lint`
- `pnpm typecheck`
- 关键页面测试
- 管理端主链路 E2E

## 6. 当前推荐执行顺序

本地在提交前，优先跑：

1. `just lint`
2. `just test`
3. `just openapi-check`
4. 涉及数据库主路径时再跑 `just db-migrate-up`
5. 涉及数据库主路径时再跑 `just test-backend-db`

在本地要模拟 CI 时，跑：

```bash
just ci-local
```

如果这一步失败，先确认失败原因属于哪一类：

- 代码质量失败
- OpenAPI 产物未同步
- 数据库环境未准备
- 前端依赖未准备

不要把所有失败都归因于 CI runner。
