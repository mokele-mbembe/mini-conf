# 性能测试与观测

## 1. 当前阶段

`mini-conf` 已进入 Phase 4 性能门禁阶段。

当前目标不是给出最终容量承诺，而是建立可重复的本地、CI 与数据集矩阵入口：

- 后端运行态暴露 `/metrics`
- 后端 `/metrics` 增加业务热点 histogram
- 后端 perf smoke 使用 release server、独立 PostgreSQL schema 和 S/M/L 数据集
- 前端生产态 perf smoke 使用 `apps/web/dist` 与 Playwright 读取浏览器内 route/API timing
- 前端 bundle budget 检查构建体积回归
- CI 定时 `Perf` workflow 上传后端 S/M 数据集 smoke、前端 smoke、bundle budget、DB 慢查询报告与 Markdown 汇总

## 2. 后端运行态观测

服务暴露 Prometheus 文本格式端点：

- `GET /metrics`

当前指标：

- `mini_conf_process_uptime_seconds`
- `mini_conf_http_request_duration_ms`
- `mini_conf_business_operation_duration_ms`
- `mini_conf_business_observation`
- `mini_conf_db_pool_connected`
- `mini_conf_db_pool_connections`

HTTP duration 维度：

- `method`
- `route`：优先使用 Axum matched path，避免动态 ID、部署 key 造成高基数指标
- `status`

现阶段是进程内轻量 histogram，适合确认请求耗时分布与回归趋势。后续生产部署建议接入 Prometheus/Grafana，并补充慢查询、业务热点指标。

业务 duration 维度：

- `operation`
- `outcome`

当前业务 operation：

- `draft_save`
- `release_publish`
- `open_config_bundle`

当前业务 observation：

- `open_config_bundle` / `config_count`

## 3. 后端 Perf Smoke

入口：

- `just perf-smoke`
- `just perf-ci`

真实执行脚本：

- `scripts/run-perf-smoke.sh`

必要条件：

- `TEST_DATABASE_URL` 或 `DATABASE_URL`
- `cargo`
- `curl`
- `psql`
- `python3`

行为：

1. 编译 `server` release binary
2. 创建独立测试 schema
3. 启动 release server
4. 执行 migrations
5. seed 选定数据集
6. 测量核心接口
7. 写入 `target/perf/smoke.json`
8. 清理测试 schema

当前测量接口：

- `GET /api/healthz`
- `GET /api/open/configs/resolve`
- `GET /api/open/deployments/:deployment_key/config-bundle`
- `GET /metrics`

数据集：

| 数据集 | projects | configs/project | deployments/project | releases |
| ------ | -------: | --------------: | ------------------: | -------: |
| `S`    |        1 |               3 |                   1 |        3 |
| `M`    |       10 |              20 |                  30 |     6000 |
| `L`    |       50 |              20 |                 100 |   100000 |

选择数据集：

```bash
PERF_SMOKE_DATASET=M just perf-smoke
```

也可以覆盖矩阵规模：

```bash
PERF_SMOKE_DATASET=M \
PERF_SMOKE_PROJECTS=5 \
PERF_SMOKE_CONFIGS_PER_PROJECT=10 \
PERF_SMOKE_DEPLOYMENTS_PER_PROJECT=20 \
just perf-smoke
```

输出字段包括：

- `measured_ms`
- `threshold_ms`
- `dataset`
- `dataset_size`
- `server_rss_kb`
- 每个接口的 `min_ms`、`p50_ms`、`p95_ms`、`p99_ms`、`max_ms`
- `error_count`、`error_rate`

CI 中 `Perf` workflow 会按数据集矩阵运行：

- `S`
- `M`

CI enforcement 使用 `PERF_ENFORCE=1`，当前默认阈值仍是：

- `PERF_SMOKE_MAX_MS=250`

这个阈值仍是保护性 smoke 阈值，不是最终性能 SLO。

## 4. 前端生产态 Perf Smoke

入口：

- `just perf-web-smoke`

真实执行脚本：

- `scripts/web-perf-smoke.sh`

行为：

