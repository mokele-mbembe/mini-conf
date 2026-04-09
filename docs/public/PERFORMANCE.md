# 性能测试 Scaffold

## 1. 文档目标

这份文档定义 `mini-conf` 在 MVP 阶段的性能测试最小脚手架。

目标不是现在就做重性能优化，而是：

- 提前把性能测试入口接入到工程工作流
- 让后续 TDD 不只覆盖功能正确性，也能逐步覆盖性能约束
- 先用一个很轻量的指标把流程跑起来

## 2. MVP 指标

MVP 阶段先保留一个容易落地的指标：

- `config_resolve_smoke_ms`

它代表：

- 单次配置解析接口的轻量 smoke benchmark

目标接口：

- `GET /api/open/configs/resolve`

## 3. 当前实现方式

当前仓库还没有正式代码，因此先提供一个 scaffold：

- `scripts/perf-smoke.sh`
- `just perf-smoke`
- `just perf-ci`
- GitHub Actions 中的 `Perf` workflow

当前状态：

- 如果还没有真实 benchmark 命令，就输出 placeholder 结果
- 一旦后续补上 `scripts/run-perf-smoke.sh`，就自动切换成真实测量

## 4. 默认阈值

默认阈值：

- `PERF_SMOKE_MAX_MS=250`

说明：

- 这个值只是为了让流程先运转起来
- 不是最终性能目标

## 5. 后续演进

后续可以逐步增强为：

1. 本地启动服务后对 `/api/open/configs/resolve` 进行真实压测
2. 将结果写入 `target/perf/smoke.json`
3. 在 PR 或定时任务里做趋势对比
4. 增加 bundle 接口的 benchmark
5. 增加数据库查询路径 benchmark

## 6. 与 TDD 的关系

建议后续工作流：

- 功能逻辑先写失败测试
- 接口落地后补性能 smoke
- 性能回退超过阈值时，再决定是否阻塞合并

MVP 阶段先做到：

- 有入口
- 有产物
- 有 CI

这样后续做性能优化时就不用从零搭流程。
