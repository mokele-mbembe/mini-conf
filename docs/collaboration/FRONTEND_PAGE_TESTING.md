# 前端页面测试方式

## 1. 文档目标

这份文档用于说明 `mini-conf` 管理台前端在本地开发阶段应该怎么启动、怎么验证页面、以及出现“空白页 / 接口不通 / 登录态异常”时优先怎么排查。

它不定义业务语义本身，业务真值仍然以这些文档为准：

- `docs/collaboration/FRONTEND_HANDOFF.md`
- `docs/collaboration/FRONTEND_IMPLEMENTATION_PLAN.md`
- `docs/constraints/FRONTEND_MVP_BLUEPRINT.md`
- `docs/constraints/product-qa/*`
- `docs/constraints/ADMIN_API.md`

## 2. 推荐测试顺序

前端页面联调不要一上来只看浏览器。

建议固定按这个顺序排查：

1. 先确认本机 runtime DB 可用
2. 再确认后端服务真的监听成功
3. 再确认前端 dev server 已启动
4. 先用 `curl` 验证关键 API
5. 最后再看浏览器页面

这样能避免把“后端没起来”或“代理端口错了”误判成 Vue 页面本身问题。

## 3. 本地启动命令

推荐顺序：

```bash
just dev-db-prepare-local
just run-server-local
just dev-web
```

含义：

- `just dev-db-prepare-local`
  - 准备 runtime DB
  - 执行迁移
  - 写入 demo 数据
- `just run-server-local`
  - 启动 Rust 后端
- `just dev-web`
  - 启动 Vite 前端开发服务

## 4. 启动后应检查的地址

当前本地联调至少应检查：

- 前端页面入口
  - `http://127.0.0.1:5173/`
- 后端健康或接口入口
  - `http://127.0.0.1:<server-port>/api/auth/me`

注意：

- 不要先假设后端一定监听 `3000`
- 应以服务实际监听端口为准
- 前端 Vite proxy 必须和后端真实监听端口一致

## 5. 先用 API 验证链路

在浏览器看页面前，先验证这几条最小链路。

### 5.1 未登录探测

```bash
curl --noproxy '*' -i http://127.0.0.1:<server-port>/api/auth/me
```

预期：

- 未登录时应返回 `401`
- body 应是统一错误格式

### 5.2 登录

```bash
curl --noproxy '*' -i \
  -H 'Content-Type: application/json' \
  -d '{"username":"admin","password":"admin123456"}' \
  http://127.0.0.1:<server-port>/api/auth/login
```

预期：

- 返回 `200`
- 设置 session cookie

### 5.3 登录后拉项目列表

可以带 cookie jar 测：

```bash
tmpdir=$(mktemp -d)
cookiejar="$tmpdir/cookies.txt"

curl --noproxy '*' -sS -c "$cookiejar" \
  -H 'Content-Type: application/json' \
  -d '{"username":"admin","password":"admin123456"}' \
  http://127.0.0.1:<server-port>/api/auth/login

curl --noproxy '*' -sS -b "$cookiejar" \
  http://127.0.0.1:<server-port>/api/projects

rm -rf "$tmpdir"
```

预期：

- 返回 `{ "items": [...] }`
- 能看到 demo 项目列表

如果这一步不通，不要继续把问题归因到前端页面。

## 6. 浏览器页面测试方式

当 API 验证没问题后，再用浏览器验证页面。

建议至少检查这些路径：

- `/`
- `/login`
- `/projects`
- `/projects/:projectId`

建议按下面顺序观察：

1. 打开 `http://127.0.0.1:5173/`
2. 看是否跳到 `/login` 或 `/projects`
3. 登录后是否能进入项目列表
4. 点击项目卡片是否能进入项目详情骨架页
5. 项目详情页是否能看到：
   - 项目基础信息
   - 状态 badge
   - 8 个 tab 占位

## 7. 空白页排查顺序

如果浏览器只看到空白页，按下面顺序查。

### 7.1 先看 HTML 是否返回

```bash
curl --noproxy '*' -i http://127.0.0.1:5173/
```

如果连 HTML 都拿不到，先排查 Vite 是否启动。

### 7.2 再看前端代理的 API 是否通

```bash
curl --noproxy '*' -i http://127.0.0.1:5173/api/auth/me
```

如果这里失败，但直接请求后端接口是通的，优先怀疑：

- `vite.config.ts` 代理端口写错
- 后端实际监听地址与代理目标不一致

### 7.3 再看浏览器控制台

重点看：

- JS runtime error
- `Failed to fetch`
- `ECONNREFUSED`
- `500` / `502` / `504`
- 资源加载失败

如果是白屏但控制台报错，优先先修 runtime error，不要只看样式。

### 7.4 再看路由守卫逻辑

如果 `/api/auth/me` 失败后页面跳转异常，要确认：

- `401` 是否只表示未登录
- `5xx` / 网络错误 是否被错误地当成未登录
- 登录页是否能显示系统错误而不是死循环跳转

## 8. 页面验收时至少要覆盖的状态

### 登录页

- 已登录
- 未登录
- 登录失败
- session 探测失败（非 401）

### 项目列表页

- loading
- empty
- error
- success

### 项目详情骨架页

- loading
- not-found
- forbidden
- error
- success

并且要明确区分：

- `404` 是资源未命中语义
- `403 project_permission_denied` 是权限不足语义

## 9. 当前已确认过的联调问题

以下问题已经在本地联调中出现过，后续前端改动要优先防止回归。

### 9.1 Vite proxy 目标端口和后端实际监听端口不一致

已观察到：

- 前端 dev server 在 `5173`
- 后端实际监听端口不是先验假设的 `3000`
- 当 `vite.config.ts` 把 `/api` 代理到错误端口时：
  - 页面 HTML 可以打开
  - 但所有接口请求失败
  - 登录页可能出现系统错误
  - 页面可能表现成白屏、空白内容或无法进入主路径

### 9.2 `format:check` 可能把 `dist/` 也扫进去

如果先执行 `build`，再执行：

```bash
pnpm --dir apps/web run format:check
```

可能因为 `dist/` 中的产物未格式化而失败。

这类问题不影响页面渲染，但会影响提交前校验。

## 10. 推荐的提交前最小检查

提交前至少跑：

```bash
pnpm --dir apps/web typecheck
pnpm --dir apps/web build
pnpm --dir apps/web run format:check
pnpm --dir apps/web run test:e2e
```

如果是联调相关修改，再额外做：

```bash
curl --noproxy '*' -i http://127.0.0.1:5173/api/auth/me
curl --noproxy '*' -i http://127.0.0.1:<server-port>/api/auth/me
```

目标是同时确认：

- 前端开发服务器能返回页面
- 前端代理能打到后端
- 后端接口本身能返回正确语义
