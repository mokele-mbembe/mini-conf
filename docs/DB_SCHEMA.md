# 数据库表设计草案

## 1. 设计目标

- 支持多项目
- 支持每个项目多份配置文件
- 支持每个项目在多环境下创建多份独立部署实例
- 支持部署实例模板克隆
- 支持 Draft / Release
- 支持部署实例级共享凭证和同步记录
- 支持审计与项目级权限
- 兼容 PostgreSQL 16+
- 适合 SQLx migrations 管理

## 2. 表清单

- `users`
- `projects`
- `project_members`
- `config_files`
- `deployment_instances`
- `drafts`
- `releases`
- `deployment_credentials`
- `deployment_sync_records`
- `audit_logs`

## 3. 核心表说明

### users

- `id` bigserial pk
- `username` varchar(64) not null unique
- `password_hash` varchar(255) not null
- `status` varchar(32) not null default 'active'
- `created_at` timestamptz not null default now()
- `updated_at` timestamptz not null default now()

### projects

- `id` bigserial pk
- `code` varchar(64) not null unique
- `name` varchar(128) not null
- `description` text null
- `status` varchar(32) not null default 'active'
- `created_at` timestamptz not null default now()
- `updated_at` timestamptz not null default now()

### project_members

- `id` bigserial pk
- `project_id` bigint not null
- `user_id` bigint not null
- `role` varchar(32) not null
- `created_at` timestamptz not null default now()
- unique (`project_id`, `user_id`)

说明：

- 首版角色只保留 `admin`、`editor`、`viewer`
- 权限判断以项目成员关系为主，而不是全局角色

### config_files

- `id` bigserial pk
- `project_id` bigint not null
- `code` varchar(64) not null
- `name` varchar(128) not null
- `format` varchar(16) not null
- `schema_name` varchar(128) null
- `schema_version` varchar(64) null
- `sensitivity` varchar(16) not null default 'normal'
- `secret_paths` jsonb null
- `description` text null
- `status` varchar(32) not null default 'active'
- `created_at` timestamptz not null default now()
- `updated_at` timestamptz not null default now()
- unique (`project_id`, `code`)

说明：

- `sensitivity` 首版可取 `normal` 或 `secret`
- `secret_paths` 用于记录需要脱敏显示的字段路径
- MVP 先做脱敏展示和日志裁剪，不强制要求字段级加密存储

### deployment_instances

- `id` bigserial pk
- `project_id` bigint not null
- `environment` varchar(32) not null
- `deployment_key` varchar(64) not null
- `name` varchar(128) not null
- `description` text null
- `is_template` boolean not null default false
- `template_source_id` bigint null
- `status` varchar(32) not null default 'active'
- `created_at` timestamptz not null default now()
- `updated_at` timestamptz not null default now()
- unique (`project_id`, `environment`, `deployment_key`)

说明：

- `DeploymentInstance` 是 MVP 主路径中的核心对象
- 它代表项目在某个环境下的一整套独立部署实例
- 可以从一个模板部署实例克隆，但克隆完成后保持独立

### drafts

- `id` bigserial pk
- `project_id` bigint not null
- `config_file_id` bigint not null
- `deployment_instance_id` bigint not null
- `content` text not null
- `content_hash` char(64) not null
- `format` varchar(16) not null
- `schema_version` varchar(64) null
- `version` bigint not null default 1
- `editor_user_id` bigint not null
- `updated_at` timestamptz not null default now()
- unique (`config_file_id`, `deployment_instance_id`)

说明：

- 一个部署实例下，每个配置文件只有一个当前 Draft
- `version` 用于乐观锁控制
- 保存 Draft 时必须带上当前版本号，服务端比对失败返回 `409 Conflict`

### releases

- `id` bigserial pk
- `project_id` bigint not null
- `config_file_id` bigint not null
- `deployment_instance_id` bigint not null
- `revision` varchar(64) not null
- `content` text not null
- `content_hash` char(64) not null
- `format` varchar(16) not null
- `change_summary` varchar(255) null
- `diff_summary` jsonb null
- `apply_mode` varchar(16) not null
- `published_by` bigint not null
- `published_at` timestamptz not null default now()
- unique (`deployment_instance_id`, `config_file_id`, `revision`)

说明：

- 同一份 Draft 内容重复发布时，允许生成新的 `revision`
- Release 不可修改，回滚通过重新发布旧内容实现

### deployment_credentials

- `id` bigserial pk
- `deployment_instance_id` bigint not null
- `credential_name` varchar(64) not null default 'default'
- `token_hash` varchar(255) not null
- `status` varchar(32) not null default 'active'
- `last_used_at` timestamptz null
- `created_at` timestamptz not null default now()
- `updated_at` timestamptz not null default now()
- unique (`deployment_instance_id`, `credential_name`)

