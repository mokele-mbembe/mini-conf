import { client } from "./client";
import type {
  CloneSourceListResponse,
  ListCloneSourcesParams,
} from "./types/clone-source";

function buildCloneSourcesQuery(params: ListCloneSourcesParams): string {
  const query = new URLSearchParams();
  query.set("project_id", String(params.project_id));
  query.set("target_deployment_id", String(params.target_deployment_id));
  query.set("config_file_id", String(params.config_file_id));
  if (params.keyword) {
    query.set("keyword", params.keyword);
  }
  if (params.limit !== undefined) {
    query.set("limit", String(params.limit));
  }
  if (params.cursor !== undefined) {
    query.set("cursor", String(params.cursor));
  }
  return query.toString();
}

export function listCloneSources(
  params: ListCloneSourcesParams,
): Promise<CloneSourceListResponse> {
  const qs = buildCloneSourcesQuery(params);
  return client.get<CloneSourceListResponse>(`/clone-sources?${qs}`);
}
