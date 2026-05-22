#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output_dir="${PERF_BASELINE_DIR:-${repo_root}/target/perf/baseline}"
summary_json="${PERF_BASELINE_SUMMARY_JSON:-${output_dir}/summary.json}"
summary_md="${PERF_BASELINE_SUMMARY_MD:-${output_dir}/summary.md}"
threshold_env="${PERF_BASELINE_THRESHOLD_ENV:-${output_dir}/threshold-suggestions.env}"
datasets="${PERF_BASELINE_DATASETS:-S M}"
backend_repetitions="${PERF_BASELINE_REPETITIONS:-3}"
web_repetitions="${PERF_BASELINE_WEB_REPETITIONS:-1}"
run_backend="${PERF_BASELINE_RUN_BACKEND:-1}"
run_web="${PERF_BASELINE_RUN_WEB:-1}"
run_bundle="${PERF_BASELINE_RUN_BUNDLE:-1}"
run_db_slow_queries="${PERF_BASELINE_RUN_DB_SLOW_QUERIES:-1}"
backend_headroom="${PERF_BASELINE_BACKEND_HEADROOM:-2.5}"
backend_floor_ms="${PERF_BASELINE_BACKEND_FLOOR_MS:-100}"
web_route_headroom="${PERF_BASELINE_WEB_ROUTE_HEADROOM:-2.5}"
web_route_floor_ms="${PERF_BASELINE_WEB_ROUTE_FLOOR_MS:-250}"
web_api_headroom="${PERF_BASELINE_WEB_API_HEADROOM:-2.5}"
web_api_floor_ms="${PERF_BASELINE_WEB_API_FLOOR_MS:-100}"
bundle_headroom="${PERF_BASELINE_BUNDLE_HEADROOM:-1.15}"
collect_backend_limit_ms="${PERF_BASELINE_COLLECT_BACKEND_LIMIT_MS:-100000}"
collect_web_route_limit_ms="${PERF_BASELINE_COLLECT_WEB_ROUTE_LIMIT_MS:-100000}"
collect_web_api_limit_ms="${PERF_BASELINE_COLLECT_WEB_API_LIMIT_MS:-100000}"
collect_bundle_limit_kb="${PERF_BASELINE_COLLECT_BUNDLE_LIMIT_KB:-100000}"

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required for performance baseline aggregation" >&2
  exit 1
fi

if [[ ! "${backend_repetitions}" =~ ^[1-9][0-9]*$ ]]; then
  echo "PERF_BASELINE_REPETITIONS must be a positive integer" >&2
  exit 1
fi

if [[ ! "${web_repetitions}" =~ ^[0-9]+$ ]]; then
  echo "PERF_BASELINE_WEB_REPETITIONS must be a non-negative integer" >&2
  exit 1
fi

cd "${repo_root}"
mkdir -p "${output_dir}"

if [[ "${run_backend}" == "1" ]]; then
  for dataset in ${datasets}; do
    for run_index in $(seq 1 "${backend_repetitions}"); do
      result_file="${output_dir}/backend-${dataset}-${run_index}.json"
      echo "Collecting backend baseline: dataset=${dataset}, run=${run_index}/${backend_repetitions}"
      PERF_ENFORCE=0 \
        PERF_SMOKE_DATASET="${dataset}" \
        PERF_SMOKE_MAX_MS="${collect_backend_limit_ms}" \
        PERF_SMOKE_RESULT_FILE="${result_file}" \
        bash scripts/perf-smoke.sh
    done
  done
fi

if [[ "${run_web}" == "1" ]]; then
  for run_index in $(seq 1 "${web_repetitions}"); do
    result_file="${output_dir}/web-route-${run_index}.json"
    echo "Collecting web baseline: run=${run_index}/${web_repetitions}"
    PERF_WEB_MAX_ROUTE_MS="${collect_web_route_limit_ms}" \
      PERF_WEB_MAX_API_MS="${collect_web_api_limit_ms}" \
      PERF_WEB_RESULT_FILE="${result_file}" \
      bash scripts/web-perf-smoke.sh
  done
fi

if [[ "${run_bundle}" == "1" ]]; then
  bundle_build="${PERF_BASELINE_BUNDLE_BUILD:-1}"
  if [[ "${run_web}" == "1" ]]; then
    bundle_build="${PERF_BASELINE_BUNDLE_BUILD:-0}"
  fi

  echo "Collecting bundle baseline"
  BUNDLE_BUDGET_BUILD="${bundle_build}" \
    BUNDLE_BUDGET_MAX_JS_GZIP_KB="${collect_bundle_limit_kb}" \
    BUNDLE_BUDGET_MAX_CSS_GZIP_KB="${collect_bundle_limit_kb}" \
    BUNDLE_BUDGET_MAX_TOTAL_GZIP_KB="${collect_bundle_limit_kb}" \
    BUNDLE_BUDGET_RESULT_FILE="${output_dir}/bundle-budget.json" \
    bash scripts/web-bundle-budget.sh
