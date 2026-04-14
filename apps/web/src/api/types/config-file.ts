export interface ConfigFileSummary {
  id: number;
  project_id: number;
  code: string;
  name: string;
  format: string;
  sensitivity: string;
  is_required: boolean;
  status: string;
  description: string | null;
  secret_paths: string[] | null;
}

export interface ConfigFileListResponse {
  items: ConfigFileSummary[];
}

export interface CreateConfigFileRequest {
  project_id: number;
  code: string;
  name: string;
  format: string;
  description?: string | null;
  sensitivity?: string | null;
  secret_paths?: string[] | null;
  is_required?: boolean | null;
}

export interface UpdateConfigFileRequest {
  project_id: number;
  code: string;
  name: string;
  format: string;
  status: string;
  description?: string | null;
  sensitivity?: string | null;
  secret_paths?: string[] | null;
  is_required?: boolean | null;
}
