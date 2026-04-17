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
  deployment_not_found: "error.deployment_instance_not_found",
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
