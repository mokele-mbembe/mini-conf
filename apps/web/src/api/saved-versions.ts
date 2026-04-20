import { client } from "./client";
import type {
  RestoreSavedVersionRequestBody,
  SavedVersionDetailResponse,
  SavedVersionListResponse,
  SavedVersionRestoreResponse,
  UpdateSavedVersionRequestBody,
} from "./types/saved-version";

export function listSavedVersions(params?: {
  deployment_instance_id?: number;
  config_file_id?: number;
}): Promise<SavedVersionListResponse> {
  const query = new URLSearchParams();
  if (params?.deployment_instance_id !== undefined) {
    query.set("deployment_instance_id", String(params.deployment_instance_id));
  }
  if (params?.config_file_id !== undefined) {
    query.set("config_file_id", String(params.config_file_id));
  }
  const qs = query.toString();
  return client.get<SavedVersionListResponse>(
    `/draft-saved-versions${qs ? `?${qs}` : ""}`,
  );
}

export function getSavedVersion(
  id: number,
): Promise<SavedVersionDetailResponse> {
  return client.get<SavedVersionDetailResponse>(`/draft-saved-versions/${id}`);
}

export function updateSavedVersion(
  id: number,
  body: UpdateSavedVersionRequestBody,
): Promise<SavedVersionDetailResponse> {
  return client.patch<SavedVersionDetailResponse>(
    `/draft-saved-versions/${id}`,
    body,
  );
}

export function restoreSavedVersion(
  id: number,
  body: RestoreSavedVersionRequestBody,
): Promise<SavedVersionRestoreResponse> {
  return client.post<SavedVersionRestoreResponse>(
    `/draft-saved-versions/${id}/restore`,
    body,
  );
}

export function deleteSavedVersion(id: number): Promise<void> {
  return client.delete(`/draft-saved-versions/${id}`);
}
