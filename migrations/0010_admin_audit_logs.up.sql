CREATE TABLE IF NOT EXISTS audit_logs (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NULL REFERENCES projects(id),
    user_id BIGINT NULL REFERENCES users(id),
    action VARCHAR(64) NOT NULL,
    resource_type VARCHAR(64) NOT NULL,
    resource_id VARCHAR(64) NOT NULL,
    detail JSONB NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_project_created_at
    ON audit_logs(project_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_audit_logs_user_created_at
    ON audit_logs(user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_audit_logs_action_created_at
    ON audit_logs(action, created_at DESC);
