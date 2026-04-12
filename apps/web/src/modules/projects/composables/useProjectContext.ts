import { ref, readonly } from "vue";
import type { ProjectSummary } from "@/api/types/project";
import * as projectsApi from "@/api/projects";
import { ApiRequestError } from "@/api/error";

export function useProjectContext() {
  const project = ref<ProjectSummary | null>(null);
  const loading = ref(false);
  const error = ref<ApiRequestError | null>(null);

  async function fetchProject(id: number) {
    loading.value = true;
    error.value = null;
    project.value = null;
    try {
      project.value = await projectsApi.getProject(id);
    } catch (err) {
      if (err instanceof ApiRequestError) {
        error.value = err;
      } else {
        error.value = new ApiRequestError(0, {
          code: "unknown_error",
          message: "Failed to fetch project",
        });
      }
    } finally {
      loading.value = false;
    }
  }

  return {
    project: readonly(project),
    loading: readonly(loading),
    error: readonly(error),
    fetchProject,
  };
}
