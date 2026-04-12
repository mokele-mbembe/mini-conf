import { client } from "./client";
import type { ProjectSummary, ProjectListResponse } from "./types/project";

export function listProjects(): Promise<ProjectListResponse> {
  return client.get<ProjectListResponse>("/projects");
}

export function getProject(id: number): Promise<ProjectSummary> {
  return client.get<ProjectSummary>(`/projects/${id}`);
}
