export const ERROR_MESSAGES: Record<string, string> = {
  auth_invalid_credentials: "用户名或密码错误",
  auth_session_expired: "登录状态已过期，请重新登录",
  project_permission_denied: "你当前角色没有执行这个操作的权限",
  project_code_conflict: "项目编码已存在",
  project_not_found: "项目不存在",
  network_error: "网络连接失败，请检查网络后重试",
  unknown_error: "发生未知错误，请稍后重试",
};

export function getErrorMessage(code: string): string {
  return ERROR_MESSAGES[code] ?? ERROR_MESSAGES.unknown_error;
}
