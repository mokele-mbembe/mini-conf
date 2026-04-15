#!/usr/bin/env bash

set -euo pipefail

mode="${1:---dry-run}"

case "${mode}" in
  --dry-run|--apply)
    ;;
  *)
    echo "usage: bash scripts/db-clean-alpha-projects.sh [--dry-run|--apply]" >&2
    exit 1
    ;;
esac

if ! command -v psql >/dev/null 2>&1; then
  echo "psql is required for alpha project cleanup" >&2
  exit 1
fi

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "DATABASE_URL is required for alpha project cleanup" >&2
  exit 1
fi

readarray -t project_rows < <(
  psql "${DATABASE_URL}" -X -A -t \
    -c "SELECT id || '|' || code FROM projects WHERE code LIKE 'alpha-%' ORDER BY code ASC"
)

if [[ "${#project_rows[@]}" -eq 0 ]]; then
  echo "No alpha-* projects found."
  exit 0
fi

echo "Alpha runtime projects:"
for row in "${project_rows[@]}"; do
  project_id="${row%%|*}"
  project_code="${row#*|}"
  echo "  - ${project_code} (id=${project_id})"
done

if [[ "${mode}" == "--dry-run" ]]; then
  echo
  echo "Dry run only. Re-run with --apply to delete related records."
  exit 0
fi

psql "${DATABASE_URL}" -v ON_ERROR_STOP=1 <<'SQL'
BEGIN;

WITH target_projects AS (
    SELECT id FROM projects WHERE code LIKE 'alpha-%'
),
target_deployments AS (
    SELECT id FROM deployment_instances WHERE project_id IN (SELECT id FROM target_projects)
)
DELETE FROM deployment_credentials
WHERE deployment_instance_id IN (SELECT id FROM target_deployments);

WITH target_projects AS (
    SELECT id FROM projects WHERE code LIKE 'alpha-%'
),
target_deployments AS (
    SELECT id FROM deployment_instances WHERE project_id IN (SELECT id FROM target_projects)
)
DELETE FROM drafts
WHERE deployment_instance_id IN (SELECT id FROM target_deployments);

WITH target_projects AS (
    SELECT id FROM projects WHERE code LIKE 'alpha-%'
)
DELETE FROM releases
WHERE project_id IN (SELECT id FROM target_projects);

WITH target_projects AS (
    SELECT id FROM projects WHERE code LIKE 'alpha-%'
)
DELETE FROM deployment_sync_records
WHERE project_id IN (SELECT id FROM target_projects);

WITH target_projects AS (
    SELECT id FROM projects WHERE code LIKE 'alpha-%'
)
DELETE FROM deployment_heartbeats
WHERE project_id IN (SELECT id FROM target_projects);

WITH target_projects AS (
    SELECT id FROM projects WHERE code LIKE 'alpha-%'
)
DELETE FROM audit_logs
WHERE project_id IN (SELECT id FROM target_projects);

WITH target_projects AS (
    SELECT id FROM projects WHERE code LIKE 'alpha-%'
)
DELETE FROM deployment_instances
WHERE project_id IN (SELECT id FROM target_projects);

WITH target_projects AS (
    SELECT id FROM projects WHERE code LIKE 'alpha-%'
)
DELETE FROM config_files
WHERE project_id IN (SELECT id FROM target_projects);

WITH target_projects AS (
    SELECT id FROM projects WHERE code LIKE 'alpha-%'
)
DELETE FROM project_members
WHERE project_id IN (SELECT id FROM target_projects);

WITH target_projects AS (
    SELECT id FROM projects WHERE code LIKE 'alpha-%'
)
DELETE FROM project_environments
WHERE project_id IN (SELECT id FROM target_projects);

DELETE FROM projects
WHERE code LIKE 'alpha-%';

COMMIT;
SQL

echo
echo "Deleted alpha-* runtime projects and related records."
