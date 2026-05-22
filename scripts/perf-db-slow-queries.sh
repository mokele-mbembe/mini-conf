#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
database_url="${TEST_DATABASE_URL:-${DATABASE_URL:-}}"
result_file="${PERF_DB_SLOW_QUERIES_RESULT_FILE:-${repo_root}/target/perf/db-slow-queries.json}"
limit="${PERF_DB_SLOW_QUERIES_LIMIT:-20}"
create_extension="${PERF_DB_SLOW_QUERIES_CREATE_EXTENSION:-0}"

if [[ -z "${database_url}" ]]; then
  echo "TEST_DATABASE_URL or DATABASE_URL is required for DB slow query reporting" >&2
  exit 1
fi

if [[ ! "${limit}" =~ ^[1-9][0-9]*$ ]]; then
  echo "PERF_DB_SLOW_QUERIES_LIMIT must be a positive integer" >&2
  exit 1
fi

if ! command -v psql >/dev/null 2>&1; then
  echo "psql is required for DB slow query reporting" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required for DB slow query reporting" >&2
  exit 1
fi

mkdir -p "$(dirname "${result_file}")"

write_unavailable_report() {
  local reason_file="$1"
  python3 - "${result_file}" "${limit}" "${reason_file}" <<'PY'
import json
import sys
from pathlib import Path

result_file = Path(sys.argv[1])
limit = int(sys.argv[2])
reason = Path(sys.argv[3]).read_text(encoding="utf-8", errors="replace").strip()

result_file.write_text(json.dumps({
    "mode": "pg_stat_statements",
    "available": False,
    "limit": limit,
    "reason": reason,
    "queries": [],
}, indent=2, ensure_ascii=False) + "\n")
PY
}

work_dir="$(mktemp -d)"
extension_error_file="${work_dir}/extension-error.txt"
query_error_file="${work_dir}/query-error.txt"
rows_file="${work_dir}/rows.tsv"

cleanup() {
  rm -rf "${work_dir}"
}
trap cleanup EXIT

if [[ "${create_extension}" == "1" ]]; then
  psql "${database_url}" \
    -v ON_ERROR_STOP=1 \
    -c "CREATE EXTENSION IF NOT EXISTS pg_stat_statements" >/dev/null 2>"${extension_error_file}" || true
fi

extension_available="$(psql "${database_url}" \
  -v ON_ERROR_STOP=1 \
  -At \
  -c "SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_stat_statements')" 2>"${extension_error_file}" || true)"

if [[ "${extension_available}" != "t" ]]; then
  if [[ -s "${extension_error_file}" ]]; then
    write_unavailable_report "${extension_error_file}"
  else
    printf '%s\n' "pg_stat_statements extension is not installed in the current database" >"${extension_error_file}"
    write_unavailable_report "${extension_error_file}"
  fi
  echo "DB slow query report:"
  cat "${result_file}"
  exit 0
fi

if ! psql "${database_url}" \
  -v ON_ERROR_STOP=1 \
  -At \
  -F $'\t' \
  -c "
    SELECT
      calls,
      round(total_exec_time::numeric, 3),
      round(mean_exec_time::numeric, 3),
      round(max_exec_time::numeric, 3),
      rows,
      regexp_replace(query, E'[\\n\\r\\t]+', ' ', 'g')
    FROM pg_stat_statements
    WHERE dbid = (SELECT oid FROM pg_database WHERE datname = current_database())
    ORDER BY total_exec_time DESC
    LIMIT ${limit}
  " >"${rows_file}" 2>"${query_error_file}"; then
  write_unavailable_report "${query_error_file}"
  echo "DB slow query report:"
  cat "${result_file}"
  exit 0
fi

python3 - "${rows_file}" "${result_file}" "${limit}" <<'PY'
import json
import sys
from pathlib import Path

rows_file = Path(sys.argv[1])
result_file = Path(sys.argv[2])
limit = int(sys.argv[3])

queries = []
for raw in rows_file.read_text(encoding="utf-8").splitlines():
    calls, total_ms, mean_ms, max_ms, rows, query = raw.split("\t", 5)
    queries.append({
        "calls": int(calls),
        "total_exec_ms": float(total_ms),
        "mean_exec_ms": float(mean_ms),
        "max_exec_ms": float(max_ms),
        "rows": int(rows),
        "query": query,
    })

result_file.write_text(json.dumps({
    "mode": "pg_stat_statements",
    "available": True,
    "limit": limit,
    "queries": queries,
}, indent=2, ensure_ascii=False) + "\n")
PY

echo "DB slow query report:"
cat "${result_file}"
