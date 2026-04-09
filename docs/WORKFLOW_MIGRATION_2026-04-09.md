# 2026-04-09 工作流迁移说明

## 1. 今晚先看这个

如果你今晚只想最快把 Fedora 43 桌面机跟上，先执行下面这套：

1. 拉取今天的改动
2. 打开 `~/.config/mini-conf/dev-env.sh`
3. 把数据库相关变量整理成下面这种最小形态：

```bash
export MINI_CONF_LOCAL_TEST_DB_HOST=127.0.0.1
export MINI_CONF_LOCAL_TEST_DB_PORT=5432
export MINI_CONF_LOCAL_TEST_DB_NAME=mini_conf
export MINI_CONF_LOCAL_TEST_DB_USER=mini_conf
export MINI_CONF_LOCAL_TEST_SECRET_ENV=dev
```

4. 重新加载 shell 环境：

```bash
source ~/.config/mini-conf/dev-env.sh
```

5. 验收：

```bash
just lint
just test
just test-backend-db-local
```

6. 如果之后确实需要临时启动后端联调，再额外补运行库变量并使用：

```bash
just run-server-local
```

今晚阶段不要做的事：

- 不要为了这次迁移立刻再建第二个本机库
- 不要今晚就把 staging / blackbox / production 库也一起补齐
- 不要再用“portable 命令自动读取本机脚本”的旧理解

## 2. 文档目的

这份文档用于解释 2026-04-09 这次工作流调整到底改了什么，以及你今晚回到 Fedora 43 桌面机后，应该怎样最快适配。

它不是新的长期规范文档。长期规范仍以 [标准 Linux 开发与部署工作流](./STANDARD_WORKFLOW.md) 为准。

这份说明只回答 4 件事：

- 今天为什么要改
- 今天具体改了哪些契约
- 当前机器和 Fedora 43 桌面机今晚应该怎么配
- 当前测试处于哪个阶段，哪些长期环境现在还没进入

## 3. 今天为什么要改

这次调整的核心原因是：

- 之前的本机脚本把“开发机便利配置”和“长期可移植的运行 / 迁移 / 测试契约”混在了一起
- `TEST_DATABASE_URL` 之前会在脚本里被隐式补成 `DATABASE_URL`
- 旧命名 `MINI_CONF_DB_*` 更像“唯一正式数据库配置”，但后续你已经明确需要：
  - 同一 PostgreSQL server 承载多套不同场景的 database
  - staging / blackbox / production 使用自定义数据库名
  - 本机开发机配置不要反过来约束长期部署设计

所以这次不是在否定当前开发工作流，而是在把它整理成：

- 当前开发阶段仍然高效
- 后续前端联调、黑盒环境和生产部署也不会被今天的本机脚本绑死

## 4. 今天改了什么

### 3.1 运行时环境枚举扩展

现在 `APP_ENV` 支持：

- `dev`
- `test`
- `staging`
- `prod`

其中：

- `staging` 现在是正式一等环境，不再被当作非法值
- `staging` 和 `prod` 不允许 `INIT_DB_ON_BOOT=true`

这表示：

- 开发 / 测试环境可以接受启动时自动迁移
- 长期黑盒环境和生产环境必须走显式迁移步骤

### 3.2 命令入口分成 portable 和 local wrapper

portable 命令：

- `just db-migrate-up`
- `just db-migrate-down`
- `just test-backend-db`
- `just run-server`

这些命令只读取当前进程里的显式环境变量，不会自动加载 `~/.config/mini-conf/dev-env.sh`。

local wrapper：

- `just db-migrate-up-local`
- `just db-migrate-down-local`
- `just test-backend-db-local`
- `just run-server-local`

这些命令会先读取 `~/.config/mini-conf/dev-env.sh`，再解析本机开发便利变量。

兼容别名：

- `just dev-server` 仍可用，但现在等价于 `just run-server-local`

### 3.3 新增本机 DSN 解析脚本

新增：

- [`scripts/local-db-env.sh`](../scripts/local-db-env.sh)

现在它负责本机 local wrapper 的数据库变量解析。

