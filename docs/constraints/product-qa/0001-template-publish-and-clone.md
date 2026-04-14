# 0001 Template / Publish / Clone 澄清

## 背景

围绕 `DeploymentInstance`、`Template`、`Draft`、`Release` 的关系，这份文档用于固定当前已经落地的产品语义，避免前后端再次按旧草案理解。

## Q1: Template 是不是单独的模型？

不是。

当前和目标都保持：

- `Template` 仍然是 `DeploymentInstance` 的一种特殊形态
- 用 `is_template = true` 表示
- 模板本质上仍然属于某个项目和环境

这点与当前表结构和代码方向一致。

## Q2: 当前 publish 是“对实例”还是“对单配置文件”？

当前实现是：

- `publish` 针对 `deployment_instance_id + config_file_id`
- 也就是“某个实例下的单个配置文件发布”
- 对应接口是 `POST /api/releases/publish`

这符合当前系统的核心建模：

- 一个实例下可以有多份配置文件
- 每份配置文件各自拥有 Draft / Release 历史

所以 `publish` 不是“整个实例一次性发布全部配置”，而是“单配置文件发布”。

## Q3: 每个实例下的每个配置文件，当前能不能有多个候选待选状态？

当前不能。

现在的模型是：

- 每个 `deployment_instance + config_file` 只有 1 份当前 Draft
- 由 `drafts` 表中的唯一约束保证
- Release 是历史不可变版本

因此当前没有“多个候选 Draft 并存”的能力。

如果后续需要“一个实例下同一配置文件有多份候选稿”，那会是另一层模型，例如：

- `draft_snapshots`
- `draft_candidates`

这不在当前 MVP 范围内。

## Q4: 一个项目创建了多份配置文件后，某些实例可不可以只配置其中一部分？

可以，这也是当前实现允许的状态。

当前允许：

- 项目下存在多份 `ConfigFile`
- 某个实例只对其中一部分有 Draft / Release
- 另一部分完全为空，没有 Draft，也没有 Release

这与 open API 当前行为一致：

- `config-bundle` 会只返回该实例下“已有发布”的配置
- 未发布的配置不会被强制补空对象

## Q5: 某些配置文件能不能做更严格的校验？

方向上可以，但不属于当前 MVP 对外能力。

当前已有：

- `config_files.sensitivity`
- `secret_paths` 局部脱敏
- 保存 Draft 时的基础格式解析
- Draft clone 时的基础格式解析
- 发布前的二次基础格式解析

当前明确不再保留的是：

- `config_files.schema_name`
- `config_files.schema_version`
- 基于用户输入 schema 名称 / 版本选择 validator 的主路径

如果后续要做更严格校验，应单独设计为完整能力，例如：

- 独立 schema 资源
- 明确的 schema 与配置文件关联关系
- 可审计、可版本化的 validator 或规则集
- 前端可理解的校验错误结构

## Q6: Template 是否应该允许发布 Release？

产品澄清后的目标：

- `Template` 是一个不可发布的特殊实例
- 它主要用于快速创建新实例
- 模板内容应以 Draft 作为来源，不应直接形成可被消费端拉取的 Release

当前代码状态：

- 已禁止模板实例执行 `publish`
- 模板仍只承担“作为 clone 来源”的职责

## Q7: Clone 的目标应该是什么？

目标语义分两类：

1. 从 Template 快速创建新实例

- 这是实例级 clone
- 创建出一个新的普通实例
- 并复制模板内容到新实例

2. 从任意有权限的实例复制“单个配置文件”

- 这是配置文件级 clone
- 目标应是某个 `deployment_instance + config_file`
- 前端可以通过多次调用单文件 clone 完成批量 clone

## Q8: 当前代码和目标语义的差异在哪里？

当前已实现：

- `POST /api/deployment-instances/:id/clone`
- 模板 clone 仅允许 `clone_source = draft`
- `POST /api/drafts/:targetDeploymentId/:configFileId/clone`
- 单配置 clone 支持 `source_kind = draft | latest_release`
- 会创建新实例，并把内容复制到新实例的 Draft
- 模板实例发布已被禁止
- 项目成员与项目级权限已接入

## 当前结论

- 模板仍然是实例概念，不是独立资源
- 模板可以 clone，但不能发布
- 模板创建新实例与单配置 clone 是两条不同交互路径
- 前端批量复制配置时，应通过多次单配置 clone 完成
