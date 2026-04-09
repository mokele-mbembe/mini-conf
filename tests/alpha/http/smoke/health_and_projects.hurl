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
[Asserts]
header "set-cookie" contains "mini_conf_session="
jsonpath "$.user.username" == "admin"
jsonpath "$.auth_mode" == "session"

POST {{base_url}}/api/projects
Content-Type: application/json
{
  "code": "alpha-smoke-{{suite_id}}",
  "name": "Alpha Smoke {{suite_id}}",
  "description": "alpha smoke project"
}
HTTP 201
[Captures]
project_id: jsonpath "$.id"
[Asserts]
jsonpath "$.code" == "alpha-smoke-{{suite_id}}"
jsonpath "$.status" == "active"

GET {{base_url}}/api/projects/{{project_id}}
HTTP 200
[Asserts]
jsonpath "$.id" == {{project_id}}
jsonpath "$.code" == "alpha-smoke-{{suite_id}}"
jsonpath "$.name" == "Alpha Smoke {{suite_id}}"
jsonpath "$.status" == "active"
