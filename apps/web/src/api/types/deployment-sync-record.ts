export type DeploymentSyncAction =
  | "version_check"
  | "fetch"
  | "apply"
  | "heartbeat";

export type DeploymentSyncStatus = "success" | "noop" | "failed";

export interface DeploymentSyncRecordSummary {
  id: number;
  project_id: number;
  deployment_instance_id: number;
  config_file_id: number;
  config: string;
  release_id: number | null;
  revision: string | null;
  action: DeploymentSyncAction;
  status: DeploymentSyncStatus;
  message: string | null;
  detail: unknown | null;
  reported_at: string;
}

export interface DeploymentSyncRecordListResponse {
  items: DeploymentSyncRecordSummary[];
}

export interface ListDeploymentSyncRecordsParams {
  project_id?: number;
  deployment_instance_id?: number;
  config_file_id?: number;
  action?: DeploymentSyncAction;
  status?: DeploymentSyncStatus;
}