fi

if [[ "${run_db_slow_queries}" == "1" ]]; then
  echo "Collecting DB slow query baseline"
  PERF_DB_SLOW_QUERIES_RESULT_FILE="${output_dir}/db-slow-queries.json" \
    bash scripts/perf-db-slow-queries.sh
fi

python3 - \
  "${output_dir}" \
  "${summary_json}" \
  "${summary_md}" \
  "${threshold_env}" \
  "${backend_headroom}" \
  "${backend_floor_ms}" \
  "${web_route_headroom}" \
  "${web_route_floor_ms}" \
  "${web_api_headroom}" \
  "${web_api_floor_ms}" \
  "${bundle_headroom}" <<'PY'
import json
import math
import statistics
import sys
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path

output_dir = Path(sys.argv[1])
summary_json = Path(sys.argv[2])
summary_md = Path(sys.argv[3])
threshold_env = Path(sys.argv[4])
backend_headroom = float(sys.argv[5])
backend_floor_ms = float(sys.argv[6])
web_route_headroom = float(sys.argv[7])
web_route_floor_ms = float(sys.argv[8])
web_api_headroom = float(sys.argv[9])
web_api_floor_ms = float(sys.argv[10])
bundle_headroom = float(sys.argv[11])


def load(path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return None


def round_up(value, step):
    if value <= 0:
        return 0
    return int(math.ceil(value / step) * step)


def fmt(value, suffix=""):
    if value is None:
        return "-"
    return f"{value}{suffix}"


def stats(values):
    if not values:
        return {
            "runs": 0,
            "min": None,
            "mean": None,
            "max": None,
        }
    return {
        "runs": len(values),
        "min": round(min(values), 3),
        "mean": round(statistics.fmean(values), 3),
        "max": round(max(values), 3),
    }


backend_by_dataset = defaultdict(list)
backend_endpoint_p95 = defaultdict(lambda: defaultdict(list))
for path in sorted(output_dir.glob("backend-*.json")):
    report = load(path)
    if not report:
        continue
    dataset = report.get("dataset", "-")
    backend_by_dataset[dataset].append(report)
    for metric in report.get("metrics", []):
        backend_endpoint_p95[dataset][metric.get("name", "-")].append(
            metric.get("p95_ms", 0)
        )

backend_summary = {}
backend_global_max = 0.0
for dataset, reports in sorted(backend_by_dataset.items()):
    measured_values = [report.get("measured_ms", 0) for report in reports]
    endpoint_summary = {}
    for name, values in sorted(backend_endpoint_p95[dataset].items()):
        endpoint_summary[name] = stats(values)
    max_measured = max(measured_values, default=0)
    backend_global_max = max(backend_global_max, max_measured)
    backend_summary[dataset] = {
        "measured_ms": stats(measured_values),
        "dataset_size": reports[-1].get("dataset_size", {}),
        "server_rss_kb": stats([report.get("server_rss_kb", 0) for report in reports]),
        "endpoints_p95_ms": endpoint_summary,
    }

backend_threshold = round_up(
    max(backend_floor_ms, backend_global_max * backend_headroom), 10
)

web_reports = []
for path in sorted(output_dir.glob("web-route-*.json")):
    report = load(path)
    if report:
        web_reports.append(report)

web_route_values = [report.get("max_route_ms", 0) for report in web_reports]
web_api_values = [report.get("max_api_ms", 0) for report in web_reports]
web_route_threshold = round_up(
    max(web_route_floor_ms, max(web_route_values, default=0) * web_route_headroom), 10
)
web_api_threshold = round_up(
    max(web_api_floor_ms, max(web_api_values, default=0) * web_api_headroom), 10
)

web_summary = {
    "max_route_ms": stats(web_route_values),
    "max_api_ms": stats(web_api_values),
}

bundle_report = load(output_dir / "bundle-budget.json")
bundle_summary = None
bundle_thresholds = {}
if bundle_report:
    actuals = bundle_report.get("actuals", {})
    bundle_summary = {
        "actuals": actuals,
        "largest_js": bundle_report.get("largest_js"),
        "largest_css": bundle_report.get("largest_css"),
        "asset_count": bundle_report.get("asset_count"),
    }
    bundle_thresholds = {
        "BUNDLE_BUDGET_MAX_JS_GZIP_KB": round_up(
            actuals.get("largest_js_gzip_kb", 0) * bundle_headroom, 10
        ),
        "BUNDLE_BUDGET_MAX_CSS_GZIP_KB": round_up(
            actuals.get("largest_css_gzip_kb", 0) * bundle_headroom, 10
        ),
        "BUNDLE_BUDGET_MAX_TOTAL_GZIP_KB": round_up(
            actuals.get("total_gzip_kb", 0) * bundle_headroom, 10
        ),
    }

db_report = load(output_dir / "db-slow-queries.json")

thresholds = {}
if backend_summary:
    thresholds["PERF_SMOKE_MAX_MS"] = backend_threshold
if web_reports:
    thresholds["PERF_WEB_MAX_ROUTE_MS"] = web_route_threshold
    thresholds["PERF_WEB_MAX_API_MS"] = web_api_threshold
thresholds.update(bundle_thresholds)

summary = {
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "source_dir": str(output_dir),
    "calibration": {
        "backend_headroom": backend_headroom,
        "backend_floor_ms": backend_floor_ms,
        "web_route_headroom": web_route_headroom,
        "web_route_floor_ms": web_route_floor_ms,
        "web_api_headroom": web_api_headroom,
        "web_api_floor_ms": web_api_floor_ms,
        "bundle_headroom": bundle_headroom,
    },
    "backend": backend_summary,
    "web": web_summary,
    "bundle": bundle_summary,
    "db_slow_queries": db_report,
    "threshold_suggestions": thresholds,
}

summary_json.write_text(json.dumps(summary, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

env_lines = [
    "# Generated by scripts/perf-baseline.sh",
    "# Review these suggestions before applying them to CI or local defaults.",
]
for key, value in thresholds.items():
    env_lines.append(f"{key}={value}")
threshold_env.write_text("\n".join(env_lines) + "\n", encoding="utf-8")

lines = ["# Performance Baseline", ""]
lines.append(f"- generated_at: `{summary['generated_at']}`")
lines.append(f"- source_dir: `{output_dir}`")
lines.append("")

if backend_summary:
    lines.extend([
        "## Backend",
        "",
        "| dataset | runs | measured min | measured mean | measured max | suggested gate | rss max |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
    ])
    for dataset, report in backend_summary.items():
        measured = report["measured_ms"]
        rss = report["server_rss_kb"]
        lines.append(
            f"| {dataset} | {measured['runs']} | {fmt(measured['min'], ' ms')} | "
            f"{fmt(measured['mean'], ' ms')} | {fmt(measured['max'], ' ms')} | "
            f"{backend_threshold} ms | {fmt(rss['max'], ' KB')} |"
        )
    lines.append("")

if web_reports:
    lines.extend([
        "## Web",
        "",
        "| metric | runs | min | mean | max | suggested gate |",
        "| --- | ---: | ---: | ---: | ---: | ---: |",
        f"| route max | {web_summary['max_route_ms']['runs']} | {fmt(web_summary['max_route_ms']['min'], ' ms')} | {fmt(web_summary['max_route_ms']['mean'], ' ms')} | {fmt(web_summary['max_route_ms']['max'], ' ms')} | {web_route_threshold} ms |",
        f"| API max | {web_summary['max_api_ms']['runs']} | {fmt(web_summary['max_api_ms']['min'], ' ms')} | {fmt(web_summary['max_api_ms']['mean'], ' ms')} | {fmt(web_summary['max_api_ms']['max'], ' ms')} | {web_api_threshold} ms |",
        "",
    ])

if bundle_summary:
    actuals = bundle_summary["actuals"]
    lines.extend([
        "## Bundle",
        "",
        "| metric | actual | suggested budget |",
        "| --- | ---: | ---: |",
        f"| largest JS gzip | {fmt(actuals.get('largest_js_gzip_kb'), ' KB')} | {thresholds.get('BUNDLE_BUDGET_MAX_JS_GZIP_KB')} KB |",
        f"| largest CSS gzip | {fmt(actuals.get('largest_css_gzip_kb'), ' KB')} | {thresholds.get('BUNDLE_BUDGET_MAX_CSS_GZIP_KB')} KB |",
        f"| total gzip | {fmt(actuals.get('total_gzip_kb'), ' KB')} | {thresholds.get('BUNDLE_BUDGET_MAX_TOTAL_GZIP_KB')} KB |",
        "",
    ])

lines.extend([
    "## Suggested Env",
    "",
    "```bash",
    *[f"{key}={value}" for key, value in thresholds.items()],
    "```",
    "",
])

summary_md.write_text("\n".join(lines), encoding="utf-8")

print(summary_json)
print(summary_md)
print(threshold_env)
PY

echo "Performance baseline summary:"
cat "${summary_md}"
