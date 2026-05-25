#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
result_file="${PERF_SMOKE_RESULT_FILE:-${repo_root}/target/perf/smoke.json}"
database_url="${TEST_DATABASE_URL:-${DATABASE_URL:-}}"
iterations="${PERF_SMOKE_ITERATIONS:-25}"
warmup_iterations="${PERF_SMOKE_WARMUP_ITERATIONS:-5}"
ready_timeout_sec="${PERF_SMOKE_READY_TIMEOUT_SEC:-180}"
token="${PERF_SMOKE_TOKEN:-mini-conf-perf-token}"
schema="${PERF_SMOKE_SCHEMA:-mini_conf_perf_$(date +%s%N)}"
cargo_target_dir="${CARGO_TARGET_DIR:-target}"
server_bin="${cargo_target_dir%/}/release/server"
dataset="${PERF_SMOKE_DATASET:-S}"
admin_username="${PERF_SMOKE_ADMIN_USERNAME:-admin}"
admin_password="${PERF_SMOKE_ADMIN_PASSWORD:-admin123456}"

case "${dataset}" in
  S)
    project_count="${PERF_SMOKE_PROJECTS:-1}"
    config_count="${PERF_SMOKE_CONFIGS_PER_PROJECT:-3}"
    deployment_count="${PERF_SMOKE_DEPLOYMENTS_PER_PROJECT:-1}"
    ;;
  M)
    project_count="${PERF_SMOKE_PROJECTS:-10}"
    config_count="${PERF_SMOKE_CONFIGS_PER_PROJECT:-20}"
    deployment_count="${PERF_SMOKE_DEPLOYMENTS_PER_PROJECT:-30}"
    ;;
  L)
    project_count="${PERF_SMOKE_PROJECTS:-50}"
    config_count="${PERF_SMOKE_CONFIGS_PER_PROJECT:-20}"
    deployment_count="${PERF_SMOKE_DEPLOYMENTS_PER_PROJECT:-100}"
    ;;
  *)
    echo "PERF_SMOKE_DATASET must be one of S, M, or L" >&2
    exit 1
    ;;
esac

if [[ -z "${database_url}" ]]; then
  echo "TEST_DATABASE_URL or DATABASE_URL is required for real performance smoke" >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required for performance smoke" >&2
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required for performance smoke" >&2
  exit 1
fi

if ! command -v psql >/dev/null 2>&1; then
  echo "psql is required for performance smoke" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required for performance smoke aggregation" >&2
  exit 1
fi

if [[ ! "${schema}" =~ ^[a-zA-Z_][a-zA-Z0-9_]*$ ]]; then
  echo "PERF_SMOKE_SCHEMA must be a valid PostgreSQL identifier" >&2
  exit 1
fi

for numeric_value in "${project_count}" "${config_count}" "${deployment_count}"; do
  if [[ ! "${numeric_value}" =~ ^[1-9][0-9]*$ ]]; then
    echo "PERF_SMOKE project/config/deployment counts must be positive integers" >&2
    exit 1
  fi
done

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

port="${PERF_SMOKE_SERVER_PORT:-$(pick_port)}"
http_addr="127.0.0.1:${port}"
base_url="http://${http_addr}"
scoped_database_url="$(append_search_path "${database_url}" "${schema}")"
work_dir="$(mktemp -d)"
server_log="${work_dir}/server.log"
samples_file="${work_dir}/samples.tsv"
server_pid=""

cleanup() {
  local status=$?

  if [[ -n "${server_pid}" ]] && kill -0 "${server_pid}" >/dev/null 2>&1; then
    kill "${server_pid}" >/dev/null 2>&1 || true
    wait "${server_pid}" >/dev/null 2>&1 || true
  fi

  if [[ "${status}" -ne 0 && -f "${server_log}" ]]; then
    echo
    echo "performance smoke server log:" >&2
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
: >"${samples_file}"

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
  STATIC_DIR="${PERF_SMOKE_STATIC_DIR:-apps/web/dist}" \
  "${server_bin}" >"${server_log}" 2>&1 &
server_pid=$!

for _ in $(seq 1 "${ready_timeout_sec}"); do
  if curl --silent --show-error --fail "${base_url}/api/healthz" >/dev/null 2>&1; then
    break
  fi

  if ! kill -0 "${server_pid}" >/dev/null 2>&1; then
    echo "performance smoke server exited before health check became ready" >&2
    exit 1
  fi

  sleep 1
done

if ! curl --silent --show-error --fail "${base_url}/api/healthz" >/dev/null 2>&1; then
  echo "performance smoke server did not become ready within ${ready_timeout_sec}s" >&2
  exit 1
fi

token_hash="$(python3 - "${token}" <<'PY'
import hashlib
import sys

print(hashlib.sha256(sys.argv[1].encode()).hexdigest())
PY
)"

