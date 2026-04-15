ALTER TABLE deployment_instances
    ADD COLUMN IF NOT EXISTS environment VARCHAR(32) NULL;

UPDATE deployment_instances di
SET environment = pe.code
FROM project_environments pe
WHERE pe.id = di.environment_id;

ALTER TABLE deployment_instances
    DROP CONSTRAINT IF EXISTS deployment_instances_project_id_environment_id_deployment_key_key,
    DROP CONSTRAINT IF EXISTS deployment_instances_project_id_environment_id_fkey;

DROP INDEX IF EXISTS idx_deployment_instances_lookup;
DROP INDEX IF EXISTS idx_project_environments_project_code;

ALTER TABLE deployment_instances
    ALTER COLUMN environment SET NOT NULL,
    ADD CONSTRAINT deployment_instances_project_id_environment_deployment_key_key
        UNIQUE (project_id, environment, deployment_key);

CREATE INDEX IF NOT EXISTS idx_deployment_instances_lookup
    ON deployment_instances(project_id, environment, deployment_key);

ALTER TABLE deployment_instances
    DROP COLUMN IF EXISTS environment_id;

DROP TABLE IF EXISTS project_environments;
