UPDATE deployment_sync_records dsr
SET config_file_id = cf.id
FROM config_files cf
WHERE dsr.config_file_id IS NULL
  AND cf.project_id = dsr.project_id
  AND cf.code = dsr.process_key;

DELETE FROM deployment_sync_records
WHERE config_file_id IS NULL;

ALTER TABLE deployment_sync_records
    ALTER COLUMN config_file_id SET NOT NULL,
    DROP COLUMN IF EXISTS process_key;

ALTER TABLE deployment_heartbeats
    ADD COLUMN IF NOT EXISTS config_file_id BIGINT NULL REFERENCES config_files(id);

UPDATE deployment_heartbeats dh
SET config_file_id = cf.id
FROM config_files cf
WHERE cf.project_id = dh.project_id
  AND cf.code = dh.process_key;

DELETE FROM deployment_heartbeats
WHERE config_file_id IS NULL;

ALTER TABLE deployment_heartbeats
    ALTER COLUMN config_file_id SET NOT NULL,
    DROP CONSTRAINT IF EXISTS deployment_heartbeats_deployment_instance_id_process_key_key,
    ADD CONSTRAINT deployment_heartbeats_deployment_instance_id_config_file_id_key
        UNIQUE (deployment_instance_id, config_file_id),
    DROP COLUMN IF EXISTS process_key;

UPDATE deployment_instances
SET status = 'inactive'
WHERE status = 'archived';

ALTER TABLE deployment_instances
    ADD CONSTRAINT deployment_instances_status_check
        CHECK (status IN ('active', 'inactive'));