psql "${scoped_database_url}" \
  -v ON_ERROR_STOP=1 \
  -v token_hash="${token_hash}" \
  -v admin_username="${admin_username}" \
  -v project_count="${project_count}" \
  -v config_count="${config_count}" \
  -v deployment_count="${deployment_count}" <<'SQL' >/dev/null
UPDATE system_settings
SET
    setup_completed_at = COALESCE(setup_completed_at, NOW()),
    updated_at = NOW()
WHERE id = 1;

WITH project_seed AS (
    SELECT
        n,
        CASE
            WHEN n = 1 THEN 'coffee-perf'
            ELSE 'coffee-perf-' || lpad(n::text, 3, '0')
        END AS code,
        CASE
            WHEN n = 1 THEN 'Coffee Perf'
            ELSE 'Coffee Perf ' || lpad(n::text, 3, '0')
        END AS name
    FROM generate_series(1, :project_count::int) AS seeded(n)
),
admin_row AS (
    SELECT id FROM users WHERE username = :'admin_username' LIMIT 1
),
project_rows AS (
    INSERT INTO projects (code, name, description)
    SELECT code, name, 'Performance smoke dataset'
    FROM project_seed
    RETURNING id, code
),
project_index AS (
    SELECT project_rows.id, project_rows.code, project_seed.n
    FROM project_rows
    JOIN project_seed ON project_seed.code = project_rows.code
),
member_rows AS (
    INSERT INTO project_members (project_id, user_id, role)
    SELECT project_index.id, admin_row.id, 'admin'
    FROM project_index, admin_row
    ON CONFLICT (project_id, user_id) DO NOTHING
),
environment_rows AS (
    INSERT INTO project_environments (project_id, code, name, status, sort_order)
    SELECT id, 'prod', 'Production', 'active', 10
    FROM project_index
    RETURNING id, project_id
),
deployment_seed AS (
    SELECT
        environment_rows.project_id,
        environment_rows.id AS environment_id,
        generated.n,
        CASE
            WHEN generated.n = 1 THEN 'store-001'
            ELSE 'store-' || lpad(generated.n::text, 3, '0')
        END AS deployment_key
    FROM environment_rows
    CROSS JOIN generate_series(1, :deployment_count::int) AS generated(n)
),
deployment_rows AS (
    INSERT INTO deployment_instances (
        project_id,
        environment_id,
        deployment_key,
        name,
        status
    )
    SELECT
        project_id,
        environment_id,
        deployment_key,
        'Store ' || lpad(n::text, 3, '0'),
        'active'
    FROM deployment_seed
    RETURNING id, project_id, deployment_key
),
credential_row AS (
    INSERT INTO deployment_credentials (
        deployment_instance_id,
        credential_name,
        token_hash,
        status
    )
    SELECT id, 'perf-smoke', :'token_hash', 'active'
    FROM deployment_rows
    WHERE project_id = (SELECT id FROM project_index WHERE n = 1)
      AND deployment_key = 'store-001'
),
config_seed AS (
    SELECT
        project_index.id AS project_id,
        generated.n,
        CASE
            WHEN generated.n = 1 THEN 'main'
            ELSE 'config-' || lpad(generated.n::text, 3, '0')
        END AS code,
        CASE
            WHEN generated.n = 1 THEN 'Main'
            ELSE 'Config ' || lpad(generated.n::text, 3, '0')
        END AS name
    FROM project_index
    CROSS JOIN generate_series(1, :config_count::int) AS generated(n)
),
config_rows AS (
    INSERT INTO config_files (project_id, code, name, format, sensitivity, status)
    SELECT project_id, code, name, 'yaml', 'normal', 'active'
    FROM config_seed
    RETURNING id, project_id, code
)
INSERT INTO releases (
    project_id,
    config_file_id,
    deployment_instance_id,
    revision,
    content,
    content_hash,
    format,
    change_summary,
    apply_mode,
    published_by
)
SELECT
    config_rows.project_id,
    config_rows.id,
    deployment_rows.id,
    'perf.' || config_rows.project_id || '.' || deployment_rows.id || '.' || config_rows.id,
    'config: ' || config_rows.code || E'\nvalue: perf\n',
    repeat(substr(md5(config_rows.project_id::text || ':' || deployment_rows.id::text || ':' || config_rows.id::text), 1, 32), 2),
    'yaml',
    'performance smoke seed',
    'soft',
    admin_row.id
