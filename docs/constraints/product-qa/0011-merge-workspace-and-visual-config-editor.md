# 0011 Merge Workspace / 可视化配置合并工作台 澄清

## 背景

当前配置中心已经具备：

- Current Draft 编辑
- Saved Versions 历史保存
- Release 只读回看和 Diff
- 从其他实例 / 最新 Release 恢复到 Current Draft
- preview-bundle / publish 主闭环

但围绕长期高频使用场景，仍有一个核心缺口：

- 软件版本升级后，需要给大量部署实例的配置补一行或几行配置
- 每个实例原有内容不同，不能整份复制替换
- 用户希望人工确认每一处变更，而不是完全依赖机器自动覆盖
- 当前 Draft 编辑页的视觉重心偏弱，在线编辑和变更比对没有形成统一工作台

这份文档用于固定下一阶段的主线方向：

- 不是只补一个简单 diff 弹窗
- 而是引入一套 **Merge Workspace（可视化配置合并工作台）**
- 同时收束后续在线编辑、只读查看、Diff 和 Merge 的视觉体系

## Q1: 这个能力的产品目标是什么？

目标不是做一个通用 Git 客户端，也不是做“机器自动帮你改完”。

目标是提供一套对配置中心用户足够可信的工作流：

1. 选择一个来源配置
2. 和当前配置做三方比对
3. 一次性预览所有差异
4. 用明显的色块、箭头和结果窗格人工确认
5. 把确认后的结果写回 Current Draft
6. 再按现有流程 preview / publish

核心价值：

- 用户能明确看见自己保留了什么、引入了什么
- 适合“统一加一行配置，但每个实例已有内容不同”的场景
- 让在线编辑真正成为系统核心能力，而不是附着在管理页中的普通表单

## Q2: 应该参考哪类成熟实现？

应优先参考：

1. **JetBrains Diff & Merge Viewer**
   - 左 / 中 / 右三窗格
   - 通过箭头把左侧或右侧变更应用到中间结果
   - 可一键应用全部非冲突变更
   - 支持手工继续编辑结果窗格

2. **VS Code Merge Editor**
   - 明确使用三方合并模型
   - 把 `incoming / current / result` 区分清楚
   - 合并不是单纯查看 diff，而是以“结果窗格”为中心

不建议把 GitHub 的 Web conflict editor 作为主要参考对象。

原因：

- GitHub Web conflict editor 更偏简单行级冲突处理
- 适合 PR 冲突，不适合“在线配置工作台”
- 视觉引导和结果编辑能力都偏弱

GitLab 也不适合作为 merge UX 的主要参考对象，但它在编辑器接入方式上有参考价值：

- Web 编辑器使用 Monaco
- 说明大型在线系统会把编辑能力抽成独立工作区，而不是散在普通表单中

## Q3: 当前最稳的实现路线是什么？

当前最稳的路线是：

**文本三方合并工作台 + 结构化辅助分析 + Current Draft 单一主线模型**

不建议把“结构化自动注入”作为第一阶段主线。

原因：

- 你的核心诉求是人工确认和可视化引导
- 注释、空行、排版在配置文件里也有业务价值
- 如果系统直接按 AST 改写，很容易破坏用户熟悉的原文结构
- 用户会对“机器自动改配置”缺乏信任

所以第一阶段应坚持：

- 文本是最终真值
- 用户在可视化 merge 工作台中完成确认
- 结构化解析只做辅助标签和安全预判

## Q4: 为什么一定要用三方合并，而不是普通两方 diff？

因为这个能力不是“看差异”，而是“把来源改动引入当前配置”。

要做到这一点，系统必须区分：

1. **base**
   - 当前配置和来源配置的共同基线
2. **source**
   - 用户选中的来源配置
3. **target**
   - 当前正在编辑的 Current Draft
4. **result**
   - 用户确认后的结果

如果只有两方 diff：

- 系统无法判断某一段是来源新增，还是 target 自己已有修改
- 也无法可靠判断哪些属于非冲突变更

所以最小正确模型必须是三方：

```text
base
├─ source  (来源版本)
└─ target  (当前 Draft)
   ↓
result     (合并结果)
```

## Q5: 用户工作流应该是什么？

推荐工作流：

1. 用户进入某个实例 + 某个配置文件的 Current Draft
2. 点击 `从其他配置 Merge`
3. 选择来源对象
   - 模板配置文件
   - 其他实例配置文件
   - Saved Version / Release（后续可扩展）
4. 系统计算三方合并预览
5. 打开 Merge Workspace
6. 用户逐块确认：
   - 接受来源变更
   - 保留当前 Draft
   - 同时保留后手工修改
   - 跳过该块
7. 点击 `保存到 Current Draft`
8. 系统先自动生成一条 Saved Version 作为回退点
9. 把 result 写回 Current Draft
10. 用户回到正常 Draft 工作流继续 preview / publish

