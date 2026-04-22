DROP INDEX IF EXISTS users_status_idx;
DROP INDEX IF EXISTS users_platform_admin_idx;

ALTER TABLE users
    DROP CONSTRAINT IF EXISTS users_status_check;

ALTER TABLE users
    DROP COLUMN IF EXISTS password_updated_at,
    DROP COLUMN IF EXISTS last_login_at,
    DROP COLUMN IF EXISTS must_change_password,
    DROP COLUMN IF EXISTS is_platform_admin;
