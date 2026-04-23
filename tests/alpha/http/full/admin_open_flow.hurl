GET {{base_url}}/api/healthz
HTTP 200
[Asserts]
jsonpath "$.status" == "ok"

POST {{base_url}}/api/auth/login
Content-Type: application/json
{
  "username": "{{admin_username}}",
  "password": "{{admin_password}}"
}
HTTP 200
[Captures]
admin_user_id: jsonpath "$.user.id"
[Asserts]
header "set-cookie" contains "mini_conf_session="
jsonpath "$.user.username" == "admin"

POST {{base_url}}/api/setup/complete
HTTP 200
[Asserts]
jsonpath "$.setup_required" == false
jsonpath "$.setup_completed_by_user_id" == {{admin_user_id}}

POST {{base_url}}/api/projects
Content-Type: application/json
{
  "code": "alpha-full-{{suite_id}}",
  "name": "Alpha Full {{suite_id}}",
  "description": "alpha full project",
  "initial_admin_user_id": {{admin_user_id}}
}
HTTP 201
[Captures]
project_id: jsonpath "$.project.id"
[Asserts]
jsonpath "$.project.status" == "active"
jsonpath "$.initial_admin.user_id" == {{admin_user_id}}

POST {{base_url}}/api/config-files
Content-Type: application/json
{
  "project_id": {{project_id}},
  "code": "main",
  "name": "Main Config",
  "is_required": true,
  "format": "yaml",
  "sensitivity": "normal",
  "description": "alpha full config"
}
HTTP 201
[Captures]
config_file_id: jsonpath "$.id"
[Asserts]
jsonpath "$.project_id" == {{project_id}}
jsonpath "$.code" == "main"
jsonpath "$.is_required" == true

POST {{base_url}}/api/projects/{{project_id}}/environments
Content-Type: application/json
{
  "code": "prod",
  "name": "Production",
  "status": "active",
  "sort_order": 10
}
HTTP 201
[Captures]
environment_id: jsonpath "$.id"
[Asserts]
jsonpath "$.project_id" == {{project_id}}
jsonpath "$.code" == "prod"
jsonpath "$.status" == "active"

POST {{base_url}}/api/deployment-instances
Content-Type: application/json
{
  "project_id": {{project_id}},
  "environment_id": {{environment_id}},
  "deployment_key": "template-{{suite_id}}",
  "name": "Template {{suite_id}}",
  "description": "alpha template deployment",
  "is_template": true
}
HTTP 201
[Captures]
template_id: jsonpath "$.id"
[Asserts]
jsonpath "$.is_template" == true
jsonpath "$.deployment_key" == "template-{{suite_id}}"

PUT {{base_url}}/api/drafts/{{template_id}}/{{config_file_id}}
Content-Type: application/json
{
  "content": "poll_interval_ms: 5000\n",
  "format": "yaml"
}
HTTP 200
[Asserts]
jsonpath "$.deployment_instance_id" == {{template_id}}
jsonpath "$.config_file_id" == {{config_file_id}}
jsonpath "$.version" == 1

POST {{base_url}}/api/deployment-instances/{{template_id}}/clone
Content-Type: application/json
{
  "deployment_key": "store-{{suite_id}}",
  "name": "Store {{suite_id}}",
  "environment_id": {{environment_id}},
  "description": "cloned from template",
  "clone_source": "draft"
}
HTTP 201
[Captures]
deployment_id: jsonpath "$.id"
[Asserts]
jsonpath "$.project_id" == {{project_id}}
jsonpath "$.deployment_key" == "store-{{suite_id}}"
jsonpath "$.template_source_id" == {{template_id}}
jsonpath "$.is_template" == false
jsonpath "$.status" == "inactive"

GET {{base_url}}/api/drafts/{{deployment_id}}/{{config_file_id}}
HTTP 200
[Asserts]
jsonpath "$.content" == "poll_interval_ms: 5000\n"
jsonpath "$.version" == 1

POST {{base_url}}/api/projects/{{project_id}}/members
Content-Type: application/json
{
  "username": "alice",
  "role": "viewer"
}
HTTP 201
[Captures]
alice_member_id: jsonpath "$.id"
[Asserts]
jsonpath "$.project_id" == {{project_id}}
jsonpath "$.username" == "alice"
jsonpath "$.role" == "viewer"

GET {{base_url}}/api/projects/{{project_id}}/members
HTTP 200
[Asserts]
jsonpath "$.items[0].username" == "admin"
jsonpath "$.items[0].role" == "admin"
jsonpath "$.items[1].username" == "alice"
jsonpath "$.items[1].role" == "viewer"