旧脚本：

- [`scripts/dev-db-env.sh`](../scripts/dev-db-env.sh)

现在只保留为兼容壳，避免旧习惯立刻失效。

### 3.4 测试库不再隐式继承运行库

之前的脚本行为是：

- 如果 `TEST_DATABASE_URL` 为空，就自动把它补成 `DATABASE_URL`

现在不再这样做。

新的规则是：

- portable 命令必须显式提供 `TEST_DATABASE_URL`
- local wrapper 只有在你显式设置 `MINI_CONF_LOCAL_TEST_USE_RUNTIME_DB=true` 时，才允许测试库复用运行库

这条改动的目的很直接：

- 避免未来有人把 `DATABASE_URL` 指向 staging 或生产式环境后，又无意中把测试打到那个库上

### 3.5 数据库命名不再绑定产品名

新的正式设计是：

- 默认采用 `database-per-instance`
- 同一个 PostgreSQL server 可以承载多套 `mini-conf` 的不同 database
- 数据库名由部署者按场景命名

例如：

- `mini_conf`
- `mini_conf_dev`
- `mini_conf_ci`
- `mini_conf_staging`
- `mini_conf_prod_candidate`
- `mini_conf_prod`

这意味着：

- `mini-conf` 不再假设运行库一定叫某个固定名字
- 你可以按阶段、场景和容量独立建库

## 5. 今晚回到 Fedora 43 桌面机后的最快适配方案

### 4.1 当前阶段推荐策略

对你当前这个项目阶段，最实际、最省事、也足够合理的做法是：

- 两台 Fedora 机器先继续保留各自已经存在的本机测试库
- 当前开发阶段优先恢复本机 DB 集成测试，不把本机 runtime 当作今晚必须恢复的链路
- 等后端主路径稳定、开始前端联调或共享黑盒环境时，再单独补运行库 / staging 库

这不是最终长期生产形态，但对“今晚快速恢复统一工作流”是最合适的折中。

原因：

- 当前现有测试主要还是开发阶段自测
- 后端路由、SQL、迁移、OpenAPI 和真实 PostgreSQL 行为仍然是当前最重要的验证对象
- 当前 Rust 集成测试不依赖真实监听端口，所以没必要先为了 runtime 把本机联调链路补齐
- 现在就强行把每台机器拆成运行库 + 测试库两套，会增加适配成本，但短期收益有限

### 5.2 今晚建议的本机变量写法

如果你要最快适配，建议两台 Fedora 的 `~/.config/mini-conf/dev-env.sh` 都先采用下面这组思路：

```bash
export MINI_CONF_LOCAL_TEST_DB_HOST=127.0.0.1
export MINI_CONF_LOCAL_TEST_DB_PORT=5432
export MINI_CONF_LOCAL_TEST_DB_NAME=<existing-local-mini-conf-db-name>
export MINI_CONF_LOCAL_TEST_DB_USER=mini_conf
export MINI_CONF_LOCAL_TEST_SECRET_ENV=dev
```

说明：

- `<existing-local-mini-conf-db-name>` 指你当前机器已经建好的那个本机库名
- 如果你现在本机实际库名就是 `mini_conf`，那就直接写 `mini_conf`
- 当前阶段不必今晚就再新建 `mini_conf_dev`
- 这组配置的含义是：
  - 当前机器先只恢复 local test
  - 密码继续走 `secret-tool`
  - runtime 变量等确实需要手工联调时再单独补

### 5.3 今晚建议你在 Fedora 43 桌面机上这样验收

先加载环境：

```bash
source ~/.config/mini-conf/dev-env.sh
```

然后按这个顺序跑：

```bash
just lint
just test
just test-backend-db-local
```

如果你之后要临时启动后端联调：

```bash
just run-server-local
```

## 6. 当前机器、桌面机和 GitHub Actions 的统一工作模式

### 5.1 当前机器

建议：

- 继续保留现有本机 `mini-conf` 同名测试库
- 当前阶段先只配置 `MINI_CONF_LOCAL_TEST_DB_*`
- 本机 DB 工作流优先使用 `just test-backend-db-local`