这个流程的重点是：

- 每次 merge 只修改当前这个 Current Draft
- 不生成第二份并列 Draft
- 不直接发布
- 不绕过人工确认

## Q6: 页面层级应该怎么设计？

建议把在线配置相关能力收束成统一视觉体系：

- 管理页列表、权限页、实例管理继续保持 Element Plus 默认风格
- 与“文本内容本身”强相关的页面单独形成 **Config Workspace** 风格

这组页面包括：

- Draft 编辑页
- Release 只读详情页
- Release Diff 页
- preview-bundle 只读查看区
- Merge Workspace

### Merge Workspace 的展示方式

推荐使用 **大号弹窗 / 对话框工作台**，而不是普通内联区域：

- 与管理后台表单区隔开
- 视觉上明确这是“高专注编辑任务”
- 后续方便在 Draft 编辑、Release 详情、实例详情中复用

建议布局：

```text
┌──────────────────────────────────────────────────────────────┐
│ Merge Workspace                                              │
│ Source: tpl-coffee-store-basic / coffee-main / rev ...       │
│ Base: current draft ancestor ...                             │
│ Actions: Apply non-conflicting | Collapse unchanged | Save   │
├──────────────────────────────────────────────────────────────┤
│ Left: Source            │ Middle: Result │ Right: Current    │
│ 彩色变更块 + 箭头       │ 可编辑最终结果 │ 彩色变更块 + 箭头 │
│ 行号 / gutter / 折叠    │                 │ 行号 / gutter     │
├──────────────────────────────────────────────────────────────┤
│ Bottom summary: N changes / M conflicts / saved-version info │
└──────────────────────────────────────────────────────────────┘
```

## Q7: 每个变更块应该支持哪些操作？

参考 JetBrains / VS Code merge editor，推荐支持：

- `接受左侧`
- `接受右侧`
- `附加左侧`
- `附加右侧`
- `忽略该块`
- `标记稍后处理`

同时提供全局操作：

- `应用全部非冲突变更`
- `仅看冲突`
- `折叠未变化区域`
- `撤销 / 重做`

结果窗格必须是可编辑的。

原因：

- 即使左右都接受了，最后也可能需要人工润色
- 例如 TOML 表尾逗号、括号、注释位置、段落顺序都可能需要调整

## Q8: 是否应该支持“点击单行 merge”？

不建议只做单行粒度。

最小可用粒度应是：

- **变更块（chunk / hunk）**

而不是：

- 单行
- 单字符

原因：

- 配置文件常见变更不是“某一行孤立替换”，而是一个逻辑段
- 仅做单行 merge 容易让用户陷入碎片化点击
- 也会让结果窗格更容易出现语法不完整状态

行级高亮当然要有，但操作粒度建议是块级。

## Q9: 技术上更适合用什么编辑器？

当前更推荐：

- **CodeMirror 6 作为统一代码工作区底座**
- Merge 能力优先使用 `@codemirror/merge`

原因：

1. CodeMirror 官方已有 merge view 能力
2. 更适合从“只读查看 + diff + merge”渐进扩展
3. 比直接上 Monaco 再自己拼 merge editor 风险更低
4. 当前项目前端尚未真正落地 Monaco，不存在既有 Monaco 编辑器包袱

Monaco 并不是不能做，但当前阶段不建议优先：

- Monaco 适合重型代码编辑器
- 但缺少现成、稳定、官方化的 JetBrains 式 merge editor 工作台
- 如果现在上 Monaco，merge UI 的大头仍然要自己拼

当前建议：

- Draft 编辑页、Release 详情、Diff、Merge Workspace 同一时期统一过渡到 CodeMirror 6 风格
- 管理类列表和表单仍保留 Element Plus

## Q10: 原有的 Draft 编辑是否也要统一到同一技术栈和设计？

建议明确为：**要统一，而且应把 Draft 编辑作为统一代码工作区的起点。**

原因：

1. 如果只有 Merge 用新工作区，而 Draft 仍停留在旧编辑体验中，用户会在“编辑”和“合并”之间来回切换两套心智模型
2. Draft、Release 只读、Diff、Merge 面对的是同一类对象：配置文本本身
3. 统一技术栈后，后续语法高亮、Diff、只读模式、行号、折叠、快捷键、主题都可以复用

推荐边界：

- **配置文本相关能力**统一到一套 `Config Workspace`
  - Current Draft 编辑
  - Release 只读代码视图
  - Release Diff
  - Merge Workspace
- **管理类页面**继续保留原后台风格
  - 实例列表 / 模板列表
  - 生命周期操作
  - 权限、成员、审计
  - 发布记录元信息

进一步约束：