GET {{base_url}}/api/deployment-instances/{{deployment_id}}/preview-bundle
HTTP 200
[Asserts]
jsonpath "$.deployment_instance_id" == {{deployment_id}}
jsonpath "$.items[0].code" == "main"
jsonpath "$.items[0].status" == "ready"
jsonpath "$.open_bundle_preview.configs[0].config" == "main"

POST {{base_url}}/api/releases/publish
Content-Type: application/json
{
  "project_id": {{project_id}},
  "deployment_instance_id": {{deployment_id}},
  "config_file_id": {{config_file_id}},
  "change_summary": "initial alpha publish"
}
HTTP 201
[Captures]
release_id: jsonpath "$.id"
release_revision: jsonpath "$.revision"
[Asserts]
jsonpath "$.project_id" == {{project_id}}
jsonpath "$.deployment_instance_id" == {{deployment_id}}
jsonpath "$.config_file_id" == {{config_file_id}}
jsonpath "$.format" == "yaml"

GET {{base_url}}/api/releases?deployment_instance_id={{deployment_id}}&config_file_id={{config_file_id}}
HTTP 200
[Asserts]
jsonpath "$.items[0].id" == {{release_id}}
jsonpath "$.items[0].revision" == "{{release_revision}}"

GET {{base_url}}/api/releases/{{release_id}}
HTTP 200
[Asserts]
jsonpath "$.release.id" == {{release_id}}
jsonpath "$.release.revision" == "{{release_revision}}"
jsonpath "$.content" == "poll_interval_ms: 5000\n"
jsonpath "$.content_redacted" == false

GET {{base_url}}/api/releases/{{release_id}}/diff
HTTP 200
[Asserts]
jsonpath "$.release.id" == {{release_id}}
jsonpath "$.diff_summary.is_initial" == true
jsonpath "$.diff_summary.has_changes" == true
jsonpath "$.after_redacted" == false

POST {{base_url}}/api/deployment-instances/{{deployment_id}}/activate
HTTP 200
[Captures]
deployment_token_old: jsonpath "$.token"
[Asserts]
jsonpath "$.deployment_instance_id" == {{deployment_id}}
jsonpath "$.credential_name" == "default"
jsonpath "$.token_preview" == "mc_live_***"

GET {{base_url}}/api/open/configs/resolve?project=alpha-full-{{suite_id}}&environment=prod&deployment_key=store-{{suite_id}}&config=main
Authorization: Bearer {{deployment_token_old}}
HTTP 200
[Asserts]
header "etag" contains "\""
jsonpath "$.project" == "alpha-full-{{suite_id}}"
jsonpath "$.environment" == "prod"
jsonpath "$.deployment.key" == "store-{{suite_id}}"
jsonpath "$.config" == "main"
jsonpath "$.release.revision" == "{{release_revision}}"
jsonpath "$.fetch.url" == "/api/open/releases/{{release_revision}}"

GET {{base_url}}/api/open/releases/{{release_revision}}
Authorization: Bearer {{deployment_token_old}}
HTTP 200
[Asserts]
jsonpath "$.release.revision" == "{{release_revision}}"
jsonpath "$.deployment.project" == "alpha-full-{{suite_id}}"
jsonpath "$.deployment.deployment_key" == "store-{{suite_id}}"
jsonpath "$.config.name" == "main"
jsonpath "$.content" == "poll_interval_ms: 5000\n"

GET {{base_url}}/api/open/deployments/store-{{suite_id}}/config-bundle?project=alpha-full-{{suite_id}}&environment=prod
Authorization: Bearer {{deployment_token_old}}
HTTP 200
[Asserts]
jsonpath "$.project" == "alpha-full-{{suite_id}}"
jsonpath "$.deployment.key" == "store-{{suite_id}}"
jsonpath "$.configs[0].config" == "main"
jsonpath "$.configs[0].revision" == "{{release_revision}}"

POST {{base_url}}/api/open/deployment-sync-records
Authorization: Bearer {{deployment_token_old}}
Content-Type: application/json
{
  "project": "alpha-full-{{suite_id}}",
  "environment": "prod",
  "deployment_key": "store-{{suite_id}}",
  "config": "main",
  "action": "apply",
  "revision": "{{release_revision}}",
  "status": "success",
  "message": "alpha apply ok",
  "detail": {
    "duration_ms": 42
  },
  "reported_at": "2026-04-10T12:00:00Z"
}
HTTP 200
[Asserts]
jsonpath "$.ok" == true

POST {{base_url}}/api/open/heartbeats
Authorization: Bearer {{deployment_token_old}}
Content-Type: application/json
{
  "project": "alpha-full-{{suite_id}}",
  "environment": "prod",
  "deployment_key": "store-{{suite_id}}",
  "config": "main",
  "metadata": {
    "ip": "10.0.0.10",
    "version": "alpha"
  },
  "reported_at": "2026-04-10T12:01:00Z"
}
HTTP 200
[Asserts]
jsonpath "$.ok" == true

