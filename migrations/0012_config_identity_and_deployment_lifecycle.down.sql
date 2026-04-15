ALTER TABLE deployment_instances
    DROP CONSTRAINT IF EXISTS deployment_instances_status_check;

ALTER TABLE deployment_heartbeats
    ADD COLUMN IF NOT EXISTS process_key VARCHAR(64) NULL;

UPDATE deployment_heartbeats dh
SET process_key = cf.code
FROM config_files cf
WHERE cf.id = dh.config_file_id;

ALTER TABLE deployment_heartbeats
    ALTER COLUMN process_key SET NOT NULL,
    DROP CONSTRAINT IF EXISTS deployment_heartbeats_deployment_instance_id_config_file_id_key,
    ADD CONSTRAINT deployment_heartbeats_deployment_instance_id_process_key_key
        UNIQUE (deployment_instance_id, process_key),
    DROP COLUMN IF EXISTS config_file_id;

ALTER TABLE deployment_sync_records
    ADD COLUMN IF NOT EXISTS process_key VARCHAR(64) NULL,
    ALTER COLUMN config_file_id DROP NOT NULL;
