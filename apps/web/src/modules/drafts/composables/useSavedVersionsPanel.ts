import { computed, ref, type Ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import * as savedVersionsApi from "@/api/saved-versions";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type { DraftResponse } from "@/api/types/draft";
import type {
  SavedVersionDetail,
  SavedVersionSummary,
} from "@/api/types/saved-version";

export const SAVED_VERSION_NOTE_MAX_LENGTH = 500;

interface UseSavedVersionsPanelOptions {
  canViewSavedVersions: Ref<boolean>;
  deploymentId: Ref<number>;
  configFileId: Ref<number>;
  draftVersion: Ref<number | null>;
  isDirty: Ref<boolean>;
  applyDraft: (value: DraftResponse) => void;
  refreshPreviewStatus: (deploymentId: number) => void;
  t: (
    key: string,
    params?: Record<string, string | number | null | undefined>,
  ) => string;
}

export function useSavedVersionsPanel(options: UseSavedVersionsPanelOptions) {
  const savedVersions = ref<SavedVersionSummary[]>([]);
  const savedVersionsLoading = ref(false);
  const savedVersionsError = ref<ApiRequestError | null>(null);
  const selectedSavedVersionId = ref<number | null>(null);
  const savedVersionDetail = ref<SavedVersionDetail | null>(null);
  const savedVersionDetailLoading = ref(false);
  const savedVersionNote = ref("");
  const savedNoteSnapshot = ref("");
  const updatingSavedVersionNote = ref(false);
  const restoringFromSavedVersion = ref(false);
  const deletingSavedVersion = ref(false);

  const isNoteDirty = computed(
    () =>
      savedVersionDetail.value !== null &&
      savedVersionNote.value !== savedNoteSnapshot.value,
  );

  function resetSavedVersions() {
    savedVersions.value = [];
    savedVersionsError.value = null;
    selectedSavedVersionId.value = null;
    savedVersionDetail.value = null;
    savedVersionNote.value = "";
    savedNoteSnapshot.value = "";
  }

  async function loadSavedVersions(optionsOverride?: {
    keepSelection?: boolean;
  }) {
    if (!options.canViewSavedVersions.value) {
      return;
    }

    savedVersionsLoading.value = true;
    savedVersionsError.value = null;
    try {
      const result = await savedVersionsApi.listSavedVersions({
        deployment_instance_id: options.deploymentId.value,
        config_file_id: options.configFileId.value,
      });
      savedVersions.value = result.items;

      if (result.items.length === 0) {
        selectedSavedVersionId.value = null;
        savedVersionDetail.value = null;
        savedVersionNote.value = "";
        return;
      }

      const keepSelection = optionsOverride?.keepSelection ?? true;
      const keepCurrent =
        keepSelection &&
        selectedSavedVersionId.value !== null &&
        result.items.some((item) => item.id === selectedSavedVersionId.value);

      const nextSelectedId = keepCurrent
        ? selectedSavedVersionId.value
        : result.items[0].id;
      if (nextSelectedId !== null) {
        await selectSavedVersion(nextSelectedId);
      }
    } catch (err) {
      if (err instanceof ApiRequestError) {
        savedVersionsError.value = err;
      } else {
        savedVersionsError.value = new ApiRequestError(0, {
          code: "unknown_error",
          message: options.t("savedVersions.error.loadList"),
        });
      }
    } finally {
      savedVersionsLoading.value = false;
    }
  }

  async function confirmIfNoteDirty(): Promise<boolean> {
    if (!isNoteDirty.value) return true;
    try {
      await ElMessageBox.confirm(
        options.t("savedVersions.note.discardPrompt"),
        options.t("savedVersions.note.discardTitle"),
        {
          confirmButtonText: options.t("savedVersions.note.discardConfirm"),
          cancelButtonText: options.t("common.cancel"),
          type: "warning",
        },
      );
      return true;
    } catch {
      return false;
    }
  }

  async function selectSavedVersion(id: number) {
    if (!(await confirmIfNoteDirty())) return;
    if (
      selectedSavedVersionId.value === id &&
      savedVersionDetail.value?.id === id
    )
      return;
    selectedSavedVersionId.value = id;
    savedVersionDetailLoading.value = true;
    try {
      const result = await savedVersionsApi.getSavedVersion(id);
      if (selectedSavedVersionId.value !== id) return;
      savedVersionDetail.value = result.saved_version;
      savedVersionNote.value = result.saved_version.note ?? "";
      savedNoteSnapshot.value = savedVersionNote.value;
    } catch (err) {
      if (selectedSavedVersionId.value !== id) return;
      if (err instanceof ApiRequestError) {
        ElMessage.error(getErrorMessage(err.code, err.message));
      } else {
        ElMessage.error(options.t("toast.operationFailed"));
      }
    } finally {
      if (selectedSavedVersionId.value === id) {
        savedVersionDetailLoading.value = false;
      }
    }
  }

  async function handleUpdateSavedVersionNote() {
    if (!savedVersionDetail.value) {
      return;
    }

    const note = savedVersionNote.value.trim();
    if (note.length > SAVED_VERSION_NOTE_MAX_LENGTH) {
      ElMessage.error(options.t("savedVersions.error.noteTooLong"));
      return;
    }

    updatingSavedVersionNote.value = true;
    try {
      const result = await savedVersionsApi.updateSavedVersion(
        savedVersionDetail.value.id,
        {
          note: note.length > 0 ? note : null,
        },
      );
      savedVersionDetail.value = result.saved_version;
      savedVersionNote.value = result.saved_version.note ?? "";
      savedNoteSnapshot.value = savedVersionNote.value;
      savedVersions.value = savedVersions.value.map((item) => {
        if (item.id !== result.saved_version.id) {
          return item;
        }
        return {
          ...item,
          note: result.saved_version.note,
        };
      });
      ElMessage.success(options.t("toast.savedVersions.noteSaved"));
    } catch (err) {
      if (err instanceof ApiRequestError) {
        ElMessage.error(getErrorMessage(err.code, err.message));
      } else {
        ElMessage.error(options.t("toast.operationFailed"));
      }
    } finally {
      updatingSavedVersionNote.value = false;
    }
  }

  async function handleRestoreSavedVersion() {
    if (!savedVersionDetail.value) {
      return;
    }

    if (options.isDirty.value) {
      try {
        await ElMessageBox.confirm(
          options.t("drafts.navigate.prompt"),
          options.t("drafts.navigate.title"),
          {
            confirmButtonText: options.t("drafts.navigate.confirm"),
            cancelButtonText: options.t("common.cancel"),
            type: "warning",
          },
        );
      } catch {
        return;
      }
    }

    try {
      await ElMessageBox.confirm(
        options.t("savedVersions.restore.prompt"),
        options.t("savedVersions.restore.title"),
        {
          confirmButtonText: options.t("savedVersions.restore.confirm"),
          cancelButtonText: options.t("common.cancel"),
          type: "warning",
        },
      );
    } catch {
      return;
    }

    restoringFromSavedVersion.value = true;
    try {
      const result = await savedVersionsApi.restoreSavedVersion(
        savedVersionDetail.value.id,
        {
          base_version: options.draftVersion.value,
        },
      );
      options.applyDraft(result.draft);
      ElMessage.success(options.t("toast.savedVersions.restored"));
      options.refreshPreviewStatus(options.deploymentId.value);
      await loadSavedVersions();
    } catch (err) {
      if (err instanceof ApiRequestError) {
        ElMessage.error(getErrorMessage(err.code, err.message));
      } else {
        ElMessage.error(options.t("toast.operationFailed"));
      }
    } finally {
      restoringFromSavedVersion.value = false;
    }
  }

  async function handleDeleteSavedVersion() {
    if (!savedVersionDetail.value) {
      return;
    }

    try {
      await ElMessageBox.confirm(
        options.t("savedVersions.delete.prompt"),
        options.t("savedVersions.delete.title"),
        {
          confirmButtonText: options.t("savedVersions.delete.confirm"),
          cancelButtonText: options.t("common.cancel"),
          type: "warning",
        },
      );
    } catch {
      return;
    }

    deletingSavedVersion.value = true;
    try {
      await savedVersionsApi.deleteSavedVersion(savedVersionDetail.value.id);
      ElMessage.success(options.t("toast.savedVersions.deleted"));
      await loadSavedVersions({ keepSelection: false });
    } catch (err) {
      if (err instanceof ApiRequestError) {
        ElMessage.error(getErrorMessage(err.code, err.message));
      } else {
        ElMessage.error(options.t("toast.operationFailed"));
      }
    } finally {
      deletingSavedVersion.value = false;
    }
  }

  return {
    savedVersions,
    savedVersionsLoading,
    savedVersionsError,
    selectedSavedVersionId,
    savedVersionDetail,
    savedVersionDetailLoading,
    savedVersionNote,
    updatingSavedVersionNote,
    restoringFromSavedVersion,
    deletingSavedVersion,
    resetSavedVersions,
    loadSavedVersions,
    selectSavedVersion,
    handleUpdateSavedVersionNote,
    handleRestoreSavedVersion,
    handleDeleteSavedVersion,
  };
}
