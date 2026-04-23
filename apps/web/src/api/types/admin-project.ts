export interface AdminProjectSummary {
  id: number;
  code: string;
  name: string;
  status: string;
  member_count: number;
  deployment_count: number;
  created_at: string;
}

export interface AdminProjectListQuery {
  keyword?: string;
  status?: string;
  page?: number;
  page_size?: number;
}

export interface AdminProjectCreateRequest {
  code: string;
  name: string;
  description?: string;
  initial_admin_user_id: number;
}

export interface AdminProject {
  id: number;
  code: string;
  name: string;
  description: string | null;
  status: string;
}

export interface AdminProjectInitialAdmin {
  user_id: number;
  username: string;
  role: string;
}

export interface AdminProjectCreateResponse {
  project: AdminProject;
  initial_admin: AdminProjectInitialAdmin;
}

export interface AdminProjectListResponse {
  items: AdminProjectSummary[];
  total: number;
  page: number;
  page_size: number;
}
