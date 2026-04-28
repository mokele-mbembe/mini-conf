import {
  computed,
  onBeforeUnmount,
  ref,
  watch,
  type ComputedRef,
  type Ref,
} from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import * as cloneSourcesApi from "@/api/clone-sources";
import * as draftsApi from "@/api/drafts";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type { CloneSourceSummary } from "@/api/types/clone-source";
import type { DraftResponse } from "@/api/types/draft";

export type DraftCloneSourceKind = "draft" | "latest_release";

interface UseDraftCloneDialogOptions {
  projectId: ComputedRef<number>;
  deploymentId: ComputedRef<number>;
  configFileId: ComputedRef<number>;
  isDirty: ComputedRef<boolean>;
  applyDraft: (value: DraftResponse) => void;
  refreshPreviewStatus: (deploymentId: number) => void;
  t: (
    key: string,
    params?: Record<string, string | number | null | undefined>,
  ) => string;
}

const CLONE_ERROR_KEYS: Record<string, string> = {
  draft_not_found: "drafts.cloneDialog.error.sourceDraftNotFound",
  release_not_found: "drafts.cloneDialog.error.sourceReleaseNotFound",
  draft_validation_failed: "drafts.cloneDialog.error.validationFailed",
};

export function useDraftCloneDialog(options: UseDraftCloneDialogOptions) {
  const cloneDialogVisible = ref(false);
  const cloneSourceInstanceId = ref<number | null>(null);
  const cloneSourceKind = ref<DraftCloneSourceKind>("draft");
  const cloneSources = ref<CloneSourceSummary[]>([]);
  const cloneInstancesLoading = ref(false);
  const cloneLoadingMore = ref(false);
  const cloneLoadError = ref(false);
  const cloning = ref(false);
  const cloneNextCursor = ref<number | null>(null);
  const cloneSearchKeyword = ref<string | undefined>(undefined);
  let cloneSearchTimer: ReturnType<typeof globalThis.setTimeout> | null = null;
  let cloneSearchSeq = 0;

  const selectedCloneSource = computed(() =>
    cloneSources.value.find(
      (s) => s.deployment_instance_id === cloneSourceInstanceId.value,
    ),
  );
  const cloneDraftOptionDisabled = computed(
    () =>
      selectedCloneSource.value !== undefined &&
      !selectedCloneSource.value.available_sources.draft,
  );
  const cloneReleaseOptionDisabled = computed(
    () =>
      selectedCloneSource.value !== undefined &&
      (!selectedCloneSource.value.available_sources.latest_release ||
        selectedCloneSource.value.is_template),
  );
  const selectedCloneSourceDraftUnavailable = computed(
    () =>
      selectedCloneSource.value !== undefined &&
      !selectedCloneSource.value.available_sources.draft,
  );
  const selectedCloneSourceReleaseUnavailable = computed(
    () =>
      selectedCloneSource.value !== undefined &&
      !selectedCloneSource.value.available_sources.latest_release,
  );

  const cloneSubmitDisabled = computed(() => {
    if (!cloneSourceInstanceId.value || !selectedCloneSource.value) return true;
    const src = selectedCloneSource.value;
    if (cloneSourceKind.value === "draft" && !src.available_sources.draft) {
      return true;
    }
    if (
      cloneSourceKind.value === "latest_release" &&
      !src.available_sources.latest_release
    ) {
      return true;
    }
    return false;
  });

  function resetCloneDialog() {
    cloneSourceInstanceId.value = null;
    cloneSourceKind.value = "draft";
    cloneLoadError.value = false;
    cloneSources.value = [];
    cloneNextCursor.value = null;
    cloneSearchKeyword.value = undefined;
  }

  async function searchCloneSources(keyword?: string) {
    const seq = ++cloneSearchSeq;
    const normalizedKeyword = keyword || undefined;
    cloneSearchKeyword.value = normalizedKeyword;
    cloneInstancesLoading.value = true;
    cloneLoadError.value = false;
    cloneNextCursor.value = null;
    try {
      const result = await cloneSourcesApi.listCloneSources({
        project_id: options.projectId.value,
        target_deployment_id: options.deploymentId.value,
        config_file_id: options.configFileId.value,
        keyword: normalizedKeyword,
        limit: 50,
      });
      if (seq !== cloneSearchSeq) return;
      cloneSources.value = result.items;
      cloneNextCursor.value = result.next_cursor;
    } catch {
      if (seq !== cloneSearchSeq) return;
      cloneSources.value = [];
      cloneLoadError.value = true;
    } finally {
      if (seq === cloneSearchSeq) {
        cloneInstancesLoading.value = false;
      }
    }
  }

  async function loadMoreCloneSources() {
    if (!cloneNextCursor.value || cloneLoadingMore.value) return;
    const seq = cloneSearchSeq;
    cloneLoadingMore.value = true;
    try {
      const result = await cloneSourcesApi.listCloneSources({
        project_id: options.projectId.value,
        target_deployment_id: options.deploymentId.value,
        config_file_id: options.configFileId.value,
        keyword: cloneSearchKeyword.value,
        limit: 50,
        cursor: cloneNextCursor.value,
      });
      if (seq !== cloneSearchSeq) return;
      cloneSources.value = [...cloneSources.value, ...result.items];
      cloneNextCursor.value = result.next_cursor;
    } catch {
      // The load-more action is retryable from the dialog footer.
    } finally {
      if (seq === cloneSearchSeq) {
        cloneLoadingMore.value = false;
      }
    }
  }

  function handleCloneRemoteSearch(keyword: string) {
    if (cloneSearchTimer) globalThis.clearTimeout(cloneSearchTimer);
    if (cloneLoadingMore.value) return;
    cloneSearchTimer = globalThis.setTimeout(() => {
      searchCloneSources(keyword);
    }, 300);
  }

  async function handleCloneFromInstance() {
    if (cloneSubmitDisabled.value || !cloneSourceInstanceId.value) return;

    if (options.isDirty.value) {
      try {
        await ElMessageBox.confirm(
          options.t("drafts.cloneDialog.overwritePrompt"),
          options.t("drafts.cloneDialog.overwriteTitle"),
          {
            confirmButtonText: options.t("drafts.cloneDialog.overwriteConfirm"),
            cancelButtonText: options.t("common.cancel"),
            type: "warning",
          },
        );
      } catch {
        return;
      }
    }

    cloning.value = true;
    try {
      const result = await draftsApi.cloneDraft(
        options.deploymentId.value,
        options.configFileId.value,
        {
          source_deployment_instance_id: cloneSourceInstanceId.value,
          source_kind: cloneSourceKind.value,
        },
      );
      options.applyDraft(result.draft);
      cloneDialogVisible.value = false;
      ElMessage.success(options.t("toast.drafts.clonedFromInstance"));
      options.refreshPreviewStatus(options.deploymentId.value);
    } catch (err) {
      if (err instanceof ApiRequestError) {
        const cloneErrorKey = CLONE_ERROR_KEYS[err.code];
        ElMessage.error(
          cloneErrorKey
            ? options.t(cloneErrorKey)
            : getErrorMessage(err.code, err.message),
        );
      } else {
        ElMessage.error(options.t("toast.operationFailed"));
      }
    } finally {
      cloning.value = false;
    }
  }

  watch(cloneDialogVisible, (visible) => {
    if (visible) {
      resetCloneDialog();
      searchCloneSources();
    }
  });

  watch(selectedCloneSource, (src) => {
    if (!src) return;
    if (src.is_template || !src.available_sources.latest_release) {
      cloneSourceKind.value = "draft";
    }
  });

  onBeforeUnmount(() => {
    if (cloneSearchTimer) {
      globalThis.clearTimeout(cloneSearchTimer);
    }
  });

  return {
    cloneDialogVisible,
    cloneSourceInstanceId,
    cloneSourceKind: cloneSourceKind as Ref<DraftCloneSourceKind>,
    cloneSources,
    cloneInstancesLoading,
    cloneLoadingMore,
    cloneLoadError,
    cloning,
    cloneNextCursor,
    selectedCloneSource,
    cloneDraftOptionDisabled,
    cloneReleaseOptionDisabled,
    selectedCloneSourceDraftUnavailable,
    selectedCloneSourceReleaseUnavailable,
    cloneSubmitDisabled,
    searchCloneSources,
    loadMoreCloneSources,
    handleCloneRemoteSearch,
    handleCloneFromInstance,
  };
}
