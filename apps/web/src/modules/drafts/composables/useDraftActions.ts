import { ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import * as deploymentInstancesApi from "@/api/deployment-instances";
import * as draftsApi from "@/api/drafts";
import * as releasesApi from "@/api/releases";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type { ConfigFileSummary } from "@/api/types/config-file";
import type { DraftResponse } from "@/api/types/draft";
import type { ReleaseSummary } from "@/api/types/release";

type ReadableRef<T> = {
  readonly value: T;
};

interface UseDraftActionsOptions {
  projectId: ReadableRef<number>;
  deploymentId: ReadableRef<number>;
  configFileId: ReadableRef<number>;
  configFile: ReadableRef<ConfigFileSummary | null>;
  draft: ReadableRef<DraftResponse | null>;
  content: ReadableRef<string>;
  isDirty: ReadableRef<boolean>;
  applyDraft: (value: DraftResponse) => void;
  markDraftMissing: (fallbackContent: string) => void;
  loadSavedVersions: (options?: { keepSelection?: boolean }) => Promise<void>;
  refreshPreviewStatus: (deploymentId: number) => void | Promise<void>;
  onReleasePublished: (release: ReleaseSummary) => void | Promise<void>;
  t: (
    key: string,
    params?: Record<string, string | number | null | undefined>,
  ) => string;
}

export function useDraftActions(options: UseDraftActionsOptions) {
  const saving = ref(false);
  const publishing = ref(false);
  const discarding = ref(false);
  const restoring = ref(false);

  async function handleSave() {
    if (!options.configFile.value) return;

    saving.value = true;
    try {
      const result = await draftsApi.updateDraft(
        options.deploymentId.value,
        options.configFileId.value,
        {
          content: options.content.value,
          format: options.configFile.value.format,
          base_version: options.draft.value?.version ?? 0,
        },
      );
      options.applyDraft(result);
      await options.loadSavedVersions({ keepSelection: false });
      ElMessage.success(options.t("toast.drafts.saved"));
    } catch (err) {
      showOperationError(err);
    } finally {
      saving.value = false;
    }
  }

  async function handleDiscard() {
    try {
      await ElMessageBox.confirm(
        options.t("drafts.discard.prompt"),
        options.t("drafts.discard.title"),
        {
          confirmButtonText: options.t("drafts.discard.confirm"),
          cancelButtonText: options.t("common.cancel"),
          type: "warning",
        },
      );
    } catch {
      return;
    }

    discarding.value = true;
    try {
      await draftsApi.deleteDraft(
        options.deploymentId.value,
        options.configFileId.value,
      );
      const fallbackContent = await loadFallbackPreviewContent();
      options.markDraftMissing(fallbackContent);

      ElMessage.success(options.t("toast.drafts.discarded"));
      void options.refreshPreviewStatus(options.deploymentId.value);
    } catch (err) {
      showOperationError(err);
    } finally {
      discarding.value = false;
    }
  }

  async function handleRestoreFromRelease() {
    try {
      await ElMessageBox.confirm(
        options.t("drafts.restore.prompt"),
        options.t("drafts.restore.title"),
        {
          confirmButtonText: options.t("drafts.restore.confirm"),
          cancelButtonText: options.t("common.cancel"),
          type: "warning",
        },
      );
    } catch {
      return;
    }

    restoring.value = true;
    try {
      const result = await draftsApi.cloneDraft(
        options.deploymentId.value,
        options.configFileId.value,
        {
          source_deployment_instance_id: options.deploymentId.value,
          source_kind: "latest_release",
        },
      );
      options.applyDraft(result.draft);
      ElMessage.success(options.t("toast.drafts.restoredFromRelease"));
      void options.refreshPreviewStatus(options.deploymentId.value);
    } catch (err) {
      showOperationError(err);
    } finally {
      restoring.value = false;
    }
  }

  async function handlePublish() {
    if (!options.draft.value || options.isDirty.value) {
      ElMessage.warning(options.t("drafts.notice.saveBeforePublish"));
      return;
    }

    try {
      const { value } = await ElMessageBox.prompt(
        options.t("drafts.publish.prompt"),
        options.t("drafts.publish.title"),
        {
          inputType: "textarea",
          inputPlaceholder: options.t("drafts.publish.placeholder"),
          confirmButtonText: options.t("common.publish"),
          cancelButtonText: options.t("common.cancel"),
        },
      );
      publishing.value = true;
      const release = await releasesApi.publishRelease({
        project_id: options.projectId.value,
        deployment_instance_id: options.deploymentId.value,
        config_file_id: options.configFileId.value,
        change_summary: value || null,
      });
      ElMessage.success(
        options.t("toast.releases.published", { revision: release.revision }),
      );
      void options.onReleasePublished(release);
    } catch (err) {
      if (err === "cancel" || err === "close") return;
      showOperationError(err);
    } finally {
      publishing.value = false;
    }
  }

  async function loadFallbackPreviewContent() {
    try {
      const preview = await deploymentInstancesApi.previewDeploymentBundle(
        options.deploymentId.value,
      );
      const item = preview.items.find(
        (i) => i.config_file_id === options.configFileId.value,
      );
      return item?.content ?? "";
    } catch {
      return "";
    }
  }

  function showOperationError(err: unknown) {
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(options.t("toast.operationFailed"));
    }
  }

  return {
    saving,
    publishing,
    discarding,
    restoring,
    handleSave,
    handleDiscard,
    handleRestoreFromRelease,
    handlePublish,
  };
}
