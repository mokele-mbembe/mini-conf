CREATE TABLE IF NOT EXISTS projects (
    id BIGSERIAL PRIMARY KEY,
    code VARCHAR(64) NOT NULL UNIQUE,
    name VARCHAR(128) NOT NULL,
    description TEXT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS config_files (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id),
    code VARCHAR(64) NOT NULL,
    name VARCHAR(128) NOT NULL,
    format VARCHAR(16) NOT NULL,
    schema_name VARCHAR(128) NULL,
    schema_version VARCHAR(64) NULL,
    sensitivity VARCHAR(16) NOT NULL DEFAULT 'normal',
    secret_paths JSONB NULL,
    description TEXT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (project_id, code)
);

CREATE TABLE IF NOT EXISTS deployment_instances (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id),
    environment VARCHAR(32) NOT NULL,
    deployment_key VARCHAR(64) NOT NULL,
    name VARCHAR(128) NOT NULL,
    description TEXT NULL,
    is_template BOOLEAN NOT NULL DEFAULT FALSE,
    template_source_id BIGINT NULL REFERENCES deployment_instances(id),
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (project_id, environment, deployment_key)
);

CREATE TABLE IF NOT EXISTS releases (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id),
    config_file_id BIGINT NOT NULL REFERENCES config_files(id),
    deployment_instance_id BIGINT NOT NULL REFERENCES deployment_instances(id),
    revision VARCHAR(64) NOT NULL,
    content TEXT NOT NULL,
    content_hash CHAR(64) NOT NULL,
    format VARCHAR(16) NOT NULL,
    change_summary VARCHAR(255) NULL,
    diff_summary JSONB NULL,
    apply_mode VARCHAR(16) NOT NULL,
    published_by BIGINT NOT NULL,
    published_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (deployment_instance_id, config_file_id, revision)
);

CREATE INDEX IF NOT EXISTS idx_config_files_project_code
    ON config_files(project_id, code);

CREATE INDEX IF NOT EXISTS idx_deployment_instances_lookup
    ON deployment_instances(project_id, environment, deployment_key);

CREATE INDEX IF NOT EXISTS idx_releases_latest_lookup
    ON releases(deployment_instance_id, config_file_id, published_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS idx_releases_revision_unique
    ON releases(revision);
