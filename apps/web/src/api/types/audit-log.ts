export interface AuditLogSummary {
  id: number;
  project_id: number | null;
  user_id: number | null;
  action: string;
  resource_type: string;
  resource_id: string;
  detail: unknown | null;
  created_at: string;
}

export interface AuditLogListResponse {
  items: AuditLogSummary[];
}

export interface ListAuditLogsParams {
  project_id?: number;
  user_id?: number;
  action?: string;
  resource_type?: string;
}
