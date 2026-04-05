# 提交前 Review 清单

## 1. 文档定位检查

- README 开头是否已经明确项目差异点：
  - `DeploymentInstance`
  - HTTP-first
  - 多配置文件
  - 多进程共享凭证
  - 模板克隆
  - 整部署实例配置包拉取
- README 是否已经包含 `coffee-legacy` 的真实业务例子
- README 是否没有把项目表述成“普通配置中心 clone”

## 2. 术语一致性检查

确认这些核心术语在文档中保持一致：

- `Project`
- `ConfigFile`
- `DeploymentInstance`
- `DeploymentCredential`
- `Release`
- `Draft`
- `DeploymentSyncRecord`

确认 MVP 主路径统一为：

- `Project -> Environment -> DeploymentInstance -> 多个 ConfigFile`

确认 `Scope / labels` 只作为后续扩展能力，而不是当前 MVP 主路径。

## 3. 功能边界检查

确认文档已经写清楚：

- MVP 支持单配置文件拉取
- MVP 支持整部署实例配置包拉取
- 重复发布相同 Draft 内容仍生成新 revision
- 未命中配置时明确返回失败，由客户端自己兜底
- 模板克隆后不自动联动
- 模板同步更新属于 MVP 之后计划

## 4. 认证与权限检查

确认文档已经写清楚：

- 管理端设计支持 `Session Cookie` 和 `JWT`
- MVP 只完整实现 `Session Cookie`
- 后续版本补 JWT 和 OAuth 2.0
- 消费端 token 默认长期有效、支持手动重置和吊销
- 权限模型为项目级 `admin / editor / viewer`

## 5. 工程化检查

确认仓库已包含：

- `justfile`
- GitHub Actions workflow
- `.gitignore`
- `.gitattributes`
- `.editorconfig`
- `lefthook.yml`
- 性能测试 scaffold

## 6. 性能 scaffold 检查

确认文档和脚手架已经写清楚：

- MVP 不追求现在就做深度性能优化
- 已有 `perf-smoke` 入口
- 已有轻量指标 `config_resolve_smoke_ms`
- 后续可替换为真实 benchmark

## 7. Linux / WSL2 检查

确认文档已经写清楚：

- Windows 仓库当前只用于设计与规划
- 真正开发和测试在 Linux / WSL2 中进行
- 脚本优先使用 shell / `just`
- 不以 PowerShell 作为主工作流

## 8. 提交前最后确认

提交前建议最后再人工确认这几件事：

1. 是否还有不想公开的环境信息或内部项目细节
2. `coffee-legacy` 这个例子是否允许直接出现在开源仓库中
3. 文档命名是否已经达到你满意的对外表达
4. 是否要在首次提交里同时加入 `.github/PULL_REQUEST_TEMPLATE.md`
