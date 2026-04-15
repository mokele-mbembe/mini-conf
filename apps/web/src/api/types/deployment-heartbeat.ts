export interface DeploymentHeartbeatSummary {
  id: number;
  project_id: number;
  deployment_instance_id: number;
  config_file_id: number;
  config: string;
  metadata: unknown | null;
  reported_at: string;
}

export interface DeploymentHeartbeatListResponse {
  items: DeploymentHeartbeatSummary[];
}

export interface ListDeploymentHeartbeatsParams {
  project_id?: number;
  deployment_instance_id?: number;
  config_file_id?: number;
}
