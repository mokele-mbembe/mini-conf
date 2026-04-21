#!/usr/bin/env bash
# scripts/demo-coffee-reset.sh
#
# Resets the coffee demo to a clean, fully seeded state.
#
# Steps:
#   1. Resolve local DATABASE_URL via local-db-env.sh (unless already exported).
#   2. Drop schema mini_conf_demo_coffee (if exists).
#   3. Create schema mini_conf_demo_coffee.
#   4. Run all migrations against the demo schema.
#   5. Run demo-coffee-seed binary to seed fixture data.
#   6. Write demo/coffee/generated/current-run.json.
#
# Usage:
#   just demo-coffee-reset
#   DATABASE_URL=postgres://... bash scripts/demo-coffee-reset.sh

set -euo pipefail

DEMO_SCHEMA="mini_conf_demo_coffee"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# ---------------------------------------------------------------------------
# 1. Resolve DATABASE_URL
# ---------------------------------------------------------------------------
if [[ -z "${DATABASE_URL:-}" ]]; then
  # shellcheck source=/dev/null
  source "${repo_root}/scripts/local-db-env.sh"
fi

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "ERROR: DATABASE_URL could not be resolved." >&2
  echo "  Configure MINI_CONF_LOCAL_DATABASE_URL or MINI_CONF_LOCAL_DB_* variables," >&2
  echo "  or export DATABASE_URL before running this script." >&2
  exit 1
fi

# Strip any existing search_path to get the base connection URL.
base_url="${DATABASE_URL%%\?*}"
if [[ "${DATABASE_URL}" == *"?"* ]]; then
  # Preserve existing query params except options (we'll append our own).
  existing_params="${DATABASE_URL#*\?}"
  # Remove any options= param (we replace it).
  existing_params="$(echo "${existing_params}" | sed 's/options=[^&]*//g; s/^&//; s/&&/\&/g; s/&$//')"
  if [[ -n "${existing_params}" ]]; then
    base_url="${base_url}?${existing_params}"
  fi
fi

# Build URL pointing at the demo schema.
if [[ "${base_url}" == *"?"* ]]; then
  demo_url="${base_url}&options=-csearch_path%3D${DEMO_SCHEMA}"
else
  demo_url="${base_url}?options=-csearch_path%3D${DEMO_SCHEMA}"
fi

echo "=== Coffee Demo Reset ==="
echo "Schema:  ${DEMO_SCHEMA}"
echo "Base DB: $(sed -E 's#//([^:/@]+):[^@]+@#//\1:***@#' <<<"${base_url}")"
echo ""

# ---------------------------------------------------------------------------
# 2 & 3. Drop and recreate schema
# ---------------------------------------------------------------------------
echo "→ Dropping schema ${DEMO_SCHEMA} (if exists)..."
psql "${base_url}" -c "DROP SCHEMA IF EXISTS ${DEMO_SCHEMA} CASCADE;" -q

echo "→ Creating schema ${DEMO_SCHEMA}..."
psql "${base_url}" -c "CREATE SCHEMA ${DEMO_SCHEMA};" -q

# ---------------------------------------------------------------------------
# 4. Run migrations against the demo schema
# ---------------------------------------------------------------------------
echo "→ Running migrations..."
cd "${repo_root}"
DATABASE_URL="${demo_url}" sqlx migrate run

# ---------------------------------------------------------------------------
# 5 & 6. Seed demo data (binary writes current-run.json)
# ---------------------------------------------------------------------------
echo "→ Seeding demo fixtures..."
DATABASE_URL="${demo_url}" \
CONFIG_CENTER_URL="${CONFIG_CENTER_URL:-http://127.0.0.1:8080}" \
  cargo run --bin demo-coffee-seed

echo ""
echo "✓ Coffee demo ready."
echo "  Next steps:"
echo "    just run-server-local-demo-coffee   # start server against demo schema"
echo "    just dev-web                        # start web frontend"
