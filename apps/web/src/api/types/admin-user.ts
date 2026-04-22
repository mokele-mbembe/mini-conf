export type UserStatus = "active" | "disabled";

export interface AdminUserSummary {
  id: number;
  username: string;
  status: UserStatus;
  is_platform_admin: boolean;
  must_change_password: boolean;
  last_login_at: string | null;
  password_updated_at: string | null;
  project_count: number;
  created_at: string;
}

export type AdminUser = AdminUserSummary;

export interface AdminUserProjectSummary {
  project_id: number;
  project_code: string;
  project_name: string;
  role: string;
}

export interface AdminUserDetail {
  id: number;
  username: string;
  status: UserStatus;
  is_platform_admin: boolean;
  must_change_password: boolean;
  last_login_at: string | null;
  password_updated_at: string | null;
  projects: AdminUserProjectSummary[];
  created_at: string;
}

export interface AdminUserCreateRequest {
  username: string;
  password: string;
  status: UserStatus;
  is_platform_admin: boolean;
  must_change_password: boolean;
}

export interface AdminUserUpdateRequest {
  status?: UserStatus;
  is_platform_admin?: boolean;
  must_change_password?: boolean;
}

export interface AdminUserResetPasswordRequest {
  new_password: string;
  must_change_password: boolean;
}

export interface AdminUserListResponse {
  items: AdminUserSummary[];
  total: number;
  page: number;
  page_size: number;
}

export interface AdminUserListQuery {
  keyword?: string;
  status?: UserStatus;
  is_platform_admin?: boolean;
  page?: number;
  page_size?: number;
}
