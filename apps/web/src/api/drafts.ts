import { client } from "./client";
import type {
  CloneDraftRequest,
  DraftCloneResponse,
  DraftResponse,
  UpdateDraftRequest,
} from "./types/draft";

export function getDraft(
  deploymentId: number,
  configFileId: number,
): Promise<DraftResponse> {
  return client.get<DraftResponse>(`/drafts/${deploymentId}/${configFileId}`);
}

export function updateDraft(
  deploymentId: number,
  configFileId: number,
  body: UpdateDraftRequest,
): Promise<DraftResponse> {
  return client.put<DraftResponse>(
    `/drafts/${deploymentId}/${configFileId}`,
    body,
  );
}

export function cloneDraft(
  targetDeploymentId: number,
  configFileId: number,
  body: CloneDraftRequest,
): Promise<DraftCloneResponse> {
  return client.post<DraftCloneResponse>(
    `/drafts/${targetDeploymentId}/${configFileId}/clone`,
    body,
  );
}

export function deleteDraft(
  deploymentId: number,
  configFileId: number,
): Promise<void> {
  return client.delete(`/drafts/${deploymentId}/${configFileId}`);
}
