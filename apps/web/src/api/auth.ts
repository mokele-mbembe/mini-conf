import { client } from "./client";
import type {
  AuthSessionResponse,
  ChangePasswordRequest,
  LoginRequest,
} from "./types/auth";

export function getMe(): Promise<AuthSessionResponse> {
  return client.get<AuthSessionResponse>("/auth/me");
}

export function login(body: LoginRequest): Promise<AuthSessionResponse> {
  return client.post<AuthSessionResponse>("/auth/login", body);
}

export function logout(): Promise<void> {
  return client.post<void>("/auth/logout");
}

export function changePassword(
  body: ChangePasswordRequest,
): Promise<AuthSessionResponse> {
  return client.post<AuthSessionResponse>("/auth/change-password", body);
}
