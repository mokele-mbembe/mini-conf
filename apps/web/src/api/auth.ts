import { client } from "./client";
import type {
  AuthSessionResponse,
  ChangePasswordRequest,
  LoginRequest,
} from "./types/auth";

export function getMe(): Promise<AuthSessionResponse> {
  return client.get<AuthSessionResponse>("/auth/me");
}

export function fetchCsrf(): Promise<void> {
  return client.get<void>("/auth/csrf");
}

export async function login(body: LoginRequest): Promise<AuthSessionResponse> {
  await fetchCsrf();
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