- 普通只读信息仍在后台详情页中展示
- 配置文本的“只读预览”不应再走另一套临时 UI，而应复用同一代码工作区内核的只读模式
- 也就是说，不是“只有可编辑时才进入新工作区”，而是“只要用户在看配置文本，就进入统一的代码工作区体验”

首版可以接受的落地顺序：

1. 先把 Draft 编辑页切到统一编辑器栈
2. 再把 Release 只读详情和 Diff 迁过去
3. 最后接入 Merge Workspace

如果工程顺序上需要反过来实现 Merge Workspace 以验证核心交互，也可以，但产品定稿应坚持最终收束到统一栈。

## Q11: 后端需要提供什么接口？

建议新增一组与 Merge Workspace 对应的接口。

### 1. 计算 merge 预览

```http
POST /api/drafts/:targetDeploymentId/:configFileId/merge-preview
```

请求体示例：

```json
{
  "source_type": "deployment_draft",
  "source_deployment_instance_id": 42,
  "source_config_file_id": 7
}
```

返回结构建议：

```json
{
  "base": {
    "label": "template baseline",
    "content": "..."
  },
  "source": {
    "label": "tpl-coffee-store-basic / coffee-main",
    "content": "..."
  },
  "target": {
    "label": "current draft",
    "content": "..."
  },
  "result": {
    "content": "..."
  },
  "summary": {
    "change_count": 9,
    "conflict_count": 2,
    "safe_change_count": 7
  },
  "chunks": [
    {
      "id": "chunk-1",
      "kind": "safe_add",
      "status": "auto_applicable",
      "from_line": 18,
      "to_line": 21
    },
    {
      "id": "chunk-2",
      "kind": "conflict",
      "status": "manual_required",
      "from_line": 55,
      "to_line": 64
    }
  ]
}
```

### 2. 提交 merge 结果

```http
POST /api/drafts/:targetDeploymentId/:configFileId/merge-apply
```

请求体示例：

```json
{
  "base_version": 12,
  "merged_content": "...",
  "format": "toml",
  "source": {
    "source_type": "deployment_draft",
    "source_deployment_instance_id": 42
  }
}
```

服务端行为：

- 校验当前 Draft 版本未漂移
- 先生成 Saved Version
- 覆盖 Current Draft 内容
- 写 audit log：`draft.merged`

## Q12: `base` 应该如何确定？

首版可以先采用可实现优先的折中方案：

### Phase 1

- 让用户显式选择 source
- 后端以“当前 target draft 最近一次与 source 的共同可追溯版本”或简单共同版本策略生成 base
- 如果找不到理想基线，退化为文本级 merge base 候选

### Phase 2

为 deployment instance 增加模板同步追踪字段，例如：

- `template_source_id`
- `template_base_release_id`
- `template_last_synced_release_id`

这样模板同步型 merge 可以稳定找到共同基线。

当前结论是：

- 三方合并模型必须存在
- 但 MVP 可以先允许 base 的求法有工程化折中
- 不必等完整模板世代追踪建好后才开始做 Merge Workspace

## Q13: YAML / TOML 解析在第一阶段应该扮演什么角色？

建议定位为：

- **辅助分析层**

不是唯一合并引擎。

### 对 YAML

可使用支持 comment / metadata 的文档级 API。

### 对 TOML

后端可评估 `toml_edit` 这类保留 comment / formatting 的工具做辅助分析。

解析层可帮助系统识别：

- `safe_add`：新增 key，target 未触及相关区块
- `safe_update`：来源修改某个值，而 target 仍保持 base 值
- `conflict`：source 和 target 同时改了同一区域
- `comment_only`：只有注释变化

但最终：

- merge 结果仍以文本形式提交
- 注释、空行、顺序仍属于有效结果的一部分

## Q14: 是否要在第一阶段做批量自动引入？

不建议。

第一阶段应先做：

- **单实例、单配置文件的 Merge Workspace**

原因：

- 用户刚开始最需要的是可信的人工确认流程
- 先把视觉交互、三方模型、保存语义跑通
- 再考虑批量能力

批量能力应作为下一阶段建立在 Merge Workspace 之上：

1. 先做批量预检
2. 把实例分成：
   - 可自动合并
   - 有冲突
   - 无需变更
3. 自动合并类也应允许人工抽样复核
4. 冲突类回落到单实例 Merge Workspace

## Q15: 首版应该支持哪些来源？

首版建议限制为：

1. 其他实例的 Current Draft
2. 模板实例的 Current Draft

后续再扩展：

3. Saved Version
4. Latest Release
5. 指定 Release

原因：

- Draft 到 Draft 是最接近用户心智的“从其他配置 merge”
- 也最贴合你当前描述的工作流

## Q16: 这项能力与现有 Saved Versions / Release / Draft 模型如何配合？

