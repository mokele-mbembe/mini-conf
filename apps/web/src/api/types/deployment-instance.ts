export type DeploymentInstanceStatus = "active" | "inactive";

export interface DeploymentInstanceSummary {
  id: number;
  project_id: number;
  environment_id: number;
  environment_code: string;
  environment_name: string;
  deployment_key: string;
  name: string;
  description: string | null;
  is_template: boolean;
  template_source_id: number | null;
  status: DeploymentInstanceStatus;
}

export interface DeploymentInstanceListResponse {
  items: DeploymentInstanceSummary[];
  total: number;
  page: number;
  page_size: number;
}

export interface ListDeploymentInstancesParams {
  project_id?: number;
  environment_id?: number;
  status?: DeploymentInstanceStatus;
  keyword?: string;
  page?: number;
  page_size?: number;
}

export interface CreateDeploymentInstanceRequest {
  project_id: number;
  environment_id: number;
  deployment_key: string;
  name: string;
  description?: string | null;
  is_template?: boolean | null;
}

export interface UpdateDeploymentInstanceRequest {
  environment_id: number;
  deployment_key: string;
  name: string;
  description?: string | null;
}

export interface CloneDeploymentInstanceRequest {
  deployment_key: string;
  name: string;
  environment_id: number;
  description?: string | null;
  clone_source: "draft";
}

export interface DeploymentTokenResponse {
  deployment_instance_id: number;
  credential_name: string;
  token_preview: string;
  token: string;
}
