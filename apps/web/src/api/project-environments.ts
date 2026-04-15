import { client } from "./client";
import type {
  CreateProjectEnvironmentRequest,
  ProjectEnvironmentListResponse,
  ProjectEnvironmentSummary,
  UpdateProjectEnvironmentRequest,
} from "./types/project-environment";

export function listProjectEnvironments(
  projectId: number,
): Promise<ProjectEnvironmentListResponse> {
  return client.get<ProjectEnvironmentListResponse>(
    `/projects/${projectId}/environments`,
  );
}

export function getProjectEnvironment(
  projectId: number,
  environmentId: number,
): Promise<ProjectEnvironmentSummary> {
  return client.get<ProjectEnvironmentSummary>(
    `/projects/${projectId}/environments/${environmentId}`,
  );
}

export function createProjectEnvironment(
  projectId: number,
  body: CreateProjectEnvironmentRequest,
): Promise<ProjectEnvironmentSummary> {
  return client.post<ProjectEnvironmentSummary>(
    `/projects/${projectId}/environments`,
    body,
  );
}

export function updateProjectEnvironment(
  projectId: number,
  environmentId: number,
  body: UpdateProjectEnvironmentRequest,
): Promise<ProjectEnvironmentSummary> {
  return client.put<ProjectEnvironmentSummary>(
    `/projects/${projectId}/environments/${environmentId}`,
    body,
  );
}

export function deleteProjectEnvironment(
  projectId: number,
  environmentId: number,
): Promise<void> {
  return client.delete<void>(
    `/projects/${projectId}/environments/${environmentId}`,
  );
}
