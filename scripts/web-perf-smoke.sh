#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
database_url="${TEST_DATABASE_URL:-${DATABASE_URL:-}}"
schema="${PERF_WEB_SCHEMA:-mini_conf_web_perf_$(date +%s%N)}"
result_file="${PERF_WEB_RESULT_FILE:-${repo_root}/target/perf/web-route.json}"
ready_timeout_sec="${PERF_WEB_READY_TIMEOUT_SEC:-180}"
admin_username="${PERF_WEB_ADMIN_USERNAME:-admin}"
admin_password="${PERF_WEB_ADMIN_PASSWORD:-admin123456}"
cargo_target_dir="${CARGO_TARGET_DIR:-target}"
server_bin="${cargo_target_dir%/}/release/server"

if [[ -z "${database_url}" ]]; then
  echo "TEST_DATABASE_URL or DATABASE_URL is required for web performance smoke" >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required for web performance smoke" >&2
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required for web performance smoke" >&2
  exit 1
fi

if ! command -v pnpm >/dev/null 2>&1; then
  echo "pnpm is required for web performance smoke" >&2
  exit 1
fi

if ! command -v psql >/dev/null 2>&1; then
  echo "psql is required for web performance smoke" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required for web performance smoke" >&2
  exit 1
fi

if [[ ! "${schema}" =~ ^[a-zA-Z_][a-zA-Z0-9_]*$ ]]; then
  echo "PERF_WEB_SCHEMA must be a valid PostgreSQL identifier" >&2
  exit 1
fi

pick_port() {
  python3 - <<'PY'
import socket

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
}

append_search_path() {
  local raw_url="$1"
  local raw_schema="$2"

  if [[ "${raw_url}" == *"?"* ]]; then
    printf '%s&options=-csearch_path%%3D%s' "${raw_url}" "${raw_schema}"
  else
    printf '%s?options=-csearch_path%%3D%s' "${raw_url}" "${raw_schema}"
  fi
}

port="${PERF_WEB_SERVER_PORT:-$(pick_port)}"
http_addr="127.0.0.1:${port}"
base_url="http://${http_addr}"
scoped_database_url="$(append_search_path "${database_url}" "${schema}")"
work_dir="$(mktemp -d)"
server_log="${work_dir}/server.log"
server_pid=""

cleanup() {
  local status=$?

  if [[ -n "${server_pid}" ]] && kill -0 "${server_pid}" >/dev/null 2>&1; then
    kill "${server_pid}" >/dev/null 2>&1 || true
    wait "${server_pid}" >/dev/null 2>&1 || true
  fi

  if [[ "${status}" -ne 0 && -f "${server_log}" ]]; then
    echo
    echo "web performance smoke server log:" >&2
    cat "${server_log}" >&2
  fi

  psql "${database_url}" \
    -v ON_ERROR_STOP=1 \
    -c "DROP SCHEMA IF EXISTS ${schema} CASCADE" >/dev/null 2>&1 || true

  rm -rf "${work_dir}"
  exit "${status}"
}

trap cleanup EXIT

cd "${repo_root}"
mkdir -p "$(dirname "${result_file}")"

pnpm --dir apps/web build
cargo build --release --bin server

psql "${database_url}" \
  -v ON_ERROR_STOP=1 \
  -c "CREATE SCHEMA ${schema}" >/dev/null

env \
  APP_ENV="${APP_ENV:-dev}" \
  HTTP_ADDR="${http_addr}" \
  DATABASE_URL="${scoped_database_url}" \
  INIT_DB_ON_BOOT=true \
  INIT_ADMIN_USERNAME="${admin_username}" \
  INIT_ADMIN_PASSWORD="${admin_password}" \
  STATIC_DIR="${repo_root}/apps/web/dist" \
  "${server_bin}" >"${server_log}" 2>&1 &
server_pid=$!

for _ in $(seq 1 "${ready_timeout_sec}"); do
  if curl --silent --show-error --fail "${base_url}/api/healthz" >/dev/null 2>&1; then
    break
  fi

  if ! kill -0 "${server_pid}" >/dev/null 2>&1; then
    echo "web performance smoke server exited before health check became ready" >&2
    exit 1
  fi

  sleep 1
done

if ! curl --silent --show-error --fail "${base_url}/api/healthz" >/dev/null 2>&1; then
  echo "web performance smoke server did not become ready within ${ready_timeout_sec}s" >&2
  exit 1
fi

psql "${scoped_database_url}" \
  -v ON_ERROR_STOP=1 \
  -v admin_username="${admin_username}" <<'SQL' >/dev/null
UPDATE system_settings
SET
    setup_completed_at = COALESCE(setup_completed_at, NOW()),
    updated_at = NOW()
WHERE id = 1;

WITH admin_row AS (
    SELECT id FROM users WHERE username = :'admin_username' LIMIT 1
),
project_row AS (
    INSERT INTO projects (code, name, description)
    VALUES ('perf-ui', 'Perf UI Project', 'Production route timing smoke')
    RETURNING id
)
INSERT INTO project_members (project_id, user_id, role)
SELECT project_row.id, admin_row.id, 'admin'
FROM project_row, admin_row
ON CONFLICT (project_id, user_id) DO NOTHING;
SQL

PLAYWRIGHT_BASE_URL="${base_url}" \
  E2E_ADMIN_USERNAME="${admin_username}" \
  E2E_ADMIN_PASSWORD="${admin_password}" \
  PERF_WEB_RESULT_FILE="${result_file}" \
  pnpm --dir apps/web exec playwright test --config playwright.config.ts e2e/performance.spec.ts

echo "Web performance smoke result:"
cat "${result_file}"
