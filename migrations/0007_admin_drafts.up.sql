CREATE TABLE IF NOT EXISTS drafts (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id),
    config_file_id BIGINT NOT NULL REFERENCES config_files(id),
    deployment_instance_id BIGINT NOT NULL REFERENCES deployment_instances(id),
    content TEXT NOT NULL,
    content_hash CHAR(64) NOT NULL,
    format VARCHAR(16) NOT NULL,
    schema_version VARCHAR(64) NULL,
    version BIGINT NOT NULL DEFAULT 1,
    editor_user_id BIGINT NOT NULL REFERENCES users(id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (config_file_id, deployment_instance_id)
);

CREATE INDEX IF NOT EXISTS idx_drafts_deployment_config
    ON drafts(deployment_instance_id, config_file_id);
