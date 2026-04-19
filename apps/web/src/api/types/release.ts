export interface ReleaseSummary {
  id: number;
  project_id: number;
  deployment_instance_id: number;
  config_file_id: number;
  revision: string;
  content_hash: string;
  format: string;
  change_summary: string | null;
  apply_mode: string;
  published_by: number;
  published_at: string;
}

export interface ReleaseListResponse {
  items: ReleaseSummary[];
}

export interface ReleaseDiffSummary {
  is_initial: boolean;
  has_changes: boolean;
  added_lines: number;
  removed_lines: number;
}

export interface ReleaseDetailResponse {
  release: ReleaseSummary;
  content: string;
  diff_summary: ReleaseDiffSummary | null;
  content_redacted: boolean;
}

export interface ReleaseDiffResponse {
  release: ReleaseSummary;
  base_release: ReleaseSummary | null;
  before_content: string | null;
  after_content: string;
  diff_summary: ReleaseDiffSummary;
  before_redacted: boolean;
  after_redacted: boolean;
}

export interface PublishReleaseRequest {
  project_id: number;
  deployment_instance_id: number;
  config_file_id: number;
  change_summary?: string | null;
}
