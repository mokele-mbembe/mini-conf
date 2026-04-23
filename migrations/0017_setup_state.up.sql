CREATE TABLE IF NOT EXISTS system_settings (
    id SMALLINT PRIMARY KEY CHECK (id = 1),
    setup_completed_at TIMESTAMPTZ NULL,
    setup_completed_by_user_id BIGINT NULL REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO system_settings (id)
VALUES (1)
ON CONFLICT (id) DO NOTHING;
