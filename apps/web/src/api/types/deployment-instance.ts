export type DeploymentInstanceStatus = "active" | "inactive";

export interface DeploymentInstanceSummary {
  id: number;
  deployment_uid: string;
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
  is_archived: boolean;
  archived_at: string | null;
  archived_by: number | null;
  archive_reason: string | null;
  deleted_at: string | null;
  deleted_by: number | null;
  delete_reason: string | null;
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
  is_template?: boolean;
  visibility_filter?: "current" | "archived" | "all";
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

export interface ArchiveDeploymentInstanceRequest {
  reason?: string | null;
}

export interface DeleteDeploymentInstanceRequest {
  reason?: string | null;
}

export interface ConfigBundleDeployment {
  key: string;
  name: string;
}

export interface ConfigBundleItem {
  config: string;
  revision: string;
  content_hash: string;
  format: string;
  content: string;
}

export interface ConfigBundlePreview {
  project: string;
  environment: string;
  deployment: ConfigBundleDeployment;
  configs: ConfigBundleItem[];
}

export interface DeploymentPreviewItem {
  config_file_id: number;
  code: string;
  name: string;
  is_required: boolean;
  source: string;
  status: string;
  format: string;
  content: string | null;
  revision: string | null;
}

export interface DeploymentBundlePreviewResponse {
  deployment_instance_id: number;
  items: DeploymentPreviewItem[];
  open_bundle_preview: ConfigBundlePreview;
}
