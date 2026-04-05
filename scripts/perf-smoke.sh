#!/usr/bin/env bash
set -euo pipefail

mkdir -p target/perf

metric_name="config_resolve_smoke_ms"
baseline_ms="${PERF_SMOKE_MAX_MS:-250}"
result_file="target/perf/smoke.json"

# MVP scaffold:
# Before the Rust service exists, keep the benchmark pipeline alive with a cheap placeholder metric.
# Once `/api/open/configs/resolve` is implemented, replace this with a real local benchmark command.
measured_ms=1
mode="placeholder"

if [[ -x scripts/run-perf-smoke.sh ]]; then
  output="$(scripts/run-perf-smoke.sh)"
  measured_ms="${output}"
  mode="real"
fi

cat > "${result_file}" <<JSON
{
  "metric": "${metric_name}",
  "mode": "${mode}",
  "threshold_ms": ${baseline_ms},
  "measured_ms": ${measured_ms}
}
JSON

echo "Performance smoke result:"
cat "${result_file}"

if [[ "${PERF_ENFORCE:-0}" == "1" ]]; then
  awk -v actual="${measured_ms}" -v limit="${baseline_ms}" 'BEGIN { exit !(actual <= limit) }'
  echo "Performance smoke check passed: ${measured_ms}ms <= ${baseline_ms}ms"
else
  echo "Performance smoke scaffold completed"
fi
