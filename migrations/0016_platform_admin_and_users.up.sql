ALTER TABLE users
    ADD COLUMN IF NOT EXISTS is_platform_admin BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS must_change_password BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS last_login_at TIMESTAMPTZ NULL,
    ADD COLUMN IF NOT EXISTS password_updated_at TIMESTAMPTZ NULL;

ALTER TABLE users
    DROP CONSTRAINT IF EXISTS users_status_check;

ALTER TABLE users
    ADD CONSTRAINT users_status_check
    CHECK (status IN ('active', 'disabled'));

CREATE INDEX IF NOT EXISTS users_platform_admin_idx
    ON users (is_platform_admin);

CREATE INDEX IF NOT EXISTS users_status_idx
    ON users (status);