说明：

- MVP 可先只启用每个部署实例一份默认凭证
- 一个部署实例上的多个进程可以共享同一份凭证访问平台

### deployment_sync_records

- `id` bigserial pk
- `project_id` bigint not null
- `deployment_instance_id` bigint not null
- `config_file_id` bigint null
- `release_id` bigint null
- `process_key` varchar(64) null
- `revision` varchar(64) null
- `action` varchar(32) not null
- `status` varchar(32) not null
- `message` varchar(255) null
- `detail` jsonb null
- `reported_at` timestamptz not null default now()
- index (`deployment_instance_id`, `reported_at`)
- index (`release_id`, `reported_at`)

说明：

- `process_key` 用于区分同一部署实例上的不同进程
- 例如 `main`、`ad-screen`、`vision`

### audit_logs

- `id` bigserial pk
- `project_id` bigint null
- `user_id` bigint null
- `action` varchar(64) not null
- `resource_type` varchar(64) not null
- `resource_id` varchar(64) not null
- `detail` jsonb null
- `created_at` timestamptz not null default now()

说明：

- `detail` 中不得写入敏感配置明文
- 如需记录差异，应该记录脱敏后的摘要

## 4. 外键建议

第一版可以先对核心关系加外键，其余关系以索引为主，保持迁移简洁。

建议优先加这些：

- `project_members.project_id -> projects.id`
- `project_members.user_id -> users.id`
- `config_files.project_id -> projects.id`
- `deployment_instances.project_id -> projects.id`
- `deployment_instances.template_source_id -> deployment_instances.id`
- `drafts.config_file_id -> config_files.id`
- `drafts.deployment_instance_id -> deployment_instances.id`
- `releases.config_file_id -> config_files.id`
- `releases.deployment_instance_id -> deployment_instances.id`
- `deployment_credentials.deployment_instance_id -> deployment_instances.id`
- `deployment_sync_records.deployment_instance_id -> deployment_instances.id`

## 5. 部署实例与模板规则

首版建议：

1. 一个项目可以创建多个模板部署实例
2. 模板部署实例本质上仍然是部署实例，只是 `is_template = true`
3. 新部署实例可以从模板部署实例克隆全部 Draft 或最新 Release 内容
4. 克隆完成后，新部署实例与模板不再联动

这样做的原因：

- 满足“多份部署实例复制”的业务诉求
- 避免第一版就引入复杂继承链

## 6. revision 规则

建议格式：

- `20260405.0001`
- `20260405.0002`

生成规则：

- 发布当天日期 + 4 位序号
- 同一个 `deployment_instance + config_file` 下唯一
- 同时保存 `content_hash`
- 即使内容相同，重复发布也生成新 `revision`

## 7. apply_mode 规则

只保留两种：

- `soft`
- `hard`

判定来源：

- 发布时根据字段变更规则自动计算
- 结果写入 `releases.apply_mode`
- 平台给出建议，最终如何生效由消费端自己决定

## 8. 初始索引重点

- `projects.code`
- `project_members(project_id, user_id)`
- `config_files(project_id, code)`
- `deployment_instances(project_id, environment, deployment_key)`
- `releases(deployment_instance_id, config_file_id, published_at desc)`
- `deployment_credentials(deployment_instance_id, status)`
- `deployment_sync_records(deployment_instance_id, reported_at desc)`

## 9. 关于 Scope / Labels

根据当前业务需求，MVP 主路径不依赖动态 `Scope` 或客户端 `labels`。

原因：

- 你的实际使用方式更接近“明确部署实例 + 多配置文件 + 共享凭证”
- 这比让客户端动态声明标签更直观，也更安全

后续如果开源演进需要：

- 可以在 `DeploymentInstance` 之上扩展 `Scope` 和 `labels`
- 用于支持动态分群、灰度和自动匹配

## 10. 敏感配置最小安全方案

MVP 先采用轻量方案：

1. 允许 `ConfigFile` 声明 `sensitivity`
2. 对敏感配置默认脱敏展示
3. `audit_logs`、请求日志、错误日志禁止记录敏感明文
4. Draft / Release 仍按文本存储，字段级加密存储放到后续版本

## 11. 第一批迁移顺序

1. `users`
2. `projects`
3. `project_members`
4. `config_files`
5. `deployment_instances`
6. `drafts`
7. `releases`
8. `deployment_credentials`
9. `deployment_sync_records`
10. `audit_logs`
