CREATE TABLE IF NOT EXISTS deployment_heartbeats (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id),
    deployment_instance_id BIGINT NOT NULL REFERENCES deployment_instances(id),
    process_key VARCHAR(64) NOT NULL,
    metadata JSONB NULL,
    reported_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (deployment_instance_id, process_key)
);

CREATE INDEX IF NOT EXISTS idx_deployment_heartbeats_deployment_reported_at
    ON deployment_heartbeats(deployment_instance_id, reported_at DESC);
