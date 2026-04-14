# 部署实例模型与未来 Scope 规划

## 1. 文档目标

这份文档说明 `mini-conf` 在 MVP 阶段为什么优先采用 `DeploymentInstance` 模型，以及未来如何平滑扩展到 `Scope` / `labels`。

## 2. 为什么 MVP 先不用 Scope 做主模型

从你的实际业务看，当前更自然的结构是：

- 一个 `Project` 对应一套代码
- 一个 `Project` 下有多份 `ConfigFile`
- 一个 `Project` 在多个环境下有多份独立 `DeploymentInstance`
- 一个 `DeploymentInstance` 持有该项目下的一整套配置
- 同一部署实例上的多个进程共享同一份部署实例凭证访问平台

这比“先理解 Scope、labels、动态匹配”更符合直觉。

## 3. MVP 主模型

MVP 采用：

- `Project`
- `ConfigFile`
- `DeploymentInstance`
- `DeploymentCredential`

工作流：

1. 创建项目
2. 创建配置文件
3. 在某个环境下创建一个部署实例
4. 为部署实例编辑多份配置文件的 Draft
5. 发布各配置文件的 Release
6. 给部署实例发放共享凭证
7. 各进程使用同一凭证按配置文件名拉取自己的配置，或一次拉取整部署实例配置包

## 4. 模板克隆

模板通过部署实例本身承载：

- `is_template = true`

新部署实例可从模板部署实例克隆：

- 全部 Draft

当前不支持：

- 从模板实例克隆最新 Release 内容，因为模板实例本身禁止发布

克隆完成后：

- 新部署实例与模板不联动

## 5. 后续模板同步更新

你提出的这个方向很有价值：

- 模板后续发生变化时
- 管理后台或 API 提供一个显式触发的同步更新操作
- 通过 Diff 预览和批量替换，把模板变化应用到一批部署实例

这个能力：

- 不进入 MVP
- 但很适合记录为 MVP 之后的重点计划

## 6. labels 是什么

如果后续引入 `labels`，它表示：

- 部署实例或客户端的附加标签信息

例如：

- `region=hangzhou`
- `model=coffee-v1`
- `store_type=mall`

## 7. Scope 是什么

如果后续引入 `Scope`，它表示：

- 某份配置在某个环境下的一个动态发布范围

适合场景：

- 灰度发布
- 地域分组
- 设备型号分组
- 服务实例批量分群

## 8. 未来扩展路线

后续可以在不推翻 MVP 的前提下这样扩展：

1. 保留 `DeploymentInstance` 作为显式部署实例
2. 为部署实例补充可选 `labels`
3. 在平台中增加可选 `Scope`
4. 允许某些配置通过动态匹配自动落到一批部署实例

也就是说：

- MVP 先做显式部署实例管理
- 未来再叠加动态范围能力

## 9. 结论

当前阶段：

- `DeploymentInstance` 是主路径
- `Scope / labels` 是后续扩展能力

这不是能力退化，而是更贴近真实需求的裁剪。
