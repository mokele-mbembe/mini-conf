#!/usr/bin/env bash
set -euo pipefail

base_url="${STAGING_BASE_URL:-${MINI_CONF_BASE_URL:-}}"
allow_http="${STAGING_ALLOW_HTTP:-0}"
dry_run="${STAGING_SMOKE_DRY_RUN:-0}"
connect_timeout="${STAGING_SMOKE_CONNECT_TIMEOUT:-5}"
max_time="${STAGING_SMOKE_MAX_TIME:-20}"

usage() {
  cat >&2 <<'EOF'
usage: STAGING_BASE_URL=https://config-center.example.com just staging-smoke

Environment:
  STAGING_BASE_URL               Required base URL for the deployed service.
  STAGING_ALLOW_HTTP=1           Allow http:// targets for temporary internal checks.
  STAGING_SMOKE_DRY_RUN=1        Validate inputs and print planned checks without network calls.
  STAGING_SMOKE_CONNECT_TIMEOUT  curl connect timeout in seconds, default 5.
  STAGING_SMOKE_MAX_TIME         curl max time in seconds, default 20.
EOF
}

if [[ -z "${base_url}" ]]; then
  usage
  exit 1
fi

base_url="${base_url%/}"

if [[ "${base_url}" != https://* && "${allow_http}" != "1" ]]; then
  echo "STAGING_BASE_URL must use https:// unless STAGING_ALLOW_HTTP=1 is set" >&2
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required for staging smoke checks" >&2
  exit 1
fi

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/mini-conf-staging-smoke.XXXXXX")"
cleanup() {
  rm -rf "${tmp_dir}"
}
trap cleanup EXIT

curl_common=(
  --silent
  --show-error
  --connect-timeout "${connect_timeout}"
  --max-time "${max_time}"
)

request() {
  local method="$1"
  local path="$2"
  local headers_file="$3"
  local body_file="$4"
  local status_file="$5"

  local status
  status="$(
    curl "${curl_common[@]}" \
      --request "${method}" \
      --dump-header "${headers_file}" \
      --output "${body_file}" \
      --write-out '%{http_code}' \
      "${base_url}${path}"
  )"
  printf '%s' "${status}" >"${status_file}"
}

assert_status() {
  local status_file="$1"
  local expected="$2"
  local context="$3"
  local actual
  actual="$(cat "${status_file}")"

  if [[ "${actual}" != "${expected}" ]]; then
    echo "${context}: expected HTTP ${expected}, got ${actual}" >&2
    return 1
  fi
}

assert_header() {
  local headers_file="$1"
  local header_name="$2"
  local expected_value="$3"
  local context="$4"

  if ! awk -v name="${header_name}" -v expected="${expected_value}" '
    BEGIN { found = 0 }
    {
      line = $0
      sub(/\r$/, "", line)
      split(line, parts, ":")
      key = tolower(parts[1])
      if (key == tolower(name)) {
        value = substr(line, length(parts[1]) + 2)
        sub(/^ /, "", value)
        if (value == expected) {
          found = 1
        }
      }
    }
    END { exit found ? 0 : 1 }
  ' "${headers_file}"; then
    echo "${context}: missing or unexpected ${header_name}: ${expected_value}" >&2
    echo "response headers:" >&2
    cat "${headers_file}" >&2
    return 1
  fi
}

assert_header_present() {
  local headers_file="$1"
  local header_name="$2"
  local context="$3"

  if ! awk -v name="${header_name}" '
    {
      line = $0
      sub(/\r$/, "", line)
      split(line, parts, ":")
      if (tolower(parts[1]) == tolower(name)) {
        found = 1
      }
    }
    END { exit found ? 0 : 1 }
  ' "${headers_file}"; then
    echo "${context}: missing ${header_name}" >&2
    echo "response headers:" >&2
    cat "${headers_file}" >&2
    return 1
  fi
}

assert_body_contains() {
  local body_file="$1"
  local pattern="$2"
  local context="$3"

  if ! grep -Fq "${pattern}" "${body_file}"; then
    echo "${context}: response body does not contain ${pattern}" >&2
    echo "response body:" >&2
    cat "${body_file}" >&2
    return 1
  fi
}

if [[ "${dry_run}" == "1" ]]; then
  echo "staging smoke dry run:"
  echo "- base URL: ${base_url}"
  echo "- GET /api/healthz expects 200"
  echo "- GET / expects 200"
  echo "- GET /api/open/configs/resolve without bearer token expects 401"
  echo "- security headers are checked on /api/healthz"
  exit 0
fi

health_headers="${tmp_dir}/health.headers"
health_body="${tmp_dir}/health.body"
health_status="${tmp_dir}/health.status"
request GET /api/healthz "${health_headers}" "${health_body}" "${health_status}"
assert_status "${health_status}" 200 "health check"
assert_body_contains "${health_body}" '"status":"ok"' "health check"

assert_header "${health_headers}" "x-content-type-options" "nosniff" "security headers"
assert_header "${health_headers}" "x-frame-options" "DENY" "security headers"
assert_header "${health_headers}" "cross-origin-resource-policy" "same-origin" "security headers"
assert_header_present "${health_headers}" "content-security-policy" "security headers"
assert_header_present "${health_headers}" "strict-transport-security" "security headers"

root_headers="${tmp_dir}/root.headers"
root_body="${tmp_dir}/root.body"
root_status="${tmp_dir}/root.status"
request GET / "${root_headers}" "${root_body}" "${root_status}"
assert_status "${root_status}" 200 "static frontend"
assert_body_contains "${root_body}" '<div id="app">' "static frontend"

open_headers="${tmp_dir}/open.headers"
open_body="${tmp_dir}/open.body"
open_status="${tmp_dir}/open.status"
request GET '/api/open/configs/resolve?project=smoke&environment=prod&deployment_key=smoke&config=main' "${open_headers}" "${open_body}" "${open_status}"
assert_status "${open_status}" 401 "open api unauthorized check"
assert_body_contains "${open_body}" '"code":"missing_token"' "open api unauthorized check"

echo "staging smoke passed: ${base_url}"
