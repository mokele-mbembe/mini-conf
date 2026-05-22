#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
perf_dir="${PERF_SUMMARY_DIR:-${repo_root}/target/perf}"
result_file="${PERF_SUMMARY_RESULT_FILE:-${perf_dir}/summary.md}"

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

db_report = load(perf_dir / "db-slow-queries.json")
if db_report:
    lines.extend([
        "## DB Slow Queries",
        "",
        f"- available: `{str(db_report.get('available', False)).lower()}`",
    ])
    if db_report.get("available"):
        lines.append(f"- captured queries: `{len(db_report.get('queries', []))}`")
    else:
        lines.append(f"- reason: {db_report.get('reason', '-')}")
    lines.append("")

if len(lines) == 2:
    lines.append("No performance reports found.")
    lines.append("")

result_file.write_text("\n".join(lines), encoding="utf-8")
print(result_file)
PY

echo "Performance summary:"
cat "${result_file}"
