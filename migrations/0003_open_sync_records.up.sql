CREATE TABLE IF NOT EXISTS deployment_sync_records (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id),
    deployment_instance_id BIGINT NOT NULL REFERENCES deployment_instances(id),
    config_file_id BIGINT NULL REFERENCES config_files(id),
    release_id BIGINT NULL REFERENCES releases(id),
    process_key VARCHAR(64) NULL,
    revision VARCHAR(64) NULL,
    action VARCHAR(32) NOT NULL,
    status VARCHAR(32) NOT NULL,
    message VARCHAR(255) NULL,
    detail JSONB NULL,
    reported_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_deployment_sync_records_deployment_reported_at
    ON deployment_sync_records(deployment_instance_id, reported_at DESC);

CREATE INDEX IF NOT EXISTS idx_deployment_sync_records_release_reported_at
    ON deployment_sync_records(release_id, reported_at DESC);
