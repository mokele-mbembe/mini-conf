import { client } from "./client";
import type {
  DeploymentSyncRecordListResponse,
  ListDeploymentSyncRecordsParams,
} from "./types/deployment-sync-record";

function buildDeploymentSyncRecordQuery(
  params?: ListDeploymentSyncRecordsParams,
): string {
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
  if (params?.action) {
    query.set("action", params.action);
  }
  if (params?.status) {
    query.set("status", params.status);
  }
  return query.toString();
}

export function listDeploymentSyncRecords(
  params?: ListDeploymentSyncRecordsParams,
): Promise<DeploymentSyncRecordListResponse> {
  const qs = buildDeploymentSyncRecordQuery(params);
  return client.get<DeploymentSyncRecordListResponse>(
    `/deployment-sync-records${qs ? `?${qs}` : ""}`,
  );
}
