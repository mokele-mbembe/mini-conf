import { client } from "./client";
import type {
  ProjectSummary,
  ProjectListResponse,
  ProjectMemberListResponse,
} from "./types/project";

export function listProjects(): Promise<ProjectListResponse> {
  return client.get<ProjectListResponse>("/projects");
}

export function getProject(id: number): Promise<ProjectSummary> {
  return client.get<ProjectSummary>(`/projects/${id}`);
}

export function getProjectMembers(
  projectId: number,
): Promise<ProjectMemberListResponse> {
  return client.get<ProjectMemberListResponse>(
    `/projects/${projectId}/members`,
  );
}
