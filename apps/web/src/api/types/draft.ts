export interface DraftResponse {
  deployment_instance_id: number;
  config_file_id: number;
  format: string;
  content: string;
  version: number;
  updated_at: string;
}

export interface UpdateDraftRequest {
  content: string;
  format: string;
  base_version?: number | null;
}

export interface CloneDraftRequest {
  source_deployment_instance_id: number;
  source_kind: "draft" | "latest_release";
}

export interface DraftCloneResponse {
  draft: DraftResponse;
  source_deployment_instance_id: number;
  source_kind: "draft" | "latest_release";
}
