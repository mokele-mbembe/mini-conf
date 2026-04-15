#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
suite="${1:-}"

case "${suite}" in
  smoke)
    suite_dirs=("${repo_root}/tests/alpha/http/smoke")
    ;;
  full)
    suite_dirs=(
      "${repo_root}/tests/alpha/http/smoke"
      "${repo_root}/tests/alpha/http/full"
    )
    ;;
  *)
    echo "usage: bash scripts/alpha-http.sh <smoke|full>" >&2
    exit 1
    ;;
esac

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required for alpha HTTP tests" >&2
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required for alpha HTTP tests" >&2
  exit 1
fi

if ! command -v hurl >/dev/null 2>&1; then
  echo "hurl is required for alpha HTTP tests" >&2
  exit 1
fi

if ! command -v psql >/dev/null 2>&1; then
  echo "psql is required for alpha HTTP tests" >&2
  exit 1
fi

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "DATABASE_URL is required for alpha HTTP tests" >&2
  exit 1
fi

mapfile -t hurl_files < <(find "${suite_dirs[@]}" -type f -name '*.hurl' | sort)

if [[ "${#hurl_files[@]}" -eq 0 ]]; then
  echo "no Hurl files found for alpha suite: ${suite}" >&2
  exit 1
fi

http_addr="${HTTP_ADDR:-127.0.0.1:18080}"
base_url="${MINI_CONF_BASE_URL:-http://${http_addr}}"
app_env="${APP_ENV:-dev}"
init_db_on_boot="${INIT_DB_ON_BOOT:-true}"
admin_username="${ALPHA_ADMIN_USERNAME:-admin}"
admin_password="${ALPHA_ADMIN_PASSWORD:-admin123456}"
default_users_file="${repo_root}/tests/alpha/users.seed.yaml"
if [[ -f "${default_users_file}" ]]; then
  init_users_file="${ALPHA_INIT_USERS_FILE:-${default_users_file}}"
else
  init_users_file="${ALPHA_INIT_USERS_FILE:-}"
fi
suite_id="${ALPHA_SUITE_ID:-$(date +%s%N)}"
ready_timeout_sec="${ALPHA_HTTP_READY_TIMEOUT_SEC:-180}"
alpha_schema="${ALPHA_SCHEMA:-mini_conf_alpha_${suite}_$(date +%s%N)}"
scoped_database_url=""

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
    echo "alpha HTTP server log:" >&2
    cat "${server_log}" >&2
  fi

  if [[ -n "${alpha_schema}" ]]; then
    psql "${DATABASE_URL}" \
      -v ON_ERROR_STOP=1 \
      -c "DROP SCHEMA IF EXISTS ${alpha_schema} CASCADE" >/dev/null 2>&1 || true
  fi

  rm -rf "${work_dir}"
  exit "${status}"
}

trap cleanup EXIT

cd "${repo_root}"

if [[ "${DATABASE_URL}" == *"?"* ]]; then
  scoped_database_url="${DATABASE_URL}&options=-csearch_path%3D${alpha_schema}"
else
  scoped_database_url="${DATABASE_URL}?options=-csearch_path%3D${alpha_schema}"
fi

psql "${DATABASE_URL}" \
  -v ON_ERROR_STOP=1 \
  -c "CREATE SCHEMA ${alpha_schema}" >/dev/null

env \
  APP_ENV="${app_env}" \
  HTTP_ADDR="${http_addr}" \
  DATABASE_URL="${scoped_database_url}" \
  INIT_DB_ON_BOOT="${init_db_on_boot}" \
  INIT_ADMIN_USERNAME="${admin_username}" \
  INIT_ADMIN_PASSWORD="${admin_password}" \
  INIT_USERS_FILE="${init_users_file}" \
  cargo run --bin server >"${server_log}" 2>&1 &
server_pid=$!

for _ in $(seq 1 "${ready_timeout_sec}"); do
  if curl --silent --show-error --fail "${base_url}/api/healthz" >/dev/null 2>&1; then
    break
  fi

  if ! kill -0 "${server_pid}" >/dev/null 2>&1; then
    echo "alpha HTTP server exited before health check became ready" >&2
    exit 1
  fi

  sleep 1
done

if ! curl --silent --show-error --fail "${base_url}/api/healthz" >/dev/null 2>&1; then
  echo "alpha HTTP server did not become ready within ${ready_timeout_sec}s at ${base_url}/api/healthz" >&2
  exit 1
fi

hurl --test \
  --variable base_url="${base_url}" \
  --variable admin_username="${admin_username}" \
  --variable admin_password="${admin_password}" \
  --variable suite_id="${suite_id}" \
  "${hurl_files[@]}"