FROM config_rows
JOIN deployment_rows ON deployment_rows.project_id = config_rows.project_id
CROSS JOIN admin_row;
SQL

record_sample() {
  local name="$1"
  local method="$2"
  local url="$3"
  local authorization="${4:-}"
  local output
  local code
  local seconds

  if [[ -n "${authorization}" ]]; then
    output="$(curl --silent --show-error --output /dev/null --write-out "%{http_code} %{time_total}" \
      --request "${method}" \
      --header "Authorization: Bearer ${authorization}" \
      "${url}" || printf '000 0')"
  else
    output="$(curl --silent --show-error --output /dev/null --write-out "%{http_code} %{time_total}" \
      --request "${method}" \
      "${url}" || printf '000 0')"
  fi

  code="${output%% *}"
  seconds="${output##* }"
  printf '%s\t%s\t%s\n' "${name}" "${code}" "${seconds}" >>"${samples_file}"
}

run_endpoint() {
  local name="$1"
  local method="$2"
  local url="$3"
  local authorization="${4:-}"

  for _ in $(seq 1 "${warmup_iterations}"); do
    record_sample "${name}:warmup" "${method}" "${url}" "${authorization}"
  done

  for _ in $(seq 1 "${iterations}"); do
    record_sample "${name}" "${method}" "${url}" "${authorization}"
  done
}

run_endpoint "healthz" "GET" "${base_url}/api/healthz"
run_endpoint "open_config_resolve" "GET" "${base_url}/api/open/configs/resolve?project=coffee-perf&environment=prod&deployment_key=store-001&config=main" "${token}"
run_endpoint "open_config_bundle" "GET" "${base_url}/api/open/deployments/store-001/config-bundle?project=coffee-perf&environment=prod" "${token}"
run_endpoint "metrics" "GET" "${base_url}/metrics"

server_rss_kb="$(ps -o rss= -p "${server_pid}")"
server_rss_kb="${server_rss_kb//[[:space:]]/}"

python3 - "${samples_file}" "${result_file}" "${iterations}" "${warmup_iterations}" "${PERF_SMOKE_MAX_MS:-100}" "${server_rss_kb:-0}" "${dataset}" "${project_count}" "${config_count}" "${deployment_count}" <<'PY'
import json
import math
import sys
from collections import defaultdict
from pathlib import Path

samples_path = Path(sys.argv[1])
result_path = Path(sys.argv[2])
iterations = int(sys.argv[3])
warmup_iterations = int(sys.argv[4])
threshold_ms = float(sys.argv[5])
server_rss_kb = int(sys.argv[6])
dataset = sys.argv[7]
project_count = int(sys.argv[8])
config_count = int(sys.argv[9])
deployment_count = int(sys.argv[10])

groups = defaultdict(list)
warmups = defaultdict(list)
for raw in samples_path.read_text().splitlines():
    name, code, ms = raw.split("\t")
    item = {"status": int(code), "duration_ms": float(ms) * 1000.0}
    if name.endswith(":warmup"):
        warmups[name.removesuffix(":warmup")].append(item)
    else:
        groups[name].append(item)

def percentile(values, quantile):
    if not values:
        return 0.0
    ordered = sorted(values)
    index = max(0, math.ceil(len(ordered) * quantile) - 1)
    return ordered[index]

metrics = []
for name in sorted(groups):
    items = groups[name]
    durations = [item["duration_ms"] for item in items]
    errors = sum(1 for item in items if item["status"] < 200 or item["status"] >= 400)
    metrics.append({
        "name": name,
        "samples": len(items),
        "warmup_samples": len(warmups.get(name, [])),
        "error_count": errors,
        "error_rate": round(errors / len(items), 4) if items else 0,
        "min_ms": round(min(durations), 3) if durations else 0,
        "p50_ms": round(percentile(durations, 0.50), 3),
        "p95_ms": round(percentile(durations, 0.95), 3),
        "p99_ms": round(percentile(durations, 0.99), 3),
        "max_ms": round(max(durations), 3) if durations else 0,
    })

measured_ms = round(max((metric["p95_ms"] for metric in metrics), default=0), 3)
result = {
    "mode": "real",
    "dataset": dataset,
    "dataset_size": {
        "projects": project_count,
        "configs_per_project": config_count,
        "deployments_per_project": deployment_count,
        "releases": project_count * config_count * deployment_count,
    },
    "threshold_ms": threshold_ms,
    "measured_ms": measured_ms,
    "iterations": iterations,
    "warmup_iterations": warmup_iterations,
    "server_rss_kb": server_rss_kb,
    "metrics": metrics,
}

result_path.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n")
print(json.dumps(result, ensure_ascii=False))
PY
