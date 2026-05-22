#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dist_dir="${repo_root}/apps/web/dist"
result_file="${BUNDLE_BUDGET_RESULT_FILE:-${repo_root}/target/perf/bundle-budget.json}"
max_js_gzip_kb="${BUNDLE_BUDGET_MAX_JS_GZIP_KB:-450}"
max_css_gzip_kb="${BUNDLE_BUDGET_MAX_CSS_GZIP_KB:-80}"
max_total_gzip_kb="${BUNDLE_BUDGET_MAX_TOTAL_GZIP_KB:-800}"
build_before_check="${BUNDLE_BUDGET_BUILD:-1}"

if ! command -v pnpm >/dev/null 2>&1; then
  echo "pnpm is required for bundle budget checks" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required for bundle budget aggregation" >&2
  exit 1
fi

cd "${repo_root}"
mkdir -p "$(dirname "${result_file}")"

if [[ "${build_before_check}" == "1" || ! -f "${dist_dir}/index.html" ]]; then
  pnpm --dir apps/web build
fi

python3 - \
  "${dist_dir}" \
  "${result_file}" \
  "${max_js_gzip_kb}" \
  "${max_css_gzip_kb}" \
  "${max_total_gzip_kb}" <<'PY'
import gzip
import json
import sys
from pathlib import Path

dist_dir = Path(sys.argv[1])
result_file = Path(sys.argv[2])
max_js_gzip_kb = float(sys.argv[3])
max_css_gzip_kb = float(sys.argv[4])
max_total_gzip_kb = float(sys.argv[5])

asset_dir = dist_dir / "assets"
if not asset_dir.is_dir():
    raise SystemExit(f"{asset_dir} does not exist; run a frontend build first")

assets = []
total_gzip_bytes = 0
for path in sorted(asset_dir.iterdir()):
    if not path.is_file() or path.suffix not in {".js", ".css"}:
        continue
    raw = path.read_bytes()
    gzip_size = len(gzip.compress(raw, compresslevel=9, mtime=0))
    total_gzip_bytes += gzip_size
    assets.append({
        "file": path.name,
        "type": path.suffix.removeprefix("."),
        "raw_kb": round(len(raw) / 1024, 3),
        "gzip_kb": round(gzip_size / 1024, 3),
    })

largest_js = max((asset for asset in assets if asset["type"] == "js"), key=lambda item: item["gzip_kb"], default=None)
largest_css = max((asset for asset in assets if asset["type"] == "css"), key=lambda item: item["gzip_kb"], default=None)
total_gzip_kb = round(total_gzip_bytes / 1024, 3)

budgets = {
    "max_js_gzip_kb": max_js_gzip_kb,
    "max_css_gzip_kb": max_css_gzip_kb,
    "max_total_gzip_kb": max_total_gzip_kb,
}
actuals = {
    "largest_js_gzip_kb": largest_js["gzip_kb"] if largest_js else 0,
    "largest_css_gzip_kb": largest_css["gzip_kb"] if largest_css else 0,
    "total_gzip_kb": total_gzip_kb,
}
violations = []
if actuals["largest_js_gzip_kb"] > max_js_gzip_kb:
    violations.append({
        "metric": "largest_js_gzip_kb",
        "actual": actuals["largest_js_gzip_kb"],
        "budget": max_js_gzip_kb,
        "file": largest_js["file"] if largest_js else None,
    })
if actuals["largest_css_gzip_kb"] > max_css_gzip_kb:
    violations.append({
        "metric": "largest_css_gzip_kb",
        "actual": actuals["largest_css_gzip_kb"],
        "budget": max_css_gzip_kb,
        "file": largest_css["file"] if largest_css else None,
    })
if actuals["total_gzip_kb"] > max_total_gzip_kb:
    violations.append({
        "metric": "total_gzip_kb",
        "actual": actuals["total_gzip_kb"],
        "budget": max_total_gzip_kb,
        "file": None,
    })

report = {
    "mode": "real",
    "budgets": budgets,
    "actuals": actuals,
    "largest_js": largest_js,
    "largest_css": largest_css,
    "asset_count": len(assets),
    "assets": assets,
    "violations": violations,
    "passed": not violations,
}

result_file.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
print(json.dumps({
    "passed": report["passed"],
    "actuals": actuals,
    "violations": violations,
}, ensure_ascii=False))

if violations:
    raise SystemExit(1)
PY

echo "Bundle budget result:"
cat "${result_file}"
