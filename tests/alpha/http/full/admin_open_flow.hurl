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
[Asserts]
header "set-cookie" contains "mini_conf_session="
jsonpath "$.user.username" == "admin"

POST {{base_url}}/api/projects
Content-Type: application/json
{
  "code": "alpha-full-{{suite_id}}",
  "name": "Alpha Full {{suite_id}}",
  "description": "alpha full project"
}
HTTP 201
[Captures]
project_id: jsonpath "$.id"
[Asserts]
jsonpath "$.status" == "active"

POST {{base_url}}/api/config-files
Content-Type: application/json
{
  "project_id": {{project_id}},
  "code": "main",
  "name": "Main Config",
  "is_required": true,
  "format": "yaml",
  "schema_name": "alpha-main",
  "schema_version": "v1",
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

POST {{base_url}}/api/deployment-instances
Content-Type: application/json
{
  "project_id": {{project_id}},
  "environment": "prod",
  "deployment_key": "store-{{suite_id}}",
  "name": "Store {{suite_id}}",
  "description": "alpha full deployment",
  "is_template": false
}
HTTP 201
[Captures]
deployment_id: jsonpath "$.id"
[Asserts]
jsonpath "$.project_id" == {{project_id}}
jsonpath "$.deployment_key" == "store-{{suite_id}}"
jsonpath "$.environment" == "prod"

PUT {{base_url}}/api/drafts/{{deployment_id}}/{{config_file_id}}
Content-Type: application/json
{
  "content": "poll_interval_ms: 5000\n",
  "format": "yaml"
}
HTTP 200
[Asserts]
jsonpath "$.deployment_instance_id" == {{deployment_id}}
jsonpath "$.config_file_id" == {{config_file_id}}
jsonpath "$.version" == 1
jsonpath "$.schema_version" == "v1"

GET {{base_url}}/api/drafts/{{deployment_id}}/{{config_file_id}}
HTTP 200
[Asserts]
jsonpath "$.content" == "poll_interval_ms: 5000\n"
jsonpath "$.version" == 1

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
  "change_summary": "alpha publish"
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

GET {{base_url}}/api/releases/{{release_id}}/diff
HTTP 200
[Asserts]
jsonpath "$.release.id" == {{release_id}}
jsonpath "$.diff_summary.is_initial" == true
jsonpath "$.diff_summary.has_changes" == true

POST {{base_url}}/api/deployment-instances/{{deployment_id}}/token/reset
HTTP 200
[Captures]
deployment_token: jsonpath "$.token"
[Asserts]
jsonpath "$.deployment_instance_id" == {{deployment_id}}
jsonpath "$.credential_name" == "default"
jsonpath "$.token_preview" == "mc_live_***"

GET {{base_url}}/api/open/configs/resolve?project=alpha-full-{{suite_id}}&environment=prod&deployment_key=store-{{suite_id}}&config=main
Authorization: Bearer {{deployment_token}}
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
Authorization: Bearer {{deployment_token}}
HTTP 200
[Asserts]
jsonpath "$.release.revision" == "{{release_revision}}"
jsonpath "$.deployment.project" == "alpha-full-{{suite_id}}"
jsonpath "$.deployment.deployment_key" == "store-{{suite_id}}"
jsonpath "$.config.name" == "main"
jsonpath "$.content" == "poll_interval_ms: 5000\n"

GET {{base_url}}/api/open/deployments/store-{{suite_id}}/config-bundle?project=alpha-full-{{suite_id}}&environment=prod
Authorization: Bearer {{deployment_token}}
HTTP 200
[Asserts]
jsonpath "$.project" == "alpha-full-{{suite_id}}"
jsonpath "$.deployment.key" == "store-{{suite_id}}"
jsonpath "$.configs[0].config" == "main"
jsonpath "$.configs[0].revision" == "{{release_revision}}"

POST {{base_url}}/api/open/deployment-sync-records
Authorization: Bearer {{deployment_token}}
Content-Type: application/json
{
  "project": "alpha-full-{{suite_id}}",
  "environment": "prod",
  "deployment_key": "store-{{suite_id}}",
  "config": "main",
  "process_key": "main",
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
Authorization: Bearer {{deployment_token}}
Content-Type: application/json
{
  "project": "alpha-full-{{suite_id}}",
  "environment": "prod",
  "deployment_key": "store-{{suite_id}}",
  "process_key": "main",
  "metadata": {
    "ip": "10.0.0.10",
    "version": "alpha"
  },
  "reported_at": "2026-04-10T12:01:00Z"
}
HTTP 200
[Asserts]
jsonpath "$.ok" == true
