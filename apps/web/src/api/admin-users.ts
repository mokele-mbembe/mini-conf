import { client } from "./client";
import type {
  AdminUserDetail,
  AdminUserCreateRequest,
  AdminUserListQuery,
  AdminUserSummary,
  AdminUserUpdateRequest,
  AdminUserResetPasswordRequest,
  AdminUserListResponse,
} from "./types/admin-user";

export function listAdminUsers(
  query: AdminUserListQuery = {},
): Promise<AdminUserListResponse> {
  const searchParams = new URLSearchParams();

  if (query.keyword) {
    searchParams.set("keyword", query.keyword);
  }

  if (query.status) {
    searchParams.set("status", query.status);
  }

  if (query.is_platform_admin !== undefined) {
    searchParams.set("is_platform_admin", String(query.is_platform_admin));
  }

  if (query.page !== undefined) {
    searchParams.set("page", String(query.page));
  }

  if (query.page_size !== undefined) {
    searchParams.set("page_size", String(query.page_size));
  }

  const suffix = searchParams.size > 0 ? `?${searchParams.toString()}` : "";
  return client.get<AdminUserListResponse>(`/admin/users${suffix}`);
}

export function getAdminUser(id: number): Promise<AdminUserDetail> {
  return client.get<AdminUserDetail>(`/admin/users/${id}`);
}

export function createAdminUser(
  body: AdminUserCreateRequest,
): Promise<AdminUserSummary> {
  return client.post<AdminUserSummary>("/admin/users", body);
}

export function updateAdminUser(
  id: number,
  body: AdminUserUpdateRequest,
): Promise<AdminUserSummary> {
  return client.patch<AdminUserSummary>(`/admin/users/${id}`, body);
}

export function resetAdminUserPassword(
  id: number,
  body: AdminUserResetPasswordRequest,
): Promise<void> {
  return client.post<void>(`/admin/users/${id}/reset-password`, body);
}
