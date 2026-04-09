DROP INDEX IF EXISTS idx_audit_logs_action_created_at;
DROP INDEX IF EXISTS idx_audit_logs_user_created_at;
DROP INDEX IF EXISTS idx_audit_logs_project_created_at;

DROP TABLE IF EXISTS audit_logs;
