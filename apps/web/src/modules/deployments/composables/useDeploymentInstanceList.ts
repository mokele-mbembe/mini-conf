import { ref, watch, onUnmounted } from "vue";
import * as deploymentInstancesApi from "@/api/deployment-instances";
import { ApiRequestError } from "@/api/error";
import type {
  DeploymentInstanceStatus,
  DeploymentInstanceSummary,
} from "@/api/types/deployment-instance";

export interface UseDeploymentInstanceListOptions {
  getProjectId: () => number;
  isTemplate: boolean;
  withStatusFilter: boolean;
  visibilityFilter?: "current" | "archived" | "all";
}

export function useDeploymentInstanceList(
  options: UseDeploymentInstanceListOptions,
) {
  const items = ref<DeploymentInstanceSummary[]>([]);
  const loading = ref(false);
  const error = ref<ApiRequestError | null>(null);
  const keyword = ref("");
  const environmentId = ref("");
  const status = ref<DeploymentInstanceStatus | "">("");
  const page = ref(1);
  const pageSize = ref(20);
  const total = ref(0);

  let requestSeq = 0;

  async function load() {
    const seq = ++requestSeq;
    loading.value = true;
    error.value = null;
    try {
      const res = await deploymentInstancesApi.listDeploymentInstances({
        project_id: options.getProjectId(),
        is_template: options.isTemplate,
        keyword: keyword.value.trim() || undefined,
        environment_id: environmentId.value
          ? Number(environmentId.value)
          : undefined,
        status:
          options.withStatusFilter && status.value ? status.value : undefined,
        visibility_filter: options.visibilityFilter,
        page: page.value,
        page_size: pageSize.value,
      });
      if (seq !== requestSeq) return; // stale response
      items.value = res.items;
      total.value = res.total;
      page.value = res.page;
      pageSize.value = res.page_size;
    } catch (err) {
      if (seq !== requestSeq) return;
      if (err instanceof ApiRequestError) {
        error.value = err;
      } else {
        error.value = new ApiRequestError(0, {
          code: "unknown_error",
          message: "Failed to load deployment instances",
        });
      }
    } finally {
      if (seq === requestSeq) {
        loading.value = false;
      }
    }
  }

  function search() {
    page.value = 1;
    load();
  }

  function filterChange() {
    page.value = 1;
    load();
  }

  function resetFilters() {
    keyword.value = "";
    environmentId.value = "";
    status.value = "";
    page.value = 1;
    load();
  }

  function pageSizeChange() {
    page.value = 1;
    load();
  }

  // Keyword debounce: auto-search 300ms after the user stops typing.
  let keywordDebounceTimer: ReturnType<typeof globalThis.setTimeout> | null =
    null;

  watch(keyword, () => {
    if (keywordDebounceTimer) globalThis.clearTimeout(keywordDebounceTimer);
    keywordDebounceTimer = globalThis.setTimeout(() => {
      page.value = 1;
      load();
    }, 300);
  });

  onUnmounted(() => {
    if (keywordDebounceTimer) globalThis.clearTimeout(keywordDebounceTimer);
  });

  return {
    items,
    loading,
    error,
    keyword,
    environmentId,
    status,
    page,
    pageSize,
    total,
    load,
    search,
    filterChange,
    resetFilters,
    pageSizeChange,
  };
}
