# 鉴权与安全设计草案

## 1. 文档目标

这份文档定义 `mini-conf` 在 MVP 阶段的鉴权、权限和安全边界。

目标是：

- 管理端和消费端采用不同的鉴权策略
- 权限模型足够轻量，但能覆盖 MVP
- 避免把安全复杂度做得过高，阻塞首版交付
- 为后续开源和私有部署预留扩展空间
- 对敏感配置提供最小但明确的安全边界

## 2. 安全设计原则

- 管理端与消费端分离鉴权
- 密码和 token 只存 hash，不存明文
- 默认最小权限
- 发布和鉴权相关操作必须留审计日志
- 首版只做必要安全，不引入过重 IAM 体系
- 敏感配置默认脱敏展示与日志裁剪

## 3. 两类主体

系统里有两类主要主体：

### 1. 管理端用户

用于：

- 登录后台
- 编辑配置
- 发布 Release
- 查看同步记录和审计日志

### 2. 消费端部署实例

用于：

- 检查当前版本
- 拉取配置
- 回传同步结果
- 上报心跳

## 4. 管理端鉴权

设计上支持两种方案：

- Session Cookie
- JWT

MVP 只完整实现：

- Session Cookie

原因：

- 管理端本质上是浏览器后台
- Session 更适合首版后台管理场景
- 比 JWT 更容易先把登录态失效、登出和权限校验做好

实现约束：

- Session Cookie 设置为 `HttpOnly`
- HTTPS 场景启用 `Secure`
- 适当设置 `SameSite=Lax` 或 `Strict`
- 基础安全响应头默认开启
- 登录失败需要基础节流，避免密码撞库直接打满

后续路线：

- 在 release note 中明确当前版本只完整支持 Session 模式
- 后续版本补齐 JWT 认证与 OAuth 2.0 接入

## 5. 消费端鉴权

首版建议默认使用 Bearer Token：

```http
Authorization: Bearer <token>
```

token 特点：

- 按部署实例签发
- 同一部署实例上的多个进程可共享一份 token
- 可手动重置
- 可吊销
- 数据库只存 hash
- token 是鉴权凭证，不是唯一寻址参数；Open API 仍要求请求携带 `project / environment / deployment_key`
- 服务端应校验 token 所属部署实例与请求中的 `deployment_key` 一致

## 6. 密码与 token 存储

密码存储建议：

- 使用 Argon2id

token 存储建议：

- 明文 token 仅在创建或重置时显示一次
- 数据库存储 token hash
- 比较时走常量时间比较

## 7. 权限模型

当前权限模型分两层：

- 平台层：`platform_admin`
- 项目层：`admin / editor / viewer`

`platform_admin` 负责系统初始化、用户管理、平台项目创建和平台级审计；它默认不自动拥有任何项目业务数据可见性。

项目业务权限仍以项目级成员关系为主。

项目角色只保留：

- `admin`
- `editor`
- `viewer`

### admin

可以：

- 查看项目
- 修改项目
- 管理项目成员
- 管理配置文件
- 管理部署实例
- 克隆部署实例
- 编辑 Draft
- 发布 Release
- 重置部署实例 token
- 查看审计日志

### editor

可以：

- 查看项目、配置文件、部署实例
- 编辑 Draft
- 预览实例整包配置
- 发布 Release
- 查看同步记录

不可以：

- 修改项目
- 管理项目成员
- 管理配置文件
- 管理部署实例
- 重置高风险凭证

### viewer

可以：

- 查看项目、配置文件、部署实例、Release、同步记录

不可以：

- 编辑 Draft
- 预览实例整包配置
- 发布 Release
- 重置 token

## 8. 初始化管理员

首版允许通过环境变量初始化默认平台管理员：

- `INIT_ADMIN_USERNAME`
- `INIT_ADMIN_PASSWORD`

说明：

- 这个管理员主要用于系统初始化、用户管理和创建首批项目壳
- 该用户默认不自动拥有项目业务数据可见性
- 创建项目时必须指定首个项目 `admin`
- 日常业务权限仍以项目级成员关系为主
- 已有历史项目在引入 `project_members` 时曾回填给活动用户 `admin` 作为项目 `admin`；新建项目不再依赖该历史语义

