# Alpha HTTP Tests

这套测试用于在前端页面动工前，先把后端 HTTP 主链路作为一层独立的 alpha 黑盒验证沉淀在仓库里。

特点：

- 真实启动 `server` 进程
- 通过真实 TCP 端口访问 HTTP 接口
- 使用真实 PostgreSQL
- 使用 `Hurl` 保存可审查的请求/断言文本

当前分两层：

- `smoke`：PR 级最小闭环
- `full`：合入 `main` 后的更完整管理端 + 开放接口闭环

本地运行：

```bash
just alpha-smoke
just alpha-full
```

当前开发机如果只配置了本机测试库，也可以用：

```bash
just alpha-smoke-local
just alpha-full-local
```

当前实现约定：

- 默认端口使用 `127.0.0.1:18080`
- `DATABASE_URL` 是必需项，且只作为 alpha 测试 base DB
- `alpha-*-local` 强制从 `TEST_DATABASE_URL` 派生临时 schema，不写入联调 runtime 库
- suite 结束后会 `DROP SCHEMA ... CASCADE`；异常中断后的残留 schema 用 `just db-clean-test-schemas-local` 清理
- 所有请求统一通过 `{{base_url}}` 变量构造，避免写死 `8080`
