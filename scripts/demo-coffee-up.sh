#!/usr/bin/env bash
# Starts the full local coffee demo stack:
#   - mini-conf server against the isolated coffee schema          :8080
#   - demo control API + simulated business backends               :19001/:19002/:19010
#   - admin web frontend                                           :5173
#   - demo monitor frontend                                        :5174

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
run_file="${repo_root}/demo/coffee/generated/current-run.json"
log_dir="${repo_root}/demo/coffee/generated/logs"

cd "${repo_root}"

if [[ "${DEMO_COFFEE_RESET:-0}" == "1" || ! -f "${run_file}" ]]; then
  bash scripts/demo-coffee-reset.sh
fi

if [[ ! -f "${run_file}" ]]; then
  echo "Coffee demo is not initialised. Run: just demo-coffee-reset" >&2
  exit 1
fi

mkdir -p "${log_dir}"

demo_url="$(
  python3 - <<'PY'
import json
with open("demo/coffee/generated/current-run.json", encoding="utf-8") as handle:
    print(json.load(handle)["database_url"])
PY
)"

pids=()

cleanup() {
  local status=$?
  if [[ "${#pids[@]}" -gt 0 ]]; then
    echo
    echo "Stopping coffee demo processes..."
    kill "${pids[@]}" >/dev/null 2>&1 || true
    wait "${pids[@]}" >/dev/null 2>&1 || true
  fi
  exit "${status}"
}

trap cleanup EXIT INT TERM

wait_for_http() {
  local url="$1"
  local label="$2"
  local attempts="${3:-60}"

  for _ in $(seq 1 "${attempts}"); do
    if python3 - "${url}" <<'PY' >/dev/null 2>&1
import sys
import urllib.request

with urllib.request.urlopen(sys.argv[1], timeout=1) as response:
    raise SystemExit(0 if response.status < 500 else 1)
PY
    then
      echo "${label} is ready: ${url}"
      return 0
    fi
    sleep 1
  done

  echo "Timed out waiting for ${label}: ${url}" >&2
  return 1
}

echo "=== Coffee Demo Stack ==="
echo "Logs: ${log_dir}"
echo

echo "Starting mini-conf server..."
DATABASE_URL="${demo_url}" cargo run --bin server \
  >"${log_dir}/config-center.log" 2>&1 &
pids+=("$!")
wait_for_http "http://127.0.0.1:8080/api/healthz" "config center"

echo "Starting web frontend..."
pnpm --dir apps/web dev --host 127.0.0.1 \
  >"${log_dir}/web.log" 2>&1 &
pids+=("$!")
wait_for_http "http://127.0.0.1:5173" "web frontend"

echo "Starting demo access app (backends + control API)..."
python3 scripts/demo-coffee-access-app.py serve \
  >"${log_dir}/access-app.log" 2>&1 &
pids+=("$!")
wait_for_http \
  "http://127.0.0.1:19001/api/bootstrap/config-center?sn=SN001" \
  "backend-a"
wait_for_http \
  "http://127.0.0.1:19002/api/bootstrap/config-center?sn=SN001" \
  "backend-b"
wait_for_http "http://127.0.0.1:19010/api/demo/state" "demo control API"

echo "Starting demo monitor frontend..."
pnpm --dir demo/coffee/monitor dev --host 127.0.0.1 \
  >"${log_dir}/monitor.log" 2>&1 &
pids+=("$!")
wait_for_http "http://127.0.0.1:5174" "demo monitor"

cat <<EOF

Coffee demo is running.

  Admin UI (real admin operations):
    http://127.0.0.1:5173

  Demo Monitor (observe simulated clients):
    http://127.0.0.1:5174

  Login:
    admin / admin123456

  Bootstrap endpoints:
    http://127.0.0.1:19001/api/bootstrap/config-center?sn=SN001  (backend-a)
    http://127.0.0.1:19002/api/bootstrap/config-center?sn=SN001  (backend-b)

  Generated client output:
    demo/coffee/generated/effective-configs/

  Logs:
    ${log_dir}/

Press Ctrl+C to stop the whole stack.
EOF

wait -n "${pids[@]}"
echo "A coffee demo process exited. See logs in ${log_dir}." >&2
exit 1
