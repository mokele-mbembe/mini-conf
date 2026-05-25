#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

resolve_path() {
  case "$1" in
    /*) printf '%s\n' "$1" ;;
    *) printf '%s/%s\n' "${repo_root}" "$1" ;;
  esac
}

default_perf_dir="${repo_root}/target/perf"
configured_perf_dir="${PERF_SUMMARY_DIR:-}"
configured_result_file="${PERF_SUMMARY_RESULT_FILE:-}"

if [[ -n "${configured_result_file}" ]]; then
  result_file="$(resolve_path "${configured_result_file}")"
else
  perf_dir_for_result="${configured_perf_dir:-${default_perf_dir}}"
  result_file="$(resolve_path "${perf_dir_for_result}")/summary.md"
fi

if [[ -n "${configured_perf_dir}" ]]; then
  perf_dir="$(resolve_path "${configured_perf_dir}")"
elif [[ -n "${configured_result_file}" ]]; then
  perf_dir="$(dirname "${result_file}")"
else
  perf_dir="${default_perf_dir}"
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required for performance summary" >&2
  exit 1
fi

mkdir -p "$(dirname "${result_file}")"

python3 - "${perf_dir}" "${result_file}" <<'PY'
import json
import sys
from pathlib import Path

perf_dir = Path(sys.argv[1])
result_file = Path(sys.argv[2])

lines = ["# Performance Summary", ""]

def load(path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return None

def fmt(value, suffix=""):
    if value is None:
        return "-"
    return f"{value}{suffix}"

smoke_files = sorted(perf_dir.glob("smoke*.json"))
if smoke_files:
    lines.extend([
        "## Backend Smoke",
        "",
        "| file | dataset | measured p95 max | threshold | rss | errors |",
        "| --- | --- | ---: | ---: | ---: | ---: |",
    ])
    for path in smoke_files:
        report = load(path)
        if not report:
            continue
        errors = sum(metric.get("error_count", 0) for metric in report.get("metrics", []))
        lines.append(
            "| {file} | {dataset} | {measured} | {threshold} | {rss} | {errors} |".format(
                file=path.name,
                dataset=report.get("dataset", "-"),
                measured=fmt(report.get("measured_ms"), " ms"),
                threshold=fmt(report.get("threshold_ms"), " ms"),
                rss=fmt(report.get("server_rss_kb"), " KB"),
                errors=errors,
            )
        )
    lines.append("")

web_report = load(perf_dir / "web-route.json")
if web_report:
    thresholds = web_report.get("thresholds", {})
    lines.extend([
        "## Web Route Smoke",
        "",
        "| flow | max route | route threshold | max API | API threshold | violations |",
        "| --- | ---: | ---: | ---: | ---: | ---: |",
        "| {flow} | {route} | {route_budget} | {api} | {api_budget} | {violations} |".format(
            flow=web_report.get("flow", "-"),
            route=fmt(web_report.get("max_route_ms"), " ms"),
            route_budget=fmt(thresholds.get("max_route_ms"), " ms"),
            api=fmt(web_report.get("max_api_ms"), " ms"),
            api_budget=fmt(thresholds.get("max_api_ms"), " ms"),
            violations=len(web_report.get("violations", [])),
        ),
        "",
    ])

bundle_report = load(perf_dir / "bundle-budget.json")
if bundle_report:
    actuals = bundle_report.get("actuals", {})
    budgets = bundle_report.get("budgets", {})
    lines.extend([
        "## Bundle Budget",
        "",
        "| largest JS | JS budget | largest CSS | CSS budget | total gzip | total budget | violations |",
        "| ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        "| {largest_js} | {js_budget} | {largest_css} | {css_budget} | {total} | {total_budget} | {violations} |".format(
            largest_js=fmt(actuals.get("largest_js_gzip_kb"), " KB"),
            js_budget=fmt(budgets.get("max_js_gzip_kb"), " KB"),
            largest_css=fmt(actuals.get("largest_css_gzip_kb"), " KB"),
            css_budget=fmt(budgets.get("max_css_gzip_kb"), " KB"),
            total=fmt(actuals.get("total_gzip_kb"), " KB"),
            total_budget=fmt(budgets.get("max_total_gzip_kb"), " KB"),
            violations=len(bundle_report.get("violations", [])),
        ),
        "",
    ])

db_files = sorted(perf_dir.glob("db-slow-queries*.json"))
if db_files:
    lines.extend([
        "## DB Slow Queries",
        "",
        "| file | available | captured queries | reason |",
        "| --- | ---: | ---: | --- |",
    ])
    for path in db_files:
        report = load(path)
        if not report:
            continue
        available = report.get("available", False)
        captured = len(report.get("queries", [])) if available else "-"
        reason = "-" if available else report.get("reason", "-")
        lines.append(
            "| {file} | {available} | {captured} | {reason} |".format(
                file=path.name,
                available=str(available).lower(),
                captured=captured,
                reason=reason,
            )
        )
    lines.append("")

if len(lines) == 2:
    lines.append("No performance reports found.")
    lines.append("")

result_file.write_text("\n".join(lines), encoding="utf-8")
print(result_file)
PY

echo "Performance summary:"
cat "${result_file}"
