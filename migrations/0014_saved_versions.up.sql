CREATE TABLE IF NOT EXISTS draft_saved_versions (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id),
    deployment_instance_id BIGINT NOT NULL REFERENCES deployment_instances(id) ON DELETE CASCADE,
    config_file_id BIGINT NOT NULL REFERENCES config_files(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    note TEXT NULL,
    content TEXT NOT NULL,
    content_hash VARCHAR(64) NOT NULL,
    format TEXT NOT NULL,
    source_draft_version BIGINT NOT NULL,
    created_by BIGINT NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ NULL,
    CHECK (length(content_hash) = 64)
);

CREATE INDEX IF NOT EXISTS idx_saved_versions_lookup
    ON draft_saved_versions(project_id, deployment_instance_id, config_file_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_saved_versions_active
    ON draft_saved_versions(deployment_instance_id, config_file_id, created_at DESC)
    WHERE deleted_at IS NULL;
