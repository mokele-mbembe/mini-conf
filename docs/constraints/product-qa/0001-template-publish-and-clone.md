# 0001 Template / Publish / Clone 澄清

## 背景

围绕 `DeploymentInstance`、`Template`、`Draft`、`Release` 的关系，当前代码已经有一版可运行实现，但产品语义需要进一步收紧。

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

方向上可以，当前只做了元数据预留，尚未真正实现。

当前已有：

- `config_files.schema_name`
- `config_files.schema_version`
- `config_files.sensitivity`

当前未完成：

- 保存 Draft 时的 schema 校验
- 发布前的强化校验
- “某个配置文件必须满足更强规则才能成为候选/可发布”的业务规则

所以这部分目前只是“元数据就位，校验能力未落地”。

## Q6: Template 是否应该允许发布 Release？

产品澄清后的目标：

- `Template` 是一个不可发布的特殊实例
- 它主要用于快速创建新实例
- 模板内容应以 Draft 作为来源，不应直接形成可被消费端拉取的 Release

当前代码状态：

- 还没有禁止模板实例执行 `publish`
- 这是接下来应该修正的规则

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
- 支持 `clone_source = draft | latest_release`
- 会创建新实例，并把内容复制到新实例的 Draft

当前未满足目标的部分：

- 还没有禁止模板实例发布 Release
- 还没有“单配置文件 clone”接口
- 还没有项目成员权限，当前主要还是管理员 session 语义

## 当前建议

后续代码调整按这个顺序推进：

1. 禁止 `is_template = true` 的实例执行 `publish`
2. 保留“从模板创建新实例”的 clone 语义
3. 新增“单配置文件 clone”接口，供前端批量调用
4. 接入 `project_members` 和项目级编辑权限
