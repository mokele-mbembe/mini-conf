# 0006 配置文件格式与体验对齐

## 背景

近期在配置文件页联调中，已经暴露出一组“实现可做 / 文档似乎暗示 / 产品实际并不想要”之间的偏差：

- `text` 被前端误暴露为可选格式，但它不在当前预期范围内
- `toml` 按计划应进入 MVP，目前已一次性拉齐前后端与文档
- `config_files.status` 前端曾暴露 `inactive`，但后端当前资源语义并不按它工作
- `schema_name / schema_version` 曾有 demo 期痕迹，但不适合作为 MVP 对外能力表述，目前已从主链路清理
- `code` 在中文里写成“编码”容易被理解成字符编码
- 后续希望支持中英文切换，但目前文案仍大量散落在页面组件中

这份文档用于把这组问题一次性收口，作为后续前端、后端、OpenAPI、demo seed 和文档修改时的统一真值。

## 当前决定

### 1. `schema` 从 MVP 对外能力中移除

MVP 当前只承诺：

- 配置内容的基础格式合法性校验
- Draft 保存、clone、发布前的格式解析失败提示

MVP 当前不再对外承诺：

- 独立的 schema 校验能力
- `schema_name / schema_version` 的产品级配置能力
- 可被前端明确配置和理解的一整套 validator 体系

说明：

- 已清理的 `schema_name / schema_version` 字段和内置 validator 痕迹，更接近 demo 期内嵌规则，不应继续作为 MVP 业务卖点或前端字段。
- 已移除的旧做法只是让用户输入名称和版本字符串，并不是真正的 schema 资源关联模型；如果未来真的要扩展 schema 能力，仍然需要重新设计表结构、资源模型和业务逻辑。
- 本轮执行结果不是“先把 UI 藏起来、数据库列继续保留”，而是已从表结构、接口、seed、OpenAPI 和文档里一起清理干净。
- 后续如需恢复更完整的 schema 能力，应单独立题，并按“独立 schema 资源 + 明确关联方式”重新设计，而不是恢复当前这套自由文本字段。

### 2. `text` 明确移出当前 MVP

当前结论：

- `text` 不是当前 MVP 预期内格式
- 前端不应再展示 `text`
- 后端 `config_files.format` 不应再接受 `text`
- 示例数据、文档和 OpenAPI 表达也不应继续暗示 `text` 可用

如果仓库里已存在历史 `text` 记录：

- 允许临时以兼容视角读取和修正
- 但不应再允许作为新建或稳定编辑目标继续传播

### 3. `toml` 保留在 MVP 目标内，并按专项批次一次性打通

当前结论：

- `toml` 不像 `text` 那样被彻底移除
- TOML 已按专项批次一次性拉齐前端、后端、测试、OpenAPI、文档
- 下面这些能力已补齐，TOML 视为当前 MVP 已支持：

- 配置文件格式白名单接受 `toml`
- Draft 保存支持 TOML 格式解析
- Draft clone 支持 TOML
- 发布前校验支持 TOML
- secret redaction 在 TOML 下行为明确
- OpenAPI、前端表单、产品文档同步更新

当前状态更新：

- 上述 TOML 主路径已打通
- 配置文件表单正式支持 `yaml / json / toml`
- 管理端 release detail/diff 对 TOML `secret_paths` 执行局部脱敏
- Open API 在 `resolve / release / config-bundle` 三条读取路径中保留 `format = toml`

### 4. `config_files.status` 当前只采用 `active | archived`

当前结论：

- 对配置文件资源，正式状态集合当前只认 `active` 和 `archived`
- `inactive` 不属于当前 `config_files` 的正式产品语义

因此后续收口要求是：

- 前端配置文件页不展示 `inactive`
- OpenAPI 与文档对配置文件状态的表达按 `active | archived` 收紧
- 后端 `config_files` 写接口的校验与错误提示按这两个状态收口

说明：

- 其他资源是否继续使用 `inactive`，不在本篇文档范围内
- 本文只定义 `config_files` 这一个资源的状态语义

