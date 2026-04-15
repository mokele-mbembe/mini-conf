CREATE TABLE IF NOT EXISTS project_environments (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id),
    code VARCHAR(32) NOT NULL,
    name VARCHAR(128) NOT NULL,
    description TEXT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT project_environments_project_id_code_key UNIQUE (project_id, code),
    CONSTRAINT project_environments_project_id_id_key UNIQUE (project_id, id),
    CONSTRAINT project_environments_status_check CHECK (status IN ('active', 'inactive'))
);

INSERT INTO project_environments (
    project_id,
    code,
    name,
    description,
    status,
    sort_order
)
SELECT
    seeded.project_id,
    seeded.environment,
    seeded.environment,
    NULL,
    'active',
    seeded.sort_order
FROM (
    SELECT
        di.project_id,
        di.environment,
        (ROW_NUMBER() OVER (
            PARTITION BY di.project_id
            ORDER BY di.environment ASC
        ) * 10)::INTEGER AS sort_order
    FROM (
        SELECT DISTINCT project_id, environment
        FROM deployment_instances
    ) di
) seeded
ON CONFLICT (project_id, code) DO NOTHING;

ALTER TABLE deployment_instances
    ADD COLUMN IF NOT EXISTS environment_id BIGINT NULL;

UPDATE deployment_instances di
SET environment_id = pe.id
FROM project_environments pe
WHERE pe.project_id = di.project_id
  AND pe.code = di.environment;

ALTER TABLE deployment_instances
    DROP CONSTRAINT IF EXISTS deployment_instances_project_id_environment_deployment_key_key;

DROP INDEX IF EXISTS idx_deployment_instances_lookup;

ALTER TABLE deployment_instances
    ALTER COLUMN environment_id SET NOT NULL,
    ADD CONSTRAINT deployment_instances_project_id_environment_id_fkey
        FOREIGN KEY (project_id, environment_id)
        REFERENCES project_environments(project_id, id),
    ADD CONSTRAINT deployment_instances_project_id_environment_id_deployment_key_key
        UNIQUE (project_id, environment_id, deployment_key);

CREATE INDEX IF NOT EXISTS idx_project_environments_project_code
    ON project_environments(project_id, code);

CREATE INDEX IF NOT EXISTS idx_deployment_instances_lookup
    ON deployment_instances(project_id, environment_id, deployment_key);

ALTER TABLE deployment_instances
    DROP COLUMN IF EXISTS environment;
