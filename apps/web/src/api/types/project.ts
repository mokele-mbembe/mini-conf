export interface ProjectSummary {
  id: number;
  code: string;
  name: string;
  status: string;
  description: string | null;
  current_user_role: ProjectRole;
}

export interface ProjectListResponse {
  items: ProjectSummary[];
}

export type ProjectRole = "admin" | "editor" | "viewer";

export interface ProjectMember {
  id: number;
  project_id: number;
  user_id: number;
  username: string;
  role: ProjectRole;
  created_at: string;
}

export interface ProjectMemberListResponse {
  items: ProjectMember[];
}