即：

- `just test-backend-db-local`

### 5.2 Fedora 43 桌面机

建议完全跟当前机器保持同样口径：

- 继续使用它本机现有的同名测试库
- 也只先设置 `MINI_CONF_LOCAL_TEST_DB_*`
- 也先用 local wrapper 恢复数据库测试链路

这样今晚最省事：

- 不需要先新建第二个库
- 不需要先改数据库角色设计
- 不需要先引入 staging / blackbox 的额外复杂度

### 5.3 GitHub Actions

CI 仍建议保持现在这类模式：

- 使用一个明确的 CI 数据库
- `DATABASE_URL` 和 `TEST_DATABASE_URL` 在 CI job 里显式给出
- 测试内部继续使用隔离 schema

也就是说：

- CI 继续是一个 PostgreSQL service
- job 内一套显式 DSN
- 每个测试再做 schema 隔离

这和当前仓库测试实现最匹配，也最稳定。

## 7. 当前测试属于哪一层

到今天为止，仓库里的这些测试本质上都还属于开发阶段自测：

- 单元测试
- 后端 HTTP 集成测试
- 基于真实 PostgreSQL 的 DB 集成测试
- OpenAPI 一致性检查

它们主要回答的是：

- 路由是否正确
- handler / service / SQL 是否符合当前后端设计
- migration 是否可执行
- 管理端和开放消费端主路径在开发阶段是否自洽

它们还不属于完整的产品级环境验证。

## 8. alpha / beta / gamma 现在有没有进入

严格说，现在还没有真正进入产品级的：

- alpha
- beta
- gamma

原因不是“测试不够真”，而是测试目标还没切到那一层。

当前仓库还没有系统性覆盖这些产品级目标：

- 长期存在的共享黑盒环境
- 前端 + 后端 + 数据库的完整用户面流程
- 多版本升级验证
- staging 数据准备与回归
- 候选发布库或生产候选环境的迁移 / 启动 / 健康检查

所以现在更准确的描述是：

- 已有测试 = 开发阶段自测 + PR 级后端集成验证
- 还不是产品级 alpha / beta / gamma 验证

补充一个和多进程共存相关的事实：

- 当前 Rust 后端集成测试主要通过 in-process router `oneshot(...)` 执行
- 它们不依赖 `HTTP_ADDR`
- 它们不依赖固定 `8080`
- 所以本机 `8080` 被占不会导致现有 Rust 集成测试失败

## 9. 是否可以等后端开发完成再加运行库

可以。

对你当前阶段，我的建议就是：

- 现在先不要为了“未来长期部署”今晚就额外建运行库
- 先把两台 Fedora 机器统一到同一套开发阶段工作流
- 等后端主路径完成、开始前端联调或共享黑盒测试时，再新增这些库：
  - `mini_conf_staging`
  - `mini_conf_blackbox`
  - `mini_conf_prod_candidate`
  - `mini_conf_prod`

什么时候值得新增独立运行库：

- 开始做前端联调
- 开始做共享黑盒环境
- 需要验证候选版本升级
- 需要保留一套长期运行的 staging 数据

那时再把“开发自测库”和“长期运行库”分开，收益才明显。

## 10. 今晚最重要的结论

- 今天这次迁移不是要求你今晚立刻建更多库
- 今天真正要做的是把契约理顺：
  - portable 命令只认显式 DSN
  - local wrapper 才读本机开发便利变量
  - 当前阶段本机先恢复测试库即可
- 对当前机器和 Fedora 43 桌面机，今晚最合理的做法是：
  - 继续使用各自已经存在的本机 `mini-conf` 测试库
  - 只先设置 `MINI_CONF_LOCAL_TEST_DB_*`
  - 用 `just test-backend-db-local` 恢复开发期 DB 验证
- 如果以后需要同机并存多个后端进程：
  - 为每个进程显式设置不同 `HTTP_ADDR`
  - 不再把 `8080` 当成唯一联调端口
- 产品级 alpha / beta / gamma 还没真正开始；那一层可以等后端主路径完成后，再新增长期运行库和黑盒环境
