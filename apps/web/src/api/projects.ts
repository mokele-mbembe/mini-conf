import { client } from "./client";
import type {
  ProjectSummary,
  ProjectListResponse,
  ProjectMember,
  ProjectMemberCreateRequest,
  ProjectMemberListResponse,
  ProjectMemberUpdateRequest,
} from "./types/project";

export function listProjects(): Promise<ProjectListResponse> {
  return client.get<ProjectListResponse>("/projects");
}

export function getProject(id: number): Promise<ProjectSummary> {
  return client.get<ProjectSummary>(`/projects/${id}`);
}

export function deleteProject(id: number): Promise<void> {
  return client.delete<void>(`/projects/${id}`);
}

export function getProjectMembers(
  projectId: number,
): Promise<ProjectMemberListResponse> {
  return client.get<ProjectMemberListResponse>(
    `/projects/${projectId}/members`,
  );
}

export function createProjectMember(
  projectId: number,
  body: ProjectMemberCreateRequest,
): Promise<ProjectMember> {
  return client.post<ProjectMember>(`/projects/${projectId}/members`, body);
}

export function updateProjectMember(
  projectId: number,
  memberId: number,
  body: ProjectMemberUpdateRequest,
): Promise<ProjectMember> {
  return client.put<ProjectMember>(
    `/projects/${projectId}/members/${memberId}`,
    body,
  );
}

export function deleteProjectMember(
  projectId: number,
  memberId: number,
): Promise<void> {
  return client.delete<void>(`/projects/${projectId}/members/${memberId}`);
}
