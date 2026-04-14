#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/db-clean-test-schemas.sh [--dry-run|--apply] [--database-url URL]

Lists or drops leftover PostgreSQL schemas created by database integration tests.

Defaults:
  - dry-run mode
  - TEST_DATABASE_URL, or DB_CLEAN_DATABASE_URL when explicitly set

This script only targets schemas that match the test helper naming convention:
  mini_conf_<test-prefix>_<numeric timestamp>
USAGE
}

mode="dry-run"
database_url="${DB_CLEAN_DATABASE_URL:-${TEST_DATABASE_URL:-}}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)
      mode="dry-run"
      shift
      ;;
    --apply)
      mode="apply"
      shift
      ;;
    --database-url)
      if [[ $# -lt 2 ]]; then
        echo "--database-url requires a value" >&2
        exit 2
      fi
      database_url="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if ! command -v psql >/dev/null 2>&1; then
  echo "psql is required to clean test schemas" >&2
  exit 1
fi

if [[ -z "${database_url}" ]]; then
  echo "TEST_DATABASE_URL is required; set DB_CLEAN_DATABASE_URL or pass --database-url to override" >&2
  exit 1
fi

schema_regex='^mini_conf_(auth_session|bootstrap_users|config_files|db_bootstrap|deployment_tokens|deployments|drafts|members|open_bundle|open_heartbeat|open_release|open_resolve|open_sync|projects|releases|test)_[0-9]+$'

connection_summary="$(
  psql "${database_url}" -X -v ON_ERROR_STOP=1 -Atc \
    "SELECT current_database() || ' as ' || current_user"
)"

schema_list="$(
  psql "${database_url}" -X -v ON_ERROR_STOP=1 -Atc \
    "SELECT schema_name
     FROM information_schema.schemata
     WHERE schema_name ~ \$mini_conf_schema_regex\$${schema_regex}\$mini_conf_schema_regex\$
     ORDER BY schema_name"
)"

matching_schemas=()
if [[ -n "${schema_list}" ]]; then
  mapfile -t matching_schemas <<<"${schema_list}"
fi

if [[ "${#matching_schemas[@]}" -eq 0 ]]; then
  echo "No leftover mini-conf test schemas found in ${connection_summary}."
  exit 0
fi

echo "Found ${#matching_schemas[@]} leftover mini-conf test schema(s) in ${connection_summary}:"
printf '  %s\n' "${matching_schemas[@]}"

if [[ "${mode}" != "apply" ]]; then
  echo
  echo "Dry run only. Re-run with --apply to drop these schemas."
  exit 0
fi

drop_sql="$(
  psql "${database_url}" -X -v ON_ERROR_STOP=1 -Atc \
    "SELECT format('DROP SCHEMA IF EXISTS %I CASCADE;', schema_name)
     FROM information_schema.schemata
     WHERE schema_name ~ \$mini_conf_schema_regex\$${schema_regex}\$mini_conf_schema_regex\$
     ORDER BY schema_name"
)"

printf '%s\n' "${drop_sql}" | psql "${database_url}" -X -v ON_ERROR_STOP=1
echo "Dropped ${#matching_schemas[@]} leftover mini-conf test schema(s)."
