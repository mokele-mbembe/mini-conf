# 0005 Project Members / 权限 / 审计 澄清

## 背景

当前仓库已经有完整的管理端 session 登录和主要资源 API，但权限判断仍停留在“只要登录就是管理员”的阶段。接下来需要把它收口到项目成员模型，并补上管理端审计与同步记录查询。

## Q1: 项目对谁可见？

本轮固定为：

- 项目仅对成员可见
- `GET /api/projects` 只返回当前用户参与的项目
- `GET /api/projects/:id` 与项目下其他资源详情，对非成员按未命中处理

也就是说，不再保留“所有已登录用户都能看到全部项目”的过渡语义。

## Q2: 谁可以创建项目？

本轮固定为：

- 任意已登录用户都可以创建项目
- 创建成功后，创建者自动成为该项目 `admin`

这样项目创建与后续成员管理的责任边界一致，不依赖一个长期存在的全局超管才能开展日常业务。

## Q3: 项目成员如何绑定用户？

本轮固定为：

- 成员接口按 `username` 绑定已存在用户
- 不通过成员接口顺带创建用户
- 目标用户必须存在且 `status = active`

这样可以把“用户创建/导入”继续留在 MVP 之外，不把本轮范围扩散成完整用户管理系统。

## Q4: 首版角色边界是什么？

本轮固定保持：

- `admin`
- `editor`
- `viewer`

权限收口如下：

- `admin / editor / viewer` 都能看项目、配置文件、部署实例、release、同步记录
- `admin / editor` 能读写 Draft、clone Draft、预览实例整包、发布 release
- 只有 `admin` 能改项目、管理项目成员、管理配置文件、管理部署实例、clone 实例、reset token、查看 audit logs

## Q5: 非成员和低权限成员分别返回什么？

本轮固定为：

- 非成员访问项目资源时，返回当前资源自己的 `404`
- 已是成员但角色不够时，返回 `403 project_permission_denied`

这样既保留当前 API 的资源未命中风格，也能明确区分“资源存在但你不能做”。

## Q6: 历史项目如何迁移到成员模型？

本轮固定为：

- 新增 `project_members` 迁移时，将已有项目统一回填给活动用户 `admin`，角色为 `admin`
- 不做“项目没有成员时暂时放行”的兼容分支

这与当前仓库在成员模型上线前主要通过默认 `admin` 用户操作的现实一致。

## Q7: 审计日志本轮做到什么程度？

本轮固定做到：

- 关键事件落表到 `audit_logs`
- 提供管理端 `GET /api/audit-logs`

必须记录的事件包括：

- 登录成功 / 失败
- 项目创建 / 修改
- 项目成员创建 / 更新 / 删除
- 配置文件创建 / 修改
- 部署实例创建 / 修改 / clone
- Draft 保存 / clone
- Release 发布
- Deployment token reset

## Q8: 审计 detail 能记录什么？

本轮固定要求：

- 允许记录安全元数据，例如 `project_id`、`deployment_instance_id`、`config_file_id`、`revision`、`username`、`role`、`changed_fields`、`source_kind`、`token_preview`
- 禁止记录 Draft/Release 明文、原始 token、secret 明文、完整 diff 文本

实现上即使误传了这些字段，也应该在落库前做裁剪。

## 当前建议

按这个顺序推进实现：

1. 补 `project_members` 和 `audit_logs` 迁移
2. 先把 `POST /api/projects` 改成自动写成员与审计
3. 再把现有管理端接口统一收口到项目角色判断
4. 最后补成员接口、审计查询和同步记录管理端查询
