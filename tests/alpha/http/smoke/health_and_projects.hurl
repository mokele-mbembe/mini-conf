GET {{base_url}}/api/healthz
HTTP 200
[Asserts]
header "content-type" contains "application/json"
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
jsonpath "$.auth_mode" == "session"

POST {{base_url}}/api/setup/complete
HTTP 200
[Asserts]
jsonpath "$.setup_required" == false
jsonpath "$.setup_completed_by_user_id" == {{admin_user_id}}

POST {{base_url}}/api/projects
Content-Type: application/json
{
  "code": "alpha-smoke-{{suite_id}}",
  "name": "Alpha Smoke {{suite_id}}",
  "description": "alpha smoke project",
  "initial_admin_user_id": {{admin_user_id}}
}
HTTP 201
[Captures]
project_id: jsonpath "$.project.id"
[Asserts]
jsonpath "$.project.code" == "alpha-smoke-{{suite_id}}"
jsonpath "$.project.status" == "active"
jsonpath "$.initial_admin.user_id" == {{admin_user_id}}

GET {{base_url}}/api/projects/{{project_id}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{project_id}}
jsonpath "$.code" == "alpha-smoke-{{suite_id}}"
jsonpath "$.name" == "Alpha Smoke {{suite_id}}"
jsonpath "$.status" == "active"