### 5. `code` 的中文统一改成 `配置标识`

当前结论：

- 在配置文件上下文里，`code` 的中文统一使用 `配置标识`
- 不再使用“编码”指代该字段

理由：

- “编码”容易让人联想到字符编码或 encode 方式
- `code` 在这里实际表达的是项目内唯一标识、配置 key 或逻辑名

后续若涉及项目、部署实例等同类字段，也应优先采用“标识”而非“编码”，避免中文误导。

### 6. 中英文切换作为后续工程能力保留

当前结论：

- 本轮不要求管理台立刻完成中英文切换
- 但从当前批次开始，不应继续无节制新增散落的硬编码文案

后续最小要求：

- 新增或重写页面时，优先把业务文案抽到集中常量或字典层
- 先做到“可收口”，再决定是否正式引入 `vue-i18n`

这意味着：

- 当前先不做全站翻译
- 但后续页面实现与文案修订，应以“未来需要双语”为前提组织文案

## 本轮对齐清单

### 文档

- [x] `README.md`
  - 把“schema 校验”收口成“基础格式合法性校验”
- [x] `docs/constraints/FRONTEND_MVP_BLUEPRINT.md`
  - 配置文件表单字段移除 `schema_name / schema_version`
  - `code` 的中文语义统一改成 `配置标识`
- [x] `docs/collaboration/FRONTEND_HANDOFF.md`
  - 删除“schema validator 已作为前端主路径能力落地”的误导性表述
  - 明确 `text` 已移除、`toml` 已支持、`inactive` 不适用于配置文件
- [x] `docs/constraints/ADMIN_API.md`
  - 配置文件 API 示例移除 `schema_name / schema_version`
  - 明确 `format` 与 `status` 的正式集合
- [x] 如有配置文件页专项规范或 handoff，再统一引用本篇，不重复维护平行结论

### 后端

- [x] `config_files.format` 增加显式白名单校验
- [x] 当前阶段至少拒绝 `text`
- [x] `config_files.status` 的写接口继续按 `active | archived` 收口
- [x] Draft 保存、clone、发布前只保留基础格式合法性校验
- [x] 删除 `schema_name / schema_version` 相关表字段、查询字段和响应字段
- [x] 删除当前 schema-specific 内置 validator 及其对 Draft / Publish 的影响

### 前端

- [x] 配置文件表单移除 `Schema 名称 / Schema 版本`
- [x] `code` 字段中文统一显示为 `配置标识`
- [x] `text` 不再暴露
- [x] `inactive` 不再出现在配置文件状态筛选或编辑状态中
- [x] `invalid_request` 及格式/状态非法值需给出明确业务提示，不再只落成“未知错误”
- [x] 后续新增文案尽量先进入统一常量或字典层

### OpenAPI / 示例数据

- [x] `docs/artifacts/openapi.json`
  - 配置文件格式与状态的表达与当前正式集合一致
- [x] demo seed 与本地演示数据
  - 清理误导性的 `text` 示例
  - 删除 `schema_name / schema_version` 痕迹

## TOML 专项完成情况

TOML 已按独立批次一次性拉齐前后端、测试、OpenAPI、文档，不再处于“计划支持”状态。

本批次完成结果：

1. 后端 `config_files.format` 正式接受 `toml`
2. `validation` 增加 TOML 解析与序列化
3. Draft 保存、clone、发布前接通 TOML
4. TOML `secret_paths` 采用统一 JSONPath 风格并支持局部 redaction
5. 增加对应后端单元测试与集成测试
6. 前端格式选择恢复 TOML
7. 文档把 TOML 从“计划支持”升级为“已支持”

### 后端实施结果

为避免再次出现“数据层允许、主链路不支持”的半完成状态，本批次后端按下面的顺序一次收口：

#### A. 先收紧 `format` 契约

- 为 `config_files` 的创建 / 更新接口增加显式白名单校验
- TOML 相关白名单调整放到独立 TOML 批次内一次完成
- `text` 明确拒绝，并返回可理解的业务错误，而不是落成内部错误或“未知错误”

