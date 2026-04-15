import { client } from "./client";
import type {
  DeploymentHeartbeatListResponse,
  ListDeploymentHeartbeatsParams,
} from "./types/deployment-heartbeat";

function buildDeploymentHeartbeatQuery(
  params?: ListDeploymentHeartbeatsParams,
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
  return query.toString();
}

export function listDeploymentHeartbeats(
  params?: ListDeploymentHeartbeatsParams,
): Promise<DeploymentHeartbeatListResponse> {
  const qs = buildDeploymentHeartbeatQuery(params);
  return client.get<DeploymentHeartbeatListResponse>(
    `/deployment-heartbeats${qs ? `?${qs}` : ""}`,
  );
}
