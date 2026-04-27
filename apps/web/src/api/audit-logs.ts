import { client } from "./client";
import type {
  AuditLogListResponse,
  ListAuditLogsParams,
} from "./types/audit-log";

function buildAuditLogQuery(params?: ListAuditLogsParams): string {
  const query = new URLSearchParams();
  if (params?.project_id !== undefined) {
    query.set("project_id", String(params.project_id));
  }
  if (params?.user_id !== undefined) {
    query.set("user_id", String(params.user_id));
  }
  if (params?.action) {
    query.set("action", params.action);
  }
  if (params?.resource_type) {
    query.set("resource_type", params.resource_type);
  }
  return query.toString();
}

export function listAuditLogs(
  params?: ListAuditLogsParams,
): Promise<AuditLogListResponse> {
  const qs = buildAuditLogQuery(params);
  return client.get<AuditLogListResponse>(`/audit-logs${qs ? `?${qs}` : ""}`);
}
