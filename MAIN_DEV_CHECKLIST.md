# MVP 前单人 `main` 开发每日清单

适用范围：

- 当前只有一名维护者
- 产品还没上线
- 业务设计仍可能推翻
- 默认直接在 `main` 上开发
- mvp完成前,不要为了不需要的"平稳过渡"做没有必要,只会翻倍工作量的"分步渐进工作计划".及时转向合理优雅简洁的设计,不要在这种场景引入最小化修改的workaround.

## 每日最小规则

1. 一次提交只做一个主题，不把接口、迁移、文档、前端脚手架混进同一提交
2. 非 DB 改动 push 前至少执行 `just ci-local`
3. 涉及迁移、SQL、权限、发布链路、初始化逻辑时，额外执行 `just ci-local-db`
4. 需要完整本机收口时，直接执行 `just ci-local-full`
5. push 后若 GitHub Actions 失败，先修 `main`，不要继续叠加新主题
6. 只有在高风险实验或修 CI 时才开临时分支，回收时优先线性合回 `main`

## 常用命令

最常见的非 DB 提交前检查：

```bash
just ci-local
```

数据库主路径改动前检查：

```bash
just ci-local-db
```

需要一次跑完本机基线和 DB 校验：

```bash
just ci-local-full
```

前端联调前准备本机运行库：

```bash
just dev-db-prepare-local
just run-server-local
just dev-web
```

如果这轮改动触及前端主路径，提交前额外建议执行：

```bash
pnpm --dir apps/web build
just test-e2e-local
```

## 推荐节奏

```bash
git pull --ff-only origin main
# 开发 / 提交
just ci-local
git push origin main
```

如果这次改动碰到了数据库主路径：

```bash
git pull --ff-only origin main
# 开发 / 提交
just ci-local
just ci-local-db
git push origin main
```

## 何时暂停 `main` 直推

出现下面任一情况时，恢复短命分支或 PR：

- 一次改动跨度明显超过一个主题
- 你需要做高风险试验，不确定是否要保留
- 你需要单独修 CI，而不想打断正在进行的主线开发
- 仓库开始有第二个维护者
- staging / production 已经有真实使用者
