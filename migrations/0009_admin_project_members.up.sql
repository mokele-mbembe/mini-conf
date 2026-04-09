CREATE TABLE IF NOT EXISTS project_members (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id),
    user_id BIGINT NOT NULL REFERENCES users(id),
    role VARCHAR(32) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (project_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_project_members_project_role
    ON project_members(project_id, role);

CREATE INDEX IF NOT EXISTS idx_project_members_user_project
    ON project_members(user_id, project_id);

INSERT INTO project_members (project_id, user_id, role)
SELECT p.id, u.id, 'admin'
FROM projects p
JOIN users u
  ON u.username = 'admin'
 AND u.status = 'active'
ON CONFLICT (project_id, user_id) DO NOTHING;
