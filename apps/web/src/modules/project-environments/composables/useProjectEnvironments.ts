import { ref } from "vue";
import * as projectEnvironmentsApi from "@/api/project-environments";
import type { ProjectEnvironmentSummary } from "@/api/types/project-environment";

interface CacheEntry {
  data: ProjectEnvironmentSummary[];
  timestamp: number;
}

const CACHE_TTL_MS = 30_000; // 30 seconds
const _cache = new Map<number, CacheEntry>();

function sortEnvironments(
  items: ProjectEnvironmentSummary[],
): ProjectEnvironmentSummary[] {
  return [...items].sort((a, b) => {
    if (a.sort_order !== b.sort_order) return a.sort_order - b.sort_order;
    return a.code.localeCompare(b.code);
  });
}

/**
 * @param getProjectId - getter that returns the current project id.
 *   Evaluated on every `load()` call so it stays correct when the route changes.
 */
export function useProjectEnvironments(getProjectId: () => number) {
  const environments = ref<ProjectEnvironmentSummary[]>([]);
  const loading = ref(false);

  async function load() {
    const projectId = getProjectId();
    const cached = _cache.get(projectId);
    if (cached && Date.now() - cached.timestamp < CACHE_TTL_MS) {
      environments.value = cached.data;
      return;
    }

    loading.value = true;
    try {
      const res =
        await projectEnvironmentsApi.listProjectEnvironments(projectId);
      const sorted = sortEnvironments(res.items);
      _cache.set(projectId, { data: sorted, timestamp: Date.now() });
      environments.value = sorted;
    } catch {
      environments.value = [];
    } finally {
      loading.value = false;
    }
  }

  /** Force-refresh, bypassing cache. */
  function invalidate() {
    _cache.delete(getProjectId());
  }

  return {
    environments,
    loading,
    load,
    invalidate,
  };
}
