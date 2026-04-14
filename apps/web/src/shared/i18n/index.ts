import { readonly, ref } from "vue";

export type AppLocale = "zh-CN" | "en-US";

const STORAGE_KEY = "mini-conf.locale";

const messages: Record<AppLocale, Record<string, string>> = {
  "zh-CN": {
    "locale.zh-CN": "简体中文",
    "locale.en-US": "English",
    "login.title": "mini-conf 管理台",
    "login.sessionCheckFailed": "系统异常，无法确认登录状态",
    "login.username": "用户名",
    "login.usernamePlaceholder": "请输入用户名",
    "login.password": "密码",
    "login.passwordPlaceholder": "请输入密码",
    "login.submit": "登录",
    "validation.login.usernameRequired": "请输入用户名",
    "validation.login.passwordRequired": "请输入密码",
    "projects.list.title": "项目列表",
    "projects.list.subtitle": "管理你参与的项目",
    "projects.list.empty": "暂无项目",
    "projects.list.loadFailed": "加载项目失败",
    "app.logout": "登出",
    "app.locale.zh-CN": "简体中文",
    "app.locale.en-US": "English",
    "state.notFound.title": "未找到",
    "state.notFound.subtitle": "请求的资源不存在或你没有访问权限",
    "state.forbidden.title": "权限不足",
    "state.forbidden.subtitle": "你当前角色没有执行这个操作的权限",
    "state.error.title": "加载失败",
    "state.error.subtitle": "请稍后重试",
    "state.error.retry": "重试",
    "state.empty.description": "暂无数据",
    "state.loading": "加载中…",
    "state.back": "返回",
    "status.active": "启用中",
    "status.inactive": "未启用",
    "status.archived": "已归档",
    "status.deprecated": "已废弃",
    "tabs.overview": "项目概览",
    "tabs.configFiles": "配置文件",
    "tabs.deployments": "部署实例",
    "tabs.releases": "发布记录",
    "tabs.members": "项目成员",
    "tabs.syncRecords": "同步记录",
    "tabs.heartbeats": "心跳上报",
    "tabs.auditLogs": "审计日志",
    "project.notFound.title": "项目未找到",
    "project.notFound.subtitle": "请求的项目不存在",
    "project.forbidden.subtitle": "你当前角色没有查看该项目的权限",
    "project.loadFailed": "加载项目失败",
    "projectSection.placeholder.heading": "页面已预留",
    "projectSection.deployments.title": "部署实例",
    "projectSection.deployments.subtitle": "{project} 的部署实例管理",
    "projectSection.deployments.description":
      "该页面路由已建立，后续会接入部署实例列表、模板克隆、预览和凭证管理能力。",
    "projectSection.releases.title": "发布记录",
    "projectSection.releases.subtitle": "{project} 的发布记录与差异查看",
    "projectSection.releases.description":
      "该页面路由已建立，后续会接入发布列表、详情、差异对比和发布动作主路径。",
    "projectSection.members.title": "项目成员",
    "projectSection.members.subtitle": "{project} 的成员与权限管理",
    "projectSection.members.description":
      "该页面路由已建立，后续会接入成员列表、角色调整和最后管理员保护逻辑。",
    "projectSection.syncRecords.title": "同步记录",
    "projectSection.syncRecords.subtitle": "{project} 的同步记录与结果追踪",
    "projectSection.syncRecords.description":
      "该页面路由已建立，后续会接入同步事件列表、筛选和结果详情查看。",
    "projectSection.heartbeats.title": "心跳上报",
    "projectSection.heartbeats.subtitle": "{project} 的设备心跳与进程状态",
    "projectSection.heartbeats.description":
      "该页面路由已建立，后续会接入心跳时间线、进程状态和异常提示。",
    "projectSection.auditLogs.title": "审计日志",
    "projectSection.auditLogs.subtitle": "{project} 的审计日志与操作追踪",
    "projectSection.auditLogs.description":
      "该页面路由已建立，后续会接入按资源和用户筛选的审计日志列表。",
    "project.field.code": "项目标识",
    "project.field.status": "状态",
    "project.field.description": "描述",
    "project.emptyDescription": "暂无描述",
    "configFiles.page.loadError": "加载配置文件列表失败",
    "configFiles.filter.allStatuses": "全部状态",
    "configFiles.filter.all": "全部",
    "configFiles.create": "新建配置文件",
    "configFiles.empty": "暂无配置文件",
    "configFiles.column.code": "配置标识",
    "configFiles.column.name": "名称",
    "configFiles.column.format": "格式",
    "configFiles.column.required": "必选",
    "configFiles.column.sensitivity": "敏感度",
    "configFiles.column.status": "状态",
    "configFiles.column.actions": "操作",
    "configFiles.action.edit": "编辑",
    "configFiles.required": "必选",
    "configFiles.normal": "普通",
    "configFiles.secret": "敏感",
    "configFiles.dialog.createTitle": "新建配置文件",
    "configFiles.dialog.editTitle": "编辑配置文件",
    "configFiles.dialog.unsupportedFormat":
      "当前记录使用历史格式 {format}。保存前请改成 YAML、JSON 或 TOML，才能继续当前主路径。",
    "configFiles.form.code": "配置标识",
    "configFiles.form.codePlaceholder": "如 main / vision / device-auth",
    "configFiles.form.codeHint":
      "项目内唯一标识，用于在配置、草稿和发布链路中引用。",
    "configFiles.form.name": "名称",
    "configFiles.form.namePlaceholder": "配置文件名称",
    "configFiles.form.format": "格式",
    "configFiles.form.formatPlaceholder": "选择格式",
    "configFiles.form.formatHint": "当前主路径支持 YAML / JSON / TOML。",
    "configFiles.form.sensitivity": "敏感度",
    "configFiles.form.sensitivityPlaceholder": "选择敏感度",
    "configFiles.form.sensitivity.normal": "普通 (normal)",
    "configFiles.form.sensitivity.secret": "敏感 (secret)",
    "configFiles.form.required": "必选",
    "configFiles.form.requiredHint": "开启后该配置文件为项目发布门槛",
    "configFiles.form.status": "状态",
    "configFiles.form.statusPlaceholder": "选择状态",
    "configFiles.form.statusHint":
      "当前配置文件只定义了“启用中 / 已归档”两种状态。",
    "configFiles.form.secretPaths": "Secret 路径",
    "configFiles.form.secretPathsPlaceholder":
      "每行一个 JSONPath，如 $.wifi.password",
    "configFiles.form.secretPathsHint":
      "仅在“敏感”配置时生效，用于标记需要脱敏展示和审计裁剪的路径。",
    "configFiles.form.description": "描述",
    "configFiles.form.descriptionPlaceholder": "可选描述",
    "common.cancel": "取消",
    "common.save": "保存",
    "common.create": "创建",
    "toast.configFiles.updated": "配置文件已更新",
    "toast.configFiles.created": "配置文件已创建",
    "toast.configFiles.loadFailed": "加载配置文件详情失败",
    "toast.operationFailed": "操作失败，请稍后重试",
    "validation.configFiles.codeRequired": "请输入配置标识",
    "validation.configFiles.nameRequired": "请输入名称",
    "validation.configFiles.formatRequired": "请选择格式",
    "error.auth_invalid_credentials": "用户名或密码错误",
    "error.auth_session_expired": "登录状态已过期，请重新登录",
    "error.invalid_request": "请求参数无效，请检查输入内容",
    "error.project_permission_denied": "你当前角色没有执行这个操作的权限",
    "error.project_code_conflict": "项目标识已存在",
    "error.project_not_found": "项目不存在",
    "error.config_file_code_conflict": "配置文件标识在该项目内已存在",
    "error.config_file_not_found": "配置文件不存在",
    "error.network_error": "网络连接失败，请检查网络后重试",
    "error.unknown_error": "发生未知错误，请稍后重试",
    "error.detail.invalidConfigFileFormat":
      "当前仅支持 YAML、JSON 或 TOML 格式",
    "error.detail.invalidConfigFileStatus":
      "配置文件状态仅支持“启用中 / 已归档”",
  },
  "en-US": {
    "locale.zh-CN": "简体中文",
    "locale.en-US": "English",
    "login.title": "mini-conf Console",
    "login.sessionCheckFailed":
      "A system error occurred and the login state could not be verified.",
    "login.username": "Username",
    "login.usernamePlaceholder": "Enter username",
    "login.password": "Password",
    "login.passwordPlaceholder": "Enter password",
    "login.submit": "Sign in",
    "validation.login.usernameRequired": "Please enter a username",
    "validation.login.passwordRequired": "Please enter a password",
    "projects.list.title": "Projects",
    "projects.list.subtitle": "Manage the projects you participate in",
    "projects.list.empty": "No projects yet",
    "projects.list.loadFailed": "Failed to load projects",
    "app.logout": "Logout",
    "app.locale.zh-CN": "简体中文",
    "app.locale.en-US": "English",
    "state.notFound.title": "Not Found",
    "state.notFound.subtitle":
      "The requested resource does not exist or you do not have access.",
    "state.forbidden.title": "Forbidden",
    "state.forbidden.subtitle":
      "Your current role does not have permission to perform this action.",
    "state.error.title": "Load failed",
    "state.error.subtitle": "Please try again later.",
    "state.error.retry": "Retry",
    "state.empty.description": "No data yet",
    "state.loading": "Loading…",
    "state.back": "Back",
    "status.active": "Active",
    "status.inactive": "Inactive",
    "status.archived": "Archived",
    "status.deprecated": "Deprecated",
    "tabs.overview": "Overview",
    "tabs.configFiles": "Config Files",
    "tabs.deployments": "Deployments",
    "tabs.releases": "Releases",
    "tabs.members": "Members",
    "tabs.syncRecords": "Sync Records",
    "tabs.heartbeats": "Heartbeats",
    "tabs.auditLogs": "Audit Logs",
    "project.notFound.title": "Project not found",
    "project.notFound.subtitle": "The requested project does not exist.",
    "project.forbidden.subtitle":
      "Your current role does not have permission to view this project.",
    "project.loadFailed": "Failed to load project",
    "projectSection.placeholder.heading": "Page reserved",
    "projectSection.deployments.title": "Deployments",
    "projectSection.deployments.subtitle": "Manage deployments for {project}",
    "projectSection.deployments.description":
      "This route is now in place. A later batch will connect deployment lists, template clone, preview, and credential management.",
    "projectSection.releases.title": "Releases",
    "projectSection.releases.subtitle":
      "Release history and diffs for {project}",
    "projectSection.releases.description":
      "This route is now in place. A later batch will connect release lists, details, diff views, and publish actions.",
    "projectSection.members.title": "Members",
    "projectSection.members.subtitle":
      "Member and permission management for {project}",
    "projectSection.members.description":
      "This route is now in place. A later batch will connect member lists, role changes, and last-admin protection.",
    "projectSection.syncRecords.title": "Sync Records",
    "projectSection.syncRecords.subtitle": "Sync tracking for {project}",
    "projectSection.syncRecords.description":
      "This route is now in place. A later batch will connect sync event lists, filters, and detail views.",
    "projectSection.heartbeats.title": "Heartbeats",
    "projectSection.heartbeats.subtitle": "Device heartbeats for {project}",
    "projectSection.heartbeats.description":
      "This route is now in place. A later batch will connect heartbeat timelines, process status, and anomaly hints.",
    "projectSection.auditLogs.title": "Audit Logs",
    "projectSection.auditLogs.subtitle": "Audit trail for {project}",
    "projectSection.auditLogs.description":
      "This route is now in place. A later batch will connect audit logs filtered by resource and user.",
    "project.field.code": "Project Key",
    "project.field.status": "Status",
    "project.field.description": "Description",
    "project.emptyDescription": "No description",
    "configFiles.page.loadError": "Failed to load config files",
    "configFiles.filter.allStatuses": "All statuses",
    "configFiles.filter.all": "All",
    "configFiles.create": "New Config File",
    "configFiles.empty": "No config files yet",
    "configFiles.column.code": "Config Key",
    "configFiles.column.name": "Name",
    "configFiles.column.format": "Format",
    "configFiles.column.required": "Required",
    "configFiles.column.sensitivity": "Sensitivity",
    "configFiles.column.status": "Status",
    "configFiles.column.actions": "Actions",
    "configFiles.action.edit": "Edit",
    "configFiles.required": "Required",
    "configFiles.normal": "Normal",
    "configFiles.secret": "Secret",
    "configFiles.dialog.createTitle": "Create Config File",
    "configFiles.dialog.editTitle": "Edit Config File",
    "configFiles.dialog.unsupportedFormat":
      "This record uses legacy format {format}. Change it to YAML, JSON, or TOML before saving to continue on the current path.",
    "configFiles.form.code": "Config Key",
    "configFiles.form.codePlaceholder": "e.g. main / vision / device-auth",
    "configFiles.form.codeHint":
      "Unique within the project and used across config, draft, and release flows.",
    "configFiles.form.name": "Name",
    "configFiles.form.namePlaceholder": "Config file name",
    "configFiles.form.format": "Format",
    "configFiles.form.formatPlaceholder": "Select format",
    "configFiles.form.formatHint":
      "The current main path supports YAML / JSON / TOML.",
    "configFiles.form.sensitivity": "Sensitivity",
    "configFiles.form.sensitivityPlaceholder": "Select sensitivity",
    "configFiles.form.sensitivity.normal": "Normal",
    "configFiles.form.sensitivity.secret": "Secret",
    "configFiles.form.required": "Required",
    "configFiles.form.requiredHint":
      "When enabled, this config file becomes a publish requirement.",
    "configFiles.form.status": "Status",
    "configFiles.form.statusPlaceholder": "Select status",
    "configFiles.form.statusHint":
      "Config files currently support only Active / Archived.",
    "configFiles.form.secretPaths": "Secret Paths",
    "configFiles.form.secretPathsPlaceholder":
      "One JSONPath per line, e.g. $.wifi.password",
    "configFiles.form.secretPathsHint":
      "Used only for secret configs to mark paths that must be redacted in reads and audits.",
    "configFiles.form.description": "Description",
    "configFiles.form.descriptionPlaceholder": "Optional description",
    "common.cancel": "Cancel",
    "common.save": "Save",
    "common.create": "Create",
    "toast.configFiles.updated": "Config file updated",
    "toast.configFiles.created": "Config file created",
    "toast.configFiles.loadFailed": "Failed to load config file details",
    "toast.operationFailed": "Operation failed. Please try again later.",
    "validation.configFiles.codeRequired": "Please enter a config key",
    "validation.configFiles.nameRequired": "Please enter a name",
    "validation.configFiles.formatRequired": "Please select a format",
    "error.auth_invalid_credentials": "Incorrect username or password",
    "error.auth_session_expired":
      "Your session has expired. Please log in again.",
    "error.invalid_request": "Invalid request. Please check your input.",
    "error.project_permission_denied":
      "Your current role does not have permission to perform this action.",
    "error.project_code_conflict": "Project key already exists",
    "error.project_not_found": "Project not found",
    "error.config_file_code_conflict":
      "A config file with this key already exists in the project",
    "error.config_file_not_found": "Config file not found",
    "error.network_error": "Network connection failed. Please try again.",
    "error.unknown_error": "An unknown error occurred. Please try again later.",
    "error.detail.invalidConfigFileFormat":
      "Only YAML, JSON, or TOML formats are supported right now.",
    "error.detail.invalidConfigFileStatus":
      "Config file status supports Active / Archived only.",
  },
};

function readInitialLocale(): AppLocale {
  if (typeof window === "undefined") {
    return "zh-CN";
  }

  const stored = window.localStorage.getItem(STORAGE_KEY);
  if (stored === "zh-CN" || stored === "en-US") {
    return stored;
  }

  return "zh-CN";
}

const currentLocale = ref<AppLocale>(readInitialLocale());

export function setLocale(locale: AppLocale) {
  currentLocale.value = locale;
  if (typeof window !== "undefined") {
    window.localStorage.setItem(STORAGE_KEY, locale);
  }
}

export function t(
  key: string,
  params?: Record<string, string | number | null | undefined>,
): string {
  const template =
    messages[currentLocale.value][key] ?? messages["zh-CN"][key] ?? key;

  if (!params) {
    return template;
  }

  return template.replace(/\{(\w+)\}/g, (_, token: string) => {
    const value = params[token];
    return value == null ? "" : String(value);
  });
}

export function useI18nText() {
  return {
    locale: readonly(currentLocale),
    setLocale,
    t,
  };
}