1. `pnpm --dir apps/web build`
2. 编译 `server` release binary
3. 创建独立测试 schema
4. 用 Rust server 服务 `apps/web/dist`
5. 运行 `apps/web/e2e/performance.spec.ts`
6. 从 `window.__MINI_CONF_PERF__` 读取 route/API timing
7. 写入 `target/perf/web-route.json`

前端 recorder 位于：

- `apps/web/src/app/performance.ts`

当前记录：

- route transition duration
- API request duration/status

默认阈值：

- `PERF_WEB_MAX_ROUTE_MS=250`
- `PERF_WEB_MAX_API_MS=150`

超过阈值时 Playwright 用例失败，并在 `target/perf/web-route.json` 中写入 `violations`。

浏览器内可直接查看：

```js
window.__MINI_CONF_PERF__?.snapshot();
```

## 5. Bundle Budget

入口：

- `just perf-bundle-budget`

真实执行脚本：

- `scripts/web-bundle-budget.sh`

行为：

1. 默认执行 `pnpm --dir apps/web build`
2. 扫描 `apps/web/dist/assets`
3. 计算 JS/CSS gzip 体积
4. 写入 `target/perf/bundle-budget.json`
5. 超过 budget 时失败

默认 budget：

- `BUNDLE_BUDGET_MAX_JS_GZIP_KB=450`
- `BUNDLE_BUDGET_MAX_CSS_GZIP_KB=80`
- `BUNDLE_BUDGET_MAX_TOTAL_GZIP_KB=800`

如果已经有最新构建，可跳过 build：

```bash
BUNDLE_BUDGET_BUILD=0 just perf-bundle-budget
```

当前 Vite 构建已按 vendor 分块：

- `vendor-vue`
- `vendor-element-plus`
- `vendor-codemirror`
- `vendor`

前端入口已改为 Element Plus 局部组件注册，CodeMirror 编辑器组件也从草稿/发布详情页面中异步加载。Element Plus 和 CodeMirror 仍是主要 vendor 成本，需要继续做页面级懒加载和组件使用面收敛。

## 6. DB 慢查询归档

入口：

- `just perf-db-slow-queries`

真实执行脚本：

- `scripts/perf-db-slow-queries.sh`

行为：

1. 检查当前数据库是否可用 `pg_stat_statements`
2. 可用时按 `total_exec_time` 导出慢查询排行
3. 不可用时写入 unavailable 报告，CI 不因此失败
4. 写入 `target/perf/db-slow-queries.json`

可选参数：

- `PERF_DB_SLOW_QUERIES_LIMIT=20`
- `PERF_DB_SLOW_QUERIES_CREATE_EXTENSION=1`

说明：生产或专用性能环境应在 PostgreSQL 配置中启用 `shared_preload_libraries=pg_stat_statements`，否则该报告只能记录不可用原因。

## 7. CI

定时与手动 workflow：

- `.github/workflows/perf.yml`

该 workflow 启动 PostgreSQL，运行 `just perf-ci`，并上传：

- `target/perf/smoke-S.json`
- `target/perf/smoke-M.json`

该 workflow 也会运行 DB 慢查询归档，并上传：

- `target/perf/db-slow-queries-S.json`
- `target/perf/db-slow-queries-M.json`

该 workflow 会运行前端生产态 smoke，并上传：

- `target/perf/web-route.json`

该 workflow 也会运行 bundle budget，并上传：

- `target/perf/bundle-budget.json`

每个 perf job 会额外运行：

- `just perf-summary`

汇总产物：

- `target/perf/summary.md`
- `target/perf/summary-S.md`
- `target/perf/summary-M.md`

普通 CI 仍运行：

- `just perf-smoke`

## 8. 下一阶段

Phase 5 建议：

1. 将 `/metrics` 接入实际 Prometheus/Grafana，沉淀 dashboard 和告警阈值
2. 为 `PERF_SMOKE_DATASET=L` 建立夜间趋势对比，而不是只看单次阈值
3. 将业务热点指标细化到发布、草稿保存、bundle size、权限失败率等维度
4. 对 Element Plus 做更激进的页面级隔离，降低 `vendor-element-plus`
5. 对 CodeMirror 做编辑器路径预加载策略，平衡首屏和首次打开编辑器体验
