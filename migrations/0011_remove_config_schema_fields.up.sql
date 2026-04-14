ALTER TABLE config_files
    DROP COLUMN IF EXISTS schema_name,
    DROP COLUMN IF EXISTS schema_version;

ALTER TABLE drafts
    DROP COLUMN IF EXISTS schema_version;
