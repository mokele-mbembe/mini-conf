export type DeploymentInstanceStatus = "active" | "inactive";

export interface DeploymentInstanceSummary {
  id: number;
  project_id: number;
  environment: string;
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
  environment?: string;
  status?: DeploymentInstanceStatus;
  keyword?: string;
  page?: number;
  page_size?: number;
}

export interface CreateDeploymentInstanceRequest {
  project_id: number;
  environment: string;
  deployment_key: string;
  name: string;
  description?: string | null;
  is_template?: boolean | null;
}

export interface UpdateDeploymentInstanceRequest {
  environment: string;
  deployment_key: string;
  name: string;
  description?: string | null;
}

export interface CloneDeploymentInstanceRequest {
  deployment_key: string;
  name: string;
  environment: string;
  description?: string | null;
  clone_source: "draft";
}

export interface DeploymentTokenResponse {
  deployment_instance_id: number;
  credential_name: string;
  token_preview: string;
  token: string;
}
