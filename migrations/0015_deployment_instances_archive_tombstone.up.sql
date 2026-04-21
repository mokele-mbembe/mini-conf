CREATE EXTENSION IF NOT EXISTS pgcrypto;

ALTER TABLE deployment_instances
    ADD COLUMN IF NOT EXISTS deployment_uid UUID,
    ADD COLUMN IF NOT EXISTS is_archived BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS archived_at TIMESTAMPTZ NULL,
    ADD COLUMN IF NOT EXISTS archived_by BIGINT NULL REFERENCES users(id),
    ADD COLUMN IF NOT EXISTS archive_reason TEXT NULL,
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ NULL,
    ADD COLUMN IF NOT EXISTS deleted_by BIGINT NULL REFERENCES users(id),
    ADD COLUMN IF NOT EXISTS delete_reason TEXT NULL;

UPDATE deployment_instances
SET deployment_uid = gen_random_uuid()
WHERE deployment_uid IS NULL;

ALTER TABLE deployment_instances
    ALTER COLUMN deployment_uid SET NOT NULL,
    ALTER COLUMN deployment_uid SET DEFAULT gen_random_uuid();

ALTER TABLE deployment_instances
    DROP CONSTRAINT IF EXISTS deployment_instances_project_id_environment_id_deployment_key_key;

CREATE UNIQUE INDEX IF NOT EXISTS deployment_instances_live_key_unique
    ON deployment_instances (project_id, environment_id, deployment_key)
    WHERE deleted_at IS NULL;

DROP INDEX IF EXISTS idx_deployment_instances_lookup;

CREATE INDEX IF NOT EXISTS idx_deployment_instances_lookup
    ON deployment_instances(project_id, environment_id, deployment_key)
    WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS deployment_instances_uid_unique
    ON deployment_instances (deployment_uid);

ALTER TABLE deployment_instances
    DROP CONSTRAINT IF EXISTS deployment_instances_archived_inactive_check,
    DROP CONSTRAINT IF EXISTS deployment_instances_deleted_inactive_check,
    DROP CONSTRAINT IF EXISTS deployment_instances_deleted_requires_archived_check,
    ADD CONSTRAINT deployment_instances_archived_inactive_check
        CHECK (NOT is_archived OR status = 'inactive'),
    ADD CONSTRAINT deployment_instances_deleted_inactive_check
        CHECK (deleted_at IS NULL OR status = 'inactive'),
    ADD CONSTRAINT deployment_instances_deleted_requires_archived_check
        CHECK (deleted_at IS NULL OR is_archived);