Current Draft 仍是唯一主线编辑对象。

Merge Workspace 的结果不是新对象，而是：

- 写回 Current Draft

配套规则：

1. 进入 Merge Workspace 前不新建 Draft
2. 点击保存到 Current Draft 时，先生成一条 Saved Version
3. 发布入口仍然只发布 Current Draft
4. Release 继续只读

也就是说：

- Merge Workspace 是 Current Draft 的高级编辑模式
- 不是 Draft 分支系统

## Q17: MVP 范围应该收在哪？

推荐 MVP 范围：

### 必做

- 单实例、单配置文件 Merge Workspace
- 三方文本 merge 预览
- 彩色块级高亮
- 左 / 右应用箭头
- 结果窗格手工编辑
- 应用全部非冲突变更
- 合并摘要（变更数 / 冲突数 / 可自动应用数）
- 折叠未变化区域
- 保存前自动生成 Saved Version
- 来源对象和目标对象的清晰标签
- 保存后回到 Current Draft 主编辑页继续 preview / publish

### 可延后

- 批量 merge
- source 类型扩展到 release / saved version
- 结构化 key-path 视图
- AI merge 建议
- 模板世代完整追踪

## Q18: 这次 MVP 裁剪，具体保留了什么、后移了什么？

这一节用于逐项核对，避免“看起来像裁剪，实际把关键诉求漏掉了”。

### 已明确纳入 MVP 主线

这些能力是第一阶段方案中的核心，不是可有可无的增强项：

1. **从其他配置 Merge**
   - 来源先支持其他实例 Current Draft、模板实例 Current Draft
   - 这正对应“从某个已经加过新配置行的实例/模板里，把改动引入当前配置”

2. **JetBrains / Beyond Compare 风格的可视化引导**
   - 色块
   - 箭头
   - 结果窗格
   - 一次性预览全部冲突

3. **人工确认优先**
   - 不直接自动覆盖当前配置
   - 用户逐块确认后再保存

4. **结果可继续手工编辑**
   - 不是只能做“选左/选右”
   - 用户可以对 merge 结果继续微调

5. **注释和排版作为有效内容对待**
   - 首版不把配置简化成“只有 key/value 有意义”
   - 文本仍是最终真值

6. **Current Draft 单一编辑主线**
   - merge 完成后写回 Current Draft
   - 发布仍然只发布 Current Draft

### 在 MVP 中故意做轻，但没有否定其价值

这些方向被明确承认为后续增强，而不是“认为没必要”：

1. **批量 merge / 批量预检**
   - 不是不做
   - 是建立在单实例 merge 可信之后再推进

2. **来源扩展到 Saved Version / Release**
   - 首版先做最贴用户心智的 Draft -> Draft / Template Draft
   - 后续再补历史对象

3. **结构化 key-path 视图**
   - 未来可以补一个“按配置项路径看差异”的视图
   - 但第一阶段先不让它主导交互

4. **更严格的模板基线追踪**
   - 首版允许 `base` 求法工程化折中
   - 后续再引入更完整的同步基线字段

5. **更智能的安全合并建议**
   - 包括 safe-add / safe-update 标签更细化
   - 或 AI/规则辅助建议

### 当前方案明确没有纳入 MVP 的内容

这些并不是“漏掉”，而是当前刻意不让它们进入首版范围：

1. **全自动批量把某条配置注入所有实例**
   - 这会让用户对结果缺乏信任
   - 也会显著抬高回滚、审计、冲突解释成本

2. **一开始就做完整 AST 驱动改写**
   - 这会弱化注释、排版和原文结构
   - 不符合当前“人工确认文本结果”的主线

3. **把 merge 做成 Draft 分支系统**
   - 当前 Draft 仍保持唯一主线
   - 不引入多个并列草稿分支模型

### 对你当前需求的逐项对应

下面这些诉求，当前方案都已经覆盖，不应视为被裁掉：

- “原本的配置编辑也统一技术栈和设计”
- “从其他配置 merge”
- “通过色块和箭头快速定位并人工挑选变更”
- “merge 完后继续手工改”
- “注释修改也要被看见”
- “保存预览后有信心地修改和发布”

如果后续需要做更细的产品排期，可以把本节直接拆成实施 checklist，而不需要再重做方向讨论。

## 当前结论

- 在线配置编辑应提升为一套独立的 Config Workspace 视觉体系
- Merge 主线应参考 JetBrains / VS Code 的三方合并工作台
- 第一阶段不做“机器自动批量改配置”，而做“人工可控的可视化 merge”
- 文本是最终真值，结构化解析只做辅助分析
- 技术选型优先考虑 CodeMirror 6 + `@codemirror/merge`
- 当前最值得推进的最小功能是：**单实例、单配置文件的 Merge Workspace**
