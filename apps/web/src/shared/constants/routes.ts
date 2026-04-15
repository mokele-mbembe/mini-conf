export const ROUTE_NAMES = {
  LOGIN: "Login",
  PROJECTS: "Projects",
  PROJECT_OVERVIEW: "ProjectOverview",
  CONFIG_FILE_LIST: "ConfigFileList",
  DEPLOYMENT_LIST: "DeploymentList",
  DEPLOYMENT_DETAIL: "DeploymentDetail",
  RELEASE_LIST: "ReleaseList",
  PROJECT_MEMBERS: "ProjectMembers",
  SYNC_RECORD_LIST: "SyncRecordList",
  HEARTBEAT_LIST: "HeartbeatList",
  AUDIT_LOG_LIST: "AuditLogList",
} as const;

export const ROUTE_PATHS = {
  LOGIN: "/login",
  PROJECTS: "/projects",
  PROJECT: "/projects/:projectId",
  CONFIG_FILE_LIST: "/projects/:projectId/config-files",
  DEPLOYMENT_LIST: "/projects/:projectId/deployments",
  DEPLOYMENT_DETAIL: "/projects/:projectId/deployments/:deploymentId",
  RELEASE_LIST: "/projects/:projectId/releases",
  PROJECT_MEMBERS: "/projects/:projectId/members",
  SYNC_RECORD_LIST: "/projects/:projectId/sync-records",
  HEARTBEAT_LIST: "/projects/:projectId/heartbeats",
  AUDIT_LOG_LIST: "/projects/:projectId/audit-logs",
} as const;