这样做的目的是先阻止继续把明显不在范围内的格式写进库里，同时避免围绕 TOML 做多次折返修改。

#### A.0 先清理 schema 痕迹

在补 TOML 之前，先把当前 MVP 不再保留的 schema 痕迹从后端主链路中移除：

- 删除 `config_files.schema_name`
- 删除 `config_files.schema_version`
- 删除 `drafts.schema_version`
- 删除对应的 migration / schema 文档 / OpenAPI 字段 / seed 数据 /测试断言
- 删除当前通过 `(schema_name, schema_version)` 选择 validator 的逻辑

原因：

- 当前做法不是正式的 schema 资源模型，只是自由文本字段
- 如果未来真的支持 schema，仍然会改表和改业务逻辑
- 因此继续保留这套半成品，只会让当前 MVP 语义和后续扩展都更混乱

#### B. 补 `validation` 层的 TOML 解析

- 在 `apps/server/src/validation.rs` 中补 `toml` 的 parse 分支
- TOML 的基础格式校验目标只包括：
  - 内容可以被合法解析
  - 解析结果可转换为当前 redaction / validation 所需的中间结构
- 当前阶段不追加 TOML 专属业务规则
- 如果 TOML 解析成功但无法转换到统一结构，应返回明确的 `draft_validation_failed`

#### C. 接通 Draft 主路径

- `PUT /api/drafts/{deploymentId}/{configFileId}`
  - 对 `format = toml` 的 payload 执行基础格式合法性校验
- `POST /api/drafts/{targetDeploymentId}/{configFileId}/clone`
  - 当源 Draft / Release 为 TOML 时，允许 clone
- `draft format must match config file format` 这条规则继续保留
- 当前如果 `config_file.format = toml`，则：
  - 保存 Draft 成功
  - clone 成功
  - 非法 TOML 返回 `draft_validation_failed`

#### D. 接通发布主路径

- `POST /api/releases/publish`
  - 对 TOML Draft 在发布前执行二次基础格式校验
- `GET /api/releases/:id`
  - 对 TOML 的 `format` 正常返回
- `GET /api/releases/:id/diff`
  - 当前先继续沿用文本级 diff，不做 TOML AST 语义 diff

也就是说，TOML 在 MVP 里的目标是“可保存、可发布、可比对文本差异”，而不是“有结构化 TOML diff”。

#### E. 定义 TOML 下的 `secret_paths`

- 先明确一个实现原则：
  - `secret_paths` 只有在 TOML 可被稳定映射到统一文档结构时才生效
- 如果 TOML 已能稳定映射，则 redaction 行为对齐 YAML / JSON：
  - 路径命中则局部脱敏
  - 路径无法命中时按当前策略回退
- 当前实现已采用统一中间结构，因此 TOML 的 `secret_paths` 与 YAML / JSON 保持同等能力

#### F. 补齐后端测试

至少新增或调整这些测试：

- `config_files` 创建 / 更新
  - 接受 `yaml / json`
  - 接受 `toml`
  - 始终拒绝 `text`
- `drafts`
  - TOML Draft 保存成功
  - 非法 TOML 返回 `draft_validation_failed`
  - TOML clone 成功
- `releases`
  - TOML 发布成功
  - 非法 TOML Draft 在发布前失败
- TOML redaction：
  - 增加 secret 路径脱敏回归

#### G. 收尾同步项

- 更新 OpenAPI 导出产物
- 更新 `ADMIN_API.md`
- 更新前端 handoff / blueprint 中的格式说明
- 更新 demo seed，至少补一条合法 TOML 示例
- 确认仓库中不再残留 `schema_name / schema_version` 的对外语义描述

## 后续续工建议

如果下一轮继续收口配置文件页，不要直接只修单个表单或某个报错。

建议顺序：

1. 先确认是否有新的产品语义偏差
2. 再补对应后端契约、前端交互和验收测试
3. 最后同步 OpenAPI、demo seed 和文档

避免再次出现：

- UI 看起来支持
- 数据层能写进去
- Draft / Publish 主路径却并未真正支持