POST {{base_url}}/api/auth/login
Content-Type: application/json
{
  "username": "alice",
  "password": "alice123"
}
HTTP 200
[Asserts]
jsonpath "$.user.username" == "alice"

GET {{base_url}}/api/projects/{{project_id}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{project_id}}
jsonpath "$.code" == "alpha-full-{{suite_id}}"

GET {{base_url}}/api/deployment-sync-records?project_id={{project_id}}&deployment_instance_id={{deployment_id}}&config_file_id={{config_file_id}}
HTTP 200
[Asserts]
jsonpath "$.items[0].deployment_instance_id" == {{deployment_id}}
jsonpath "$.items[0].config_file_id" == {{config_file_id}}
jsonpath "$.items[0].config" == "main"
jsonpath "$.items[0].revision" == "{{release_revision}}"
jsonpath "$.items[0].status" == "success"

GET {{base_url}}/api/deployment-heartbeats?project_id={{project_id}}&deployment_instance_id={{deployment_id}}&config_file_id={{config_file_id}}
HTTP 200
[Asserts]
jsonpath "$.items[0].deployment_instance_id" == {{deployment_id}}
jsonpath "$.items[0].config_file_id" == {{config_file_id}}
jsonpath "$.items[0].config" == "main"
jsonpath "$.items[0].metadata.version" == "alpha"

GET {{base_url}}/api/audit-logs?project_id={{project_id}}
HTTP 403
[Asserts]
jsonpath "$.code" == "project_permission_denied"

POST {{base_url}}/api/auth/login
Content-Type: application/json
{
  "username": "{{admin_username}}",
  "password": "{{admin_password}}"
}
HTTP 200
[Asserts]
jsonpath "$.user.username" == "admin"

GET {{base_url}}/api/audit-logs?project_id={{project_id}}&action=project_member.created
HTTP 200
[Asserts]
jsonpath "$.items[0].action" == "project_member.created"
jsonpath "$.items[0].detail.username" == "alice"
jsonpath "$.items[0].detail.role" == "viewer"

PUT {{base_url}}/api/drafts/{{deployment_id}}/{{config_file_id}}
Content-Type: application/json
{
  "content": "poll_interval_ms: 7000\nlog_level: info\n",
  "format": "yaml",
  "base_version": 1
}
HTTP 200
[Asserts]
jsonpath "$.deployment_instance_id" == {{deployment_id}}
jsonpath "$.version" == 2

POST {{base_url}}/api/releases/publish
Content-Type: application/json
{
  "project_id": {{project_id}},
  "deployment_instance_id": {{deployment_id}},
  "config_file_id": {{config_file_id}},
  "change_summary": "raise alpha interval"
}
HTTP 201
[Captures]
release_id_2: jsonpath "$.id"
release_revision_2: jsonpath "$.revision"
[Asserts]
jsonpath "$.deployment_instance_id" == {{deployment_id}}
jsonpath "$.config_file_id" == {{config_file_id}}

GET {{base_url}}/api/releases/{{release_id_2}}/diff
HTTP 200
[Asserts]
jsonpath "$.release.id" == {{release_id_2}}
jsonpath "$.base_release.id" == {{release_id}}
jsonpath "$.before_content" == "poll_interval_ms: 5000\n"
jsonpath "$.after_content" == "poll_interval_ms: 7000\nlog_level: info\n"
jsonpath "$.diff_summary.is_initial" == false
jsonpath "$.diff_summary.has_changes" == true

POST {{base_url}}/api/deployment-instances/{{deployment_id}}/token/reset
HTTP 200
[Captures]
deployment_token_new: jsonpath "$.token"
[Asserts]
jsonpath "$.deployment_instance_id" == {{deployment_id}}
jsonpath "$.token_preview" == "mc_live_***"

GET {{base_url}}/api/open/configs/resolve?project=alpha-full-{{suite_id}}&environment=prod&deployment_key=store-{{suite_id}}&config=main
Authorization: Bearer {{deployment_token_old}}
HTTP 401
[Asserts]
jsonpath "$.code" == "invalid_token"

GET {{base_url}}/api/open/configs/resolve?project=alpha-full-{{suite_id}}&environment=prod&deployment_key=store-{{suite_id}}&config=main
Authorization: Bearer {{deployment_token_new}}
HTTP 200
[Asserts]
jsonpath "$.release.revision" == "{{release_revision_2}}"

GET {{base_url}}/api/audit-logs?project_id={{project_id}}&action=release.published
HTTP 200
[Asserts]
jsonpath "$.items[0].action" == "release.published"

GET {{base_url}}/api/audit-logs?project_id={{project_id}}&action=deployment_token.reset
HTTP 200
[Asserts]
jsonpath "$.items[0].action" == "deployment_token.reset"
