export interface CloneSourceAvailability {
  draft: boolean;
  latest_release: boolean;
}

export interface CloneSourceSummary {
  deployment_instance_id: number;
  deployment_key: string;
  name: string;
  environment_id: number;
  environment_name: string;
  is_template: boolean;
  available_sources: CloneSourceAvailability;
}

export interface CloneSourceListResponse {
  items: CloneSourceSummary[];
  next_cursor: number | null;
}

export interface ListCloneSourcesParams {
  project_id: number;
  target_deployment_id: number;
  config_file_id: number;
  keyword?: string;
  limit?: number;
  cursor?: number;
}
