import { ref, readonly } from "vue";
import type { ProjectSummary } from "@/api/types/project";
import * as projectsApi from "@/api/projects";
import { ApiRequestError } from "@/api/error";
import { t } from "@/shared/i18n";

// Module-scoped state: shared across all callers within the same SPA session.
// This avoids re-fetching (and flashing null) when navigating between pages
// that belong to the same project.
const _project = ref<ProjectSummary | null>(null);
const _loading = ref(false);
const _error = ref<ApiRequestError | null>(null);
const _cache = new Map<number, ProjectSummary>();
let _requestSeq = 0;

export function useProjectContext() {
  async function fetchProject(id: number) {
    const seq = ++_requestSeq;

    // Stale-while-revalidate: if we already have data for this project,
    // show it immediately and refresh in the background.
    const cached = _cache.get(id);
    if (cached) {
      _project.value = cached;
      _error.value = null;
      // Background refresh — no loading flash
      projectsApi
        .getProject(id)
        .then((fresh) => {
          _cache.set(id, fresh);
          // Only update if the user hasn't navigated to a different project
          if (seq === _requestSeq) {
            _project.value = fresh;
          }
        })
        .catch(() => {
          // Background refresh failed; stale data remains visible — acceptable.
        });
      return;
    }

    // No cache hit — full blocking fetch.
    _loading.value = true;
    _error.value = null;
    _project.value = null;
    try {
      const data = await projectsApi.getProject(id);
      _cache.set(id, data);
      if (seq === _requestSeq) {
        _project.value = data;
      }
    } catch (err) {
      if (seq !== _requestSeq) return;
      if (err instanceof ApiRequestError) {
        _error.value = err;
      } else {
        _error.value = new ApiRequestError(0, {
          code: "unknown_error",
          message: t("project.loadFailed"),
        });
      }
    } finally {
      if (seq === _requestSeq) {
        _loading.value = false;
      }
    }
  }

  return {
    project: readonly(_project),
    loading: readonly(_loading),
    error: readonly(_error),
    fetchProject,
  };
}
