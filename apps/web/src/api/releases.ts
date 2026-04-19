import { client } from "./client";
import type {
  PublishReleaseRequest,
  ReleaseDetailResponse,
  ReleaseDiffResponse,
  ReleaseListResponse,
  ReleaseSummary,
} from "./types/release";

export function listReleases(params?: {
  project_id?: number;
  deployment_instance_id?: number;
  config_file_id?: number;
}): Promise<ReleaseListResponse> {
  const query = new URLSearchParams();
  if (params?.project_id !== undefined) {
    query.set("project_id", String(params.project_id));
  }
  if (params?.deployment_instance_id !== undefined) {
    query.set("deployment_instance_id", String(params.deployment_instance_id));
  }
  if (params?.config_file_id !== undefined) {
    query.set("config_file_id", String(params.config_file_id));
  }
  const qs = query.toString();
  return client.get<ReleaseListResponse>(`/releases${qs ? `?${qs}` : ""}`);
}

export function publishRelease(
  body: PublishReleaseRequest,
): Promise<ReleaseSummary> {
  return client.post<ReleaseSummary>("/releases/publish", body);
}

export function getReleaseDetail(id: number): Promise<ReleaseDetailResponse> {
  return client.get<ReleaseDetailResponse>(`/releases/${id}`);
}

export function getReleaseDiff(id: number): Promise<ReleaseDiffResponse> {
  return client.get<ReleaseDiffResponse>(`/releases/${id}/diff`);
}
