import { client } from "./client";
import type {
  AdminProjectCreateRequest,
  AdminProjectCreateResponse,
  AdminProjectListResponse,
} from "./types/admin-project";

export function listAdminProjects(): Promise<AdminProjectListResponse> {
  return client.get<AdminProjectListResponse>("/admin/projects");
}

export function createAdminProject(
  body: AdminProjectCreateRequest,
): Promise<AdminProjectCreateResponse> {
  return client.post<AdminProjectCreateResponse>("/admin/projects", body);
}
