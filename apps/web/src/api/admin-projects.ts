import { client } from "./client";
import type {
  AdminProjectCreateRequest,
  AdminProjectCreateResponse,
  AdminProjectListQuery,
  AdminProjectListResponse,
} from "./types/admin-project";

export function listAdminProjects(
  query: AdminProjectListQuery = {},
): Promise<AdminProjectListResponse> {
  const searchParams = new URLSearchParams();

  if (query.keyword) {
    searchParams.set("keyword", query.keyword);
  }

  if (query.status) {
    searchParams.set("status", query.status);
  }

  if (query.page !== undefined) {
    searchParams.set("page", String(query.page));
  }

  if (query.page_size !== undefined) {
    searchParams.set("page_size", String(query.page_size));
  }

  const suffix = searchParams.size > 0 ? `?${searchParams.toString()}` : "";
  return client.get<AdminProjectListResponse>(`/admin/projects${suffix}`);
}

export function createAdminProject(
  body: AdminProjectCreateRequest,
): Promise<AdminProjectCreateResponse> {
  return client.post<AdminProjectCreateResponse>("/admin/projects", body);
}
