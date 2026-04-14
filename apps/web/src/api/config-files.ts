import { client } from "./client";
import type {
  ConfigFileSummary,
  ConfigFileListResponse,
  CreateConfigFileRequest,
  UpdateConfigFileRequest,
} from "./types/config-file";

export function listConfigFiles(params?: {
  project_id?: number;
  status?: string;
}): Promise<ConfigFileListResponse> {
  const query = new URLSearchParams();
  if (params?.project_id !== undefined) {
    query.set("project_id", String(params.project_id));
  }
  if (params?.status) {
    query.set("status", params.status);
  }
  const qs = query.toString();
  return client.get<ConfigFileListResponse>(
    `/config-files${qs ? `?${qs}` : ""}`,
  );
}

export function getConfigFile(id: number): Promise<ConfigFileSummary> {
  return client.get<ConfigFileSummary>(`/config-files/${id}`);
}

export function createConfigFile(
  body: CreateConfigFileRequest,
): Promise<ConfigFileSummary> {
  return client.post<ConfigFileSummary>("/config-files", body);
}

export function updateConfigFile(
  id: number,
  body: UpdateConfigFileRequest,
): Promise<ConfigFileSummary> {
  return client.put<ConfigFileSummary>(`/config-files/${id}`, body);
}
