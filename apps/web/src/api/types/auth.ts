export interface AuthUser {
  id: number;
  username: string;
}

export interface AuthSessionResponse {
  user: AuthUser;
  auth_mode: string;
}

export interface LoginRequest {
  username: string;
  password: string;
}
