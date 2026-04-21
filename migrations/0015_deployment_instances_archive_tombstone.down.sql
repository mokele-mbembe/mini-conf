DROP INDEX IF EXISTS deployment_instances_uid_unique;

ALTER TABLE deployment_instances
    DROP CONSTRAINT IF EXISTS deployment_instances_archived_inactive_check,
    DROP CONSTRAINT IF EXISTS deployment_instances_deleted_inactive_check,
    DROP CONSTRAINT IF EXISTS deployment_instances_deleted_requires_archived_check;

DROP INDEX IF EXISTS deployment_instances_live_key_unique;

ALTER TABLE deployment_instances
    ADD CONSTRAINT deployment_instances_project_id_environment_id_deployment_key_key
        UNIQUE (project_id, environment_id, deployment_key);

DROP INDEX IF EXISTS idx_deployment_instances_lookup;

CREATE INDEX IF NOT EXISTS idx_deployment_instances_lookup
    ON deployment_instances(project_id, environment_id, deployment_key);

ALTER TABLE deployment_instances
    DROP COLUMN IF EXISTS delete_reason,
    DROP COLUMN IF EXISTS deleted_by,
    DROP COLUMN IF EXISTS deleted_at,
    DROP COLUMN IF EXISTS archive_reason,
    DROP COLUMN IF EXISTS archived_by,
    DROP COLUMN IF EXISTS archived_at,
    DROP COLUMN IF EXISTS is_archived,
    DROP COLUMN IF EXISTS deployment_uid;
