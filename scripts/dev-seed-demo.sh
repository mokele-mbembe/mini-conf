#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required for demo data seeding" >&2
  exit 1
fi

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "DATABASE_URL is required; use just dev-seed-demo-local or export a runtime DATABASE_URL" >&2
  exit 1
fi

cd "${repo_root}"
cargo run --bin dev-seed-demo
