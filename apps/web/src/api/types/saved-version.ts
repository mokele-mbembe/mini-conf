import type { DraftResponse } from "./draft";

export interface SavedVersionSummary {
  id: number;
  project_id: number;
  deployment_instance_id: number;
  config_file_id: number;
  title: string;
  note: string | null;
  format: string;
  source_draft_version: number;
  created_by: number;
  created_by_username: string;
  created_at: string;
}

export interface SavedVersionListResponse {
  items: SavedVersionSummary[];
}

export interface SavedVersionDetail extends SavedVersionSummary {
  content: string;
}

export interface SavedVersionDetailResponse {
  saved_version: SavedVersionDetail;
}

export interface SavedVersionRestoreResponse {
  draft: DraftResponse;
}

export interface UpdateSavedVersionRequestBody {
  note?: string | null;
}

export interface RestoreSavedVersionRequestBody {
  base_version?: number | null;
}