Setup 状态由 `system_settings` 记录。setup 未完成时，业务接口应被 `setup_required` 阻断，仅保留认证、健康检查、setup 和平台初始化相关接口。

## 9. 审计日志要求

以下操作必须记录审计日志：

- 登录成功
- 登录失败
- 用户创建、禁用、启用、重置密码
- setup 完成
- 项目创建和修改
- 项目成员变更
- 配置文件创建和修改
- 部署实例创建、修改和克隆
- Draft 保存
- Draft 克隆
- Release 发布
- 部署实例 token 重置

额外约束：

- 审计日志和 tracing 日志不得记录敏感配置明文
- 如需记录配置差异，应记录脱敏后的摘要或字段路径
- `detail` 只允许记录安全元数据，例如 `project_id`、`deployment_instance_id`、`deployment_uid`、`deployment_key` 快照、`config_file_id`、`revision`、`username`、`role`、`changed_fields`、`source_kind`、`token_preview`

## 10. 开放接口安全要求

消费端开放接口至少应满足：

- Bearer Token 校验
- 基础限流
- 请求日志
- 审计关键失败事件

当前实现状态：

- Bearer Token 校验已落地。
- 基础 HTTP tracing 已接入。
- 基础限流和关键失败事件留痕仍是上线前缺口。

首版建议限流维度：

- 按 IP
- 按 `deployment_key`
- 按 token hash

## 11. 敏感配置最小安全方案

MVP 阶段建议这样落地：

- `ConfigFile` 可以标记 `sensitivity=secret`
- 可选记录 `secret_paths`，用于指定需要脱敏的字段路径
- 管理端查看和 Diff 展示时默认脱敏
- 普通日志、错误日志、审计日志不得输出敏感明文

这版不强制要求：

- 字段级加密存储
- 外部 KMS 集成
- 密钥轮换

这些能力会留到后续版本补齐。

## 12. Session 与 Token 生命周期

管理端 Session 建议：

- 空闲超时：8 小时
- 绝对过期：7 天

消费端 Token 建议：

- 默认长期有效
- 通过管理端手动重置

原因：

- IoT 和边缘节点的运维现实里，强制短周期轮换会显著提高复杂度

## 13. CSRF、XSS 与基础防护

管理端首版建议做到：

- Session 登录接口启用 CSRF 防护（已落地）
- 已登录会话的写操作使用 CSRF cookie + `X-CSRF-Token` header 校验（已落地）
- 所有输出默认按文本处理
- 编辑内容不直接作为 HTML 渲染
- 设置基础安全响应头（已落地；CSP / HSTS 是否纳入 MVP 仍需取舍）
- 登录失败节流（已落地）

## 14. 发布安全

发布相关接口应满足：

- 只有项目 `admin` 和 `editor` 可发布
- 发布前必须完成服务端校验
- 发布请求必须记录操作者和变更说明
- Release 发布后不可变更内容

回滚建议：

- 不直接修改旧 Release
- 通过“重新发布某旧版本内容”为新 revision 的方式实现回滚

## 15. 这版设计的取舍

这套设计不是追求“最完整”，而是追求：

- 足够安全
- 足够简单
- 足够适合你的 MVP

所以这版刻意没有引入：

- 细粒度 RBAC
- 多租户权限隔离
- OIDC / OAuth 登录
- 强制短周期 token 轮换
- 复杂客户端标签覆盖机制
- 字段级加密存储
- 外部 KMS 集成

## 16. 与 labels / Scope 的关系

根据当前业务需求，MVP 不把客户端 `labels` 或动态 `Scope` 作为主路径能力。

原因：

- 你的核心用法是“明确部署实例 + 多配置文件 + 共享凭证”
- 这比让客户端动态声明标签更直观，也更安全

后续如果开源演进需要：

- 可以在部署实例模型上增加动态 Scope 与 labels 能力
- 用于支持灰度、自动匹配和大规模分群
