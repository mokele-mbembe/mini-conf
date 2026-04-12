# 质量检查与测试收口计划

## 1. 文档目的

这份文档用于回答三件事：

- 当前已经启用哪些本地检查和 CI 检查
- 哪些检查是“先占位、后收紧”的
- 什么时候值得补更重的“带真实数据、带 HTTP 响应断言”的集成测试

目标不是把检查堆满，而是在项目阶段合适的时候引入合适的约束。

工作流分层规范见 [标准 Linux 开发工作流](../agents/STANDARD_WORKFLOW.md)；本文只讨论质量门槛收口节奏。

## 2. 当前已启用的检查

当前工作流分四层：

- `Core`：`just lint`、`just test`、`just openapi-check`
- `Isolated DB`：`just db-migrate-up`、`just test-backend-db`
- `Alpha HTTP`：基于真实 TCP 端口、真实 PostgreSQL 和 Hurl 的后端黑盒验证
- `Blackbox / Staging`：长期共享环境上的 HTTP 黑盒验证与未来前端 E2E
- `Production`：显式迁移、启动、健康检查与回滚流程

CI 对应关系：

- `quality` job 对应 `Core`
- `backend-db` job 对应 `Isolated DB`
- `alpha-smoke` job 对应 PR 级 `Alpha HTTP`
- `alpha-full` job 对应 `main` 合入后的全量 `Alpha HTTP`

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

### 2.3 Alpha HTTP 黑盒测试

- `just alpha-smoke`
- `just alpha-full`

引入时机：

- 后端接口已经足够形成管理端与开放接口的主链路
- 前端页面尚未进入主开发阶段

当前状态：

- 这层测试通过真实端口启动 `server`
- 使用 `Hurl` 保存请求与断言文本
- PR 上先跑 `alpha-smoke`
- 合入 `main` 后再跑 `alpha-full`

特别说明：

- 这层测试和当前 Rust in-process 集成测试互补，不替代后者
- 端口默认使用 `127.0.0.1:18080`，不占用文档示例里的 `8080`

后续随新接口开发时，建议按下面的规则补：

- 每新增一个对外 HTTP 接口，至少同步补一条 OpenAPI 导出校验和一条 Rust 侧路由/handler 测试
- 每新增一个会改数据库状态的管理端接口，优先补 `Isolated DB` 测试，确认状态码、响应体和最终数据库状态
- 每新增一个会进入真实业务主链路的接口，再决定是否补进 `Alpha HTTP`
- `Alpha HTTP` 不追求覆盖每个错误分支，它主要负责验证“真实进程 + 真实端口 + 真实 cookie/bearer + 真实 PostgreSQL”下的关键闭环

把新接口补进哪一层，优先按这个判断：

- 只涉及请求体验证、错误码映射、简单 JSON 返回：
  留在 `just test`
- 涉及 PostgreSQL 读写、状态变更、发布物生成、权限控制：
  补到 `just test-backend-db`
- 涉及真实登录态、cookie、bearer token、跨多个接口串起来的管理端/开放接口流程：
  补到 `just alpha-smoke` 或 `just alpha-full`

`alpha-smoke` 只收这些接口或流程：

- `healthz`
- 登录
- 一个最小管理端创建闭环
- 一个最小开放接口消费闭环
- 能挡住“服务起不来、鉴权断了、主对象创建断了、开放读取断了”的最短路径

`alpha-full` 适合持续追加这些接口或流程：

- 管理端新增的 list/detail/create/update 主路径
- 同一资源的状态迁移或发布链路
- 开放接口新增的 resolve/fetch/report 类路径
- 需要先登录、再写 draft、再 publish、再 open consume 的跨接口闭环
- 重要的鉴权失败路径与资源不存在路径

不建议进入 `alpha-smoke`，但适合进 `alpha-full` 或 `test-backend-db` 的内容：

- 同一资源的筛选、排序、分页
- 重复键、版本冲突、模板限制、必填约束这类业务错误分支
- 回归价值高但请求数偏多的管理端 CRUD 细节

当前阶段补新接口时，建议默认按这个最小配套：

1. 更新 handler、schema、OpenAPI
2. 补一条 Rust 路由层失败/成功测试
3. 如果接口读写数据库，再补一条 `test-backend-db`
4. 如果接口进入主链路，再把它接到已有 Hurl 流程；只有在无法自然接入时，才新建 Hurl 文件

当前最适合后续继续往 `alpha-full` 追加的方向：

- `project_members`
- `audit_logs`
- 更完整的 `deployment-instances` clone/template 路径
- `drafts` 的版本冲突与 clone 路径
- `releases` 的 diff、republish、required config 约束
- 开放接口的 `not found`、`forbidden`、`not modified` 路径

### 2.4 OpenAPI 检查

- `just openapi-check`

引入时机：

- `utoipa`、`/api/openapi.json`、`/swagger-ui` 和导出脚本落地后启用

当前状态：

- 已经是强制检查
- 任何 handler/schema 变更，只要影响 OpenAPI，都必须同步更新 `docs/artifacts/openapi.json`

特别说明：

- 这条检查不是语义比对，而是导出后检查生成文件是否有 git 变更
- 所以只要改了接口定义，就必须重新导出并把产物一起提交

### 2.5 SQLx 检查

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

### 2.6 性能烟雾检查

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

当前前端已经进入真实开发阶段，但仍处在 scaffold + 第一批页面阶段。

当前已经启用：

- frontend lint
- frontend format check
- frontend typecheck
- frontend build
- 最小 Playwright smoke E2E

当前仍然保持占位或暂缓引入：

- Vitest 单元测试
- 更完整的页面级 E2E 套件
- 多浏览器矩阵
- 截图回归

下一步值得正式收紧的时机：

- Draft 编辑页和预览页进入真实主链路
- 发布确认、Release 历史、Diff 开始落地
- 项目成员、配置文件、部署实例列表页全部进入稳定状态

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

## 5. 新接口开发时的补测规范

后续新增接口时，优先不要问“要不要补测试”，而是问“补到哪一层最值”。

建议按接口类型收口：

- 新增管理端读接口：
  至少补 `200` 成功路径和 `401` 未登录路径；如果有 `404`，补在 `test-backend-db`
- 新增管理端写接口：
  至少补成功写入、资源不存在、关键约束失败；如果它属于主链路，再补进 `alpha-full`
- 新增开放接口：
  至少补 bearer 成功路径和无 token/错 token 路径；如果它是客户端真实拉取或上报入口，再补进 `alpha-full`
- 新增列表接口：
  不默认进 Hurl；除非它是管理端首页或客户端启动第一跳必须依赖的列表
- 新增纯辅助接口：
  优先停留在 `just test`，不要机械追加到黑盒层

建议每个新接口至少满足这几个检查点中的合适子集：

- 成功状态码
- 未认证或未授权状态码
- 资源不存在状态码
- 响应体关键字段
- 对数据库的最终写入结果
- 对后续链路是否可继续消费

Hurl 文件组织建议保持克制：

- 优先扩已有流程文件，不要每个接口都新建一个 `.hurl`
- 只有当新接口代表一条独立业务故事时，才单独建文件
- `smoke` 文件保持极短，避免为了多覆盖而把 PR 反馈时间拉长
- `full` 文件允许更长，但仍应围绕“一个业务故事一个文件”

## 6. 建议的引入节点

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

### 阶段 C：OpenAPI、Alpha HTTP 与管理端期

- `openapi-check`
- `alpha-smoke`
- `alpha-full`
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

## 7. 当前推荐执行顺序

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
