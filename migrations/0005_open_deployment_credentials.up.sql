CREATE TABLE IF NOT EXISTS deployment_credentials (
    id BIGSERIAL PRIMARY KEY,
    deployment_instance_id BIGINT NOT NULL REFERENCES deployment_instances(id),
    credential_name VARCHAR(64) NOT NULL DEFAULT 'default',
    token_hash VARCHAR(255) NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    last_used_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (deployment_instance_id, credential_name),
    UNIQUE (token_hash)
);

CREATE INDEX IF NOT EXISTS idx_deployment_credentials_deployment_status
    ON deployment_credentials(deployment_instance_id, status);
