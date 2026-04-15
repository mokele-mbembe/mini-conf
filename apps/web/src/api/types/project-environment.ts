export type ProjectEnvironmentStatus = "active" | "inactive";

export interface ProjectEnvironmentSummary {
  id: number;
  project_id: number;
  code: string;
  name: string;
  description: string | null;
  status: ProjectEnvironmentStatus;
  sort_order: number;
  deployment_count: number;
}

export interface ProjectEnvironmentListResponse {
  items: ProjectEnvironmentSummary[];
}

export interface CreateProjectEnvironmentRequest {
  code: string;
  name: string;
  description?: string | null;
  status?: ProjectEnvironmentStatus | null;
  sort_order?: number | null;
}

export interface UpdateProjectEnvironmentRequest {
  name: string;
  description?: string | null;
  status?: ProjectEnvironmentStatus | null;
  sort_order?: number | null;
}
