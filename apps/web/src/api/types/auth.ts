export interface AuthUser {
  id: number;
  username: string;
  is_platform_admin: boolean;
  must_change_password: boolean;
  status: UserStatus;
  last_login_at: string | null;
}

export interface AuthSessionResponse {
  user: AuthUser;
  auth_mode: string;
}

export type UserStatus = "active" | "disabled";

export interface LoginRequest {
  username: string;
  password: string;
}

export interface ChangePasswordRequest {
  current_password: string;
  new_password: string;
}
