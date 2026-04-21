// ── Demo control API client (:19010, proxied via /api/demo) ─────────────

const DEMO_BASE = "/api/demo";

export interface DemoState {
  config_center_url: string;
  project_id: number | null;
  project_code: string;
  sn_bindings: Record<string, Record<string, string>>; // backend -> {SN -> dep_key}
  clients: Record<string, ClientState>;
  events: DemoEvent[];
  effective_configs: Record<string, string>; // filename -> content
  cc_instances: CcInstance[];
  cc_releases: CcRelease[];
  cc_heartbeats: CcHeartbeat[];
  cc_sync_records: CcSyncRecord[];
}

export interface ClientState {
  client_id: string;
  backend_name: string;
  sn: string;
  status: "idle" | "bootstrapped" | "polling" | "paused" | "error";
  token: string | null;
  last_bootstrap: BootstrapResponse | null;
  last_bundle: BundleResponse | null;
  last_revisions: Record<string, string>;
  last_error: string | null;
}

export interface BootstrapResponse {
  config_center_base_url: string;
  project: string;
  environment: string;
  deployment_key: string;
  token: string;
  configs: string[];
}

export interface BundleResponse {
  configs: Array<{
    config: string;
    revision: string;
    content: string;
    format: string;
    content_hash: string;
  }>;
}

export interface DemoEvent {
  at: string;
  kind: string;
  client?: string;
  backend?: string;
  sn?: string;
  deployment_key?: string;
  config?: string;
  revision?: string;
  status?: string;
  message?: string;
}

// Config-center data shapes (subset of what the API returns)
export interface CcInstance {
  id: number;
  deployment_key: string;
  status: string;
  is_template: boolean;
  is_archived: boolean;
  deleted_at: string | null;
}

export interface CcRelease {
  id: number;
  deployment_instance_id: number;
  config_file_id: number;
  revision: string;
  published_at: string;
}

export interface CcHeartbeat {
  id: number;
  deployment_instance_id: number;
  config: string;
  config_file_id: number;
  metadata: unknown;
  reported_at: string;
}

export interface CcSyncRecord {
  id: number;
  deployment_instance_id: number;
  config: string;
  revision: string | null;
  action: string;
  status: string;
  message: string | null;
  reported_at: string;
}

// ── API helpers ──────────────────────────────────────────────────────────

async function api<T>(
  method: string,
  path: string,
  body?: unknown,
): Promise<T> {
  const opts: RequestInit = {
    method,
    headers: { "Content-Type": "application/json" },
  };
  if (body !== undefined) opts.body = JSON.stringify(body);
  const res = await fetch(`${DEMO_BASE}${path}`, opts);
  const text = await res.text();
  const json = text ? JSON.parse(text) : {};
  if (!res.ok) throw new Error(json.message ?? `HTTP ${res.status}`);
  return json as T;
}

export function fetchState(): Promise<DemoState> {
  return api("GET", "/state");
}

export function clientBootstrap(
  clientId: string,
): Promise<{ ok: boolean; message: string }> {
  return api("POST", `/clients/${clientId}/bootstrap`);
}

export function clientPull(
  clientId: string,
): Promise<{ ok: boolean; message: string }> {
  return api("POST", `/clients/${clientId}/pull`);
}

export function clientApply(
  clientId: string,
): Promise<{ ok: boolean; message: string }> {
  return api("POST", `/clients/${clientId}/apply`);
}

export function clientHeartbeat(
  clientId: string,
): Promise<{ ok: boolean; message: string }> {
  return api("POST", `/clients/${clientId}/heartbeat`);
}

export function clientStartPoll(clientId: string): Promise<{ ok: boolean }> {
  return api("POST", `/clients/${clientId}/poll/start`);
}

export function clientPausePoll(clientId: string): Promise<{ ok: boolean }> {
  return api("POST", `/clients/${clientId}/poll/pause`);
}

export function clientSetToken(
  clientId: string,
  token: string,
): Promise<{ ok: boolean }> {
  return api("POST", `/clients/${clientId}/token`, { token });
}

export function clientCreate(
  clientId: string,
  backendName: string,
  sn: string,
): Promise<{ ok: boolean }> {
  return api("POST", "/clients", {
    client_id: clientId,
    backend_name: backendName,
    sn,
  });
}

export function bindSn(
  backend: string,
  sn: string,
  deploymentKey: string,
): Promise<{ ok: boolean }> {
  return api("POST", `/backends/${backend}/sn-bindings`, {
    sn,
    deployment_key: deploymentKey,
  });
}

export function unbindSn(
  backend: string,
  sn: string,
): Promise<{ ok: boolean }> {
  return api("DELETE", `/backends/${backend}/sn-bindings/${sn}`);
}

export function clearEvents(): Promise<{ ok: boolean }> {
  return api("POST", "/events/clear");
}
