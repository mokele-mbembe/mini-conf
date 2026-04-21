import { t } from "@/shared/i18n";

export const ERROR_MESSAGES: Record<string, string> = {
  auth_invalid_credentials: "error.auth_invalid_credentials",
  auth_session_expired: "error.auth_session_expired",
  invalid_request: "error.invalid_request",
  project_permission_denied: "error.project_permission_denied",
  project_code_conflict: "error.project_code_conflict",
  project_not_found: "error.project_not_found",
  project_environment_not_found: "error.project_environment_not_found",
  project_environment_code_conflict: "error.project_environment_code_conflict",
  project_environment_in_use: "error.project_environment_in_use",
  project_environment_inactive: "error.project_environment_inactive",
  config_file_code_conflict: "error.config_file_code_conflict",
  config_file_not_found: "error.config_file_not_found",
  deployment_instance_conflict: "error.deployment_instance_conflict",
  deployment_key_conflict: "error.deployment_key_conflict",
  deployment_instance_not_found: "error.deployment_instance_not_found",
  deployment_instance_not_template: "error.deployment_instance_not_template",
  deployment_instance_inactive: "error.deployment_instance_inactive",
  deployment_instance_template_token_forbidden:
    "error.deployment_instance_template_token_forbidden",
  deployment_instance_template_activate_forbidden:
    "error.deployment_instance_template_activate_forbidden",
  deployment_instance_activate_conflict:
    "error.deployment_instance_activate_conflict",
  deployment_instance_template_deactivate_forbidden:
    "error.deployment_instance_template_deactivate_forbidden",
  deployment_instance_deactivate_conflict:
    "error.deployment_instance_deactivate_conflict",
  deployment_instance_archived: "error.deployment_instance_archived",
  deployment_instance_deleted: "error.deployment_instance_deleted",
  deployment_instance_archive_conflict:
    "error.deployment_instance_archive_conflict",
  deployment_instance_restore_conflict:
    "error.deployment_instance_restore_conflict",
  deployment_instance_delete_conflict:
    "error.deployment_instance_delete_conflict",
  deployment_not_found: "error.deployment_instance_not_found",
  draft_not_found: "error.draft_not_found",
  draft_version_conflict: "error.draft_version_conflict",
  draft_validation_failed: "error.draft_validation_failed",
  draft_clone_cross_project_forbidden:
    "error.draft_clone_cross_project_forbidden",
  required_config_missing: "error.required_config_missing",
  deployment_instance_template_publish_forbidden:
    "error.deployment_instance_template_publish_forbidden",
  release_publish_failed: "error.release_publish_failed",
  release_not_found: "error.release_not_found",
  saved_version_not_found: "error.saved_version_not_found",
  saved_version_note_too_long: "error.saved_version_note_too_long",
  network_error: "error.network_error",
  unknown_error: "error.unknown_error",
};

const ERROR_DETAIL_MESSAGES: Record<string, string> = {
  "invalid config file format": "error.detail.invalidConfigFileFormat",
  "invalid config file status": "error.detail.invalidConfigFileStatus",
};

export function getErrorMessage(code: string, detail?: string): string {
  if (detail && ERROR_DETAIL_MESSAGES[detail]) {
    return t(ERROR_DETAIL_MESSAGES[detail]);
  }

  const key = ERROR_MESSAGES[code] ?? ERROR_MESSAGES.unknown_error;
  return t(key);
}
