# Docs Index

`docs/` 现在按受众和用途分成 6 类：

- [`public/`](./public/README.md)
  面向项目外部阅读者、使用者和部署者的说明文档。
- [`constraints/`](./constraints/README.md)
  面向产品边界、系统设计约束和实现语义收口的文档。
- [`agents/`](./agents/README.md)
  面向 AI agent 或本地自动化执行环境的工作流与环境约定。新会话统一从 [`AGENT_START_HERE.md`](./agents/AGENT_START_HERE.md) 进入。
- [`collaboration/`](./collaboration/README.md)
  面向潜在协作者、贡献者和仍在使用的工程协作流程文档。
- [`artifacts/`](./artifacts/README.md)
  面向生成产物和机器消费文件，不和 markdown 叙述文档混放。
- [`archive/`](./archive/README.md)
  保存已经完成、过渡期使用、或被新入口合并替代的历史文档。

整理原则：

- 叙述性 markdown 放在按受众分类的目录里。
- `OpenAPI` 这类生成 JSON 产物单独放到 `artifacts/`。
- 根目录 `README.md`、`KICKOFF.md` 与 `DEVELOPMENT_LOG.md` 分别承担项目总览、未完成工作索引和开发进度交接。
- Agent 续工不再从多个 handoff / checklist 里拼上下文，统一从 [`docs/agents/AGENT_START_HERE.md`](./agents/AGENT_START_HERE.md) 开始。

关于 `OpenAPI` 是否应和 markdown 分开：

- 不是硬性标准，但把生成产物和手写文档分开是很常见、也更稳妥的做法。
- 这样可以减少目录噪声，避免“叙述文档”和“可再生成产物”混在一起。
- 也更方便在 CI、提交检查和后续发布流程里单独处理机器产物。
