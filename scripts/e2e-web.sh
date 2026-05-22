#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required for web e2e tests" >&2
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required for web e2e tests" >&2
  exit 1
fi

if ! command -v pnpm >/dev/null 2>&1; then
  echo "pnpm is required for web e2e tests" >&2
  exit 1
fi

if ! command -v psql >/dev/null 2>&1; then
  echo "psql is required for web e2e tests" >&2
  exit 1
fi

if [[ -v NO_COLOR && -v FORCE_COLOR ]]; then
  unset FORCE_COLOR
fi

if [[ -z "${TEST_DATABASE_URL:-}" ]]; then
  echo "TEST_DATABASE_URL is required for isolated web e2e tests" >&2
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
  local database_url="$1"
  local schema="$2"

  if [[ "${database_url}" == *"?"* ]]; then
    printf '%s&options=-csearch_path%%3D%s' "${database_url}" "${schema}"
  else
    printf '%s?options=-csearch_path%%3D%s' "${database_url}" "${schema}"
  fi
}

server_port="${E2E_SERVER_PORT:-$(pick_port)}"
web_port="${E2E_WEB_PORT:-$(pick_port)}"
http_addr="127.0.0.1:${server_port}"
base_url="http://127.0.0.1:${web_port}"
schema="${E2E_SCHEMA:-mini_conf_e2e_$(date +%s%N)}"
ready_timeout_sec="${E2E_READY_TIMEOUT_SEC:-180}"
admin_username="${E2E_ADMIN_USERNAME:-admin}"
admin_password="${E2E_ADMIN_PASSWORD:-admin123456}"

if [[ ! "${schema}" =~ ^[a-zA-Z_][a-zA-Z0-9_]*$ ]]; then
  echo "E2E_SCHEMA must be a valid PostgreSQL identifier" >&2
  exit 1
fi

scoped_database_url="$(append_search_path "${TEST_DATABASE_URL}" "${schema}")"
playwright_args=("$@")

work_dir="$(mktemp -d)"
server_log="${work_dir}/server.log"
web_log="${work_dir}/web.log"
server_pid=""
web_pid=""

cleanup() {
  local status=$?

  if [[ -n "${web_pid}" ]] && kill -0 "${web_pid}" >/dev/null 2>&1; then
    kill "${web_pid}" >/dev/null 2>&1 || true
    wait "${web_pid}" >/dev/null 2>&1 || true
  fi

  if [[ -n "${server_pid}" ]] && kill -0 "${server_pid}" >/dev/null 2>&1; then
    kill "${server_pid}" >/dev/null 2>&1 || true
    wait "${server_pid}" >/dev/null 2>&1 || true
  fi

  if [[ "${status}" -ne 0 ]]; then
    if [[ -f "${server_log}" ]]; then
      echo
      echo "web e2e backend log:" >&2
      cat "${server_log}" >&2
    fi
    if [[ -f "${web_log}" ]]; then
      echo
      echo "web e2e frontend log:" >&2
      cat "${web_log}" >&2
    fi
  fi

  psql "${TEST_DATABASE_URL}" \
    -v ON_ERROR_STOP=1 \
    -c "DROP SCHEMA IF EXISTS ${schema} CASCADE" >/dev/null 2>&1 || true

  rm -rf "${work_dir}"
  exit "${status}"
}

trap cleanup EXIT

cd "${repo_root}"

psql "${TEST_DATABASE_URL}" \
  -v ON_ERROR_STOP=1 \
  -c "CREATE SCHEMA ${schema}" >/dev/null

env \
  APP_ENV="${APP_ENV:-dev}" \
  HTTP_ADDR="${http_addr}" \
  DATABASE_URL="${scoped_database_url}" \
  INIT_DB_ON_BOOT=true \
  INIT_ADMIN_USERNAME="${admin_username}" \
  INIT_ADMIN_PASSWORD="${admin_password}" \
  cargo run --bin server >"${server_log}" 2>&1 &
server_pid=$!

for _ in $(seq 1 "${ready_timeout_sec}"); do
  if curl --silent --show-error --fail "http://${http_addr}/api/healthz" >/dev/null 2>&1; then
    break
  fi

  if ! kill -0 "${server_pid}" >/dev/null 2>&1; then
    echo "web e2e backend exited before health check became ready" >&2
    exit 1
  fi

  sleep 1
done

if ! curl --silent --show-error --fail "http://${http_addr}/api/healthz" >/dev/null 2>&1; then
  echo "web e2e backend did not become ready within ${ready_timeout_sec}s" >&2
  exit 1
fi

VITE_API_TARGET="http://${http_addr}" \
  pnpm --dir apps/web exec vite --host 127.0.0.1 --port "${web_port}" --strictPort >"${web_log}" 2>&1 &
web_pid=$!

for _ in $(seq 1 "${ready_timeout_sec}"); do
  if curl --silent --show-error --fail "${base_url}/" >/dev/null 2>&1; then
    break
  fi

  if ! kill -0 "${web_pid}" >/dev/null 2>&1; then
    echo "web e2e frontend exited before becoming ready" >&2
    exit 1
  fi

  sleep 1
done

if ! curl --silent --show-error --fail "${base_url}/" >/dev/null 2>&1; then
  echo "web e2e frontend did not become ready within ${ready_timeout_sec}s" >&2
  exit 1
fi

if [[ "${#playwright_args[@]}" -eq 0 ]]; then
  mapfile -t playwright_args < <(
    find apps/web/e2e \
      -maxdepth 1 \
      -type f \
      -name '*.spec.ts' \
      ! -name 'performance.spec.ts' \
      -printf 'e2e/%f\n' | sort
  )
fi

E2E_MANAGED_SERVER=1 \
  E2E_ADMIN_USERNAME="${admin_username}" \
  E2E_ADMIN_PASSWORD="${admin_password}" \
  PLAYWRIGHT_BASE_URL="${base_url}" \
  node apps/web/scripts/ensure-e2e-target.mjs && \
  E2E_MANAGED_SERVER=1 \
  E2E_ADMIN_USERNAME="${admin_username}" \
  E2E_ADMIN_PASSWORD="${admin_password}" \
  PLAYWRIGHT_BASE_URL="${base_url}" \
  env -u NO_COLOR pnpm --dir apps/web exec playwright test --config playwright.config.ts "${playwright_args[@]}"
