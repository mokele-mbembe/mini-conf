import { client } from "./client";
import type {
  CloneDeploymentInstanceRequest,
  CreateDeploymentInstanceRequest,
  DeploymentInstanceListResponse,
  DeploymentInstanceSummary,
  DeploymentTokenResponse,
  ListDeploymentInstancesParams,
  UpdateDeploymentInstanceRequest,
} from "./types/deployment-instance";

function buildDeploymentInstanceQuery(
  params?: ListDeploymentInstancesParams,
): string {
  const query = new URLSearchParams();
  if (params?.project_id !== undefined) {
    query.set("project_id", String(params.project_id));
  }
  if (params?.environment) {
    query.set("environment", params.environment);
  }
  if (params?.status) {
    query.set("status", params.status);
  }
  if (params?.keyword) {
    query.set("keyword", params.keyword);
  }
  if (params?.page !== undefined) {
    query.set("page", String(params.page));
  }
  if (params?.page_size !== undefined) {
    query.set("page_size", String(params.page_size));
  }
  return query.toString();
}

export function listDeploymentInstances(
  params?: ListDeploymentInstancesParams,
): Promise<DeploymentInstanceListResponse> {
  const qs = buildDeploymentInstanceQuery(params);
  return client.get<DeploymentInstanceListResponse>(
    `/deployment-instances${qs ? `?${qs}` : ""}`,
  );
}

export function getDeploymentInstance(
  id: number,
): Promise<DeploymentInstanceSummary> {
  return client.get<DeploymentInstanceSummary>(`/deployment-instances/${id}`);
}

export function createDeploymentInstance(
  body: CreateDeploymentInstanceRequest,
): Promise<DeploymentInstanceSummary> {
  return client.post<DeploymentInstanceSummary>("/deployment-instances", body);
}

export function updateDeploymentInstance(
  id: number,
  body: UpdateDeploymentInstanceRequest,
): Promise<DeploymentInstanceSummary> {
  return client.put<DeploymentInstanceSummary>(
    `/deployment-instances/${id}`,
    body,
  );
}

export function cloneDeploymentInstance(
  templateId: number,
  body: CloneDeploymentInstanceRequest,
): Promise<DeploymentInstanceSummary> {
  return client.post<DeploymentInstanceSummary>(
    `/deployment-instances/${templateId}/clone`,
    body,
  );
}

export function activateDeploymentInstance(
  id: number,
): Promise<DeploymentTokenResponse> {
  return client.post<DeploymentTokenResponse>(
    `/deployment-instances/${id}/activate`,
  );
}

export function deactivateDeploymentInstance(id: number): Promise<void> {
  return client.post<void>(`/deployment-instances/${id}/deactivate`);
}

export function resetDeploymentToken(
  id: number,
): Promise<DeploymentTokenResponse> {
  return client.post<DeploymentTokenResponse>(
    `/deployment-instances/${id}/token/reset`,
  );
}
