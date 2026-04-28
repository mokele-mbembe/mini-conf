import { computed, nextTick, ref } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ApiRequestError } from "@/api/error";
import type { CloneSourceSummary } from "@/api/types/clone-source";
import type { DraftResponse } from "@/api/types/draft";

const mockState = vi.hoisted(() => ({
  listCloneSources: vi.fn(),
  cloneDraft: vi.fn(),
  confirm: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
}));

vi.mock("@/api/clone-sources", () => ({
  listCloneSources: mockState.listCloneSources,
}));

vi.mock("@/api/drafts", () => ({
  cloneDraft: mockState.cloneDraft,
}));

vi.mock("element-plus", () => ({
  ElMessage: {
    success: mockState.success,
    error: mockState.error,
  },
  ElMessageBox: {
    confirm: mockState.confirm,
  },
}));

import { useDraftCloneDialog } from "./useDraftCloneDialog";

const cloneSources: CloneSourceSummary[] = [
  {
    deployment_instance_id: 11,
    deployment_key: "store-001",
    name: "Store 001",
    environment_id: 1,
    environment_name: "Production",
    is_template: false,
    available_sources: {
      draft: true,
      latest_release: true,
    },
  },
  {
    deployment_instance_id: 12,
    deployment_key: "template-default",
    name: "Template Default",
    environment_id: 1,
    environment_name: "Production",
    is_template: true,
    available_sources: {
      draft: true,
      latest_release: false,
    },
  },
];

const clonedDraft: DraftResponse = {
  deployment_instance_id: 2,
  config_file_id: 3,
  format: "yaml",
  content: "greeting: cloned",
  version: 8,
  updated_at: "2026-04-28T10:00:00Z",
};

function flushPromises() {
  return Promise.resolve().then(() => Promise.resolve());
}

function createDialog(options?: { dirty?: boolean }) {
  const projectId = ref(1);
  const deploymentId = ref(2);
  const configFileId = ref(3);
  const isDirty = ref(options?.dirty ?? false);
  const applyDraft = vi.fn();
  const refreshPreviewStatus = vi.fn();

  const dialog = useDraftCloneDialog({
    projectId: computed(() => projectId.value),
    deploymentId: computed(() => deploymentId.value),
    configFileId: computed(() => configFileId.value),
    isDirty: computed(() => isDirty.value),
    applyDraft,
    refreshPreviewStatus,
    t: (key) => key,
  });

  return {
    dialog,
    applyDraft,
    refreshPreviewStatus,
    projectId,
    deploymentId,
    configFileId,
    isDirty,
  };
}

describe("useDraftCloneDialog", () => {
  beforeEach(() => {
    vi.useRealTimers();
    mockState.listCloneSources.mockReset();
    mockState.cloneDraft.mockReset();
    mockState.confirm.mockReset();
    mockState.success.mockReset();
    mockState.error.mockReset();
  });

  it("resets and loads clone sources when the dialog opens", async () => {
    mockState.listCloneSources.mockResolvedValue({
      items: cloneSources,
      next_cursor: 30,
    });
    const { dialog } = createDialog();

    dialog.cloneSourceInstanceId.value = 99;
    dialog.cloneSourceKind.value = "latest_release";
    dialog.cloneDialogVisible.value = true;
    await nextTick();
    await flushPromises();

    expect(mockState.listCloneSources).toHaveBeenCalledWith({
      project_id: 1,
      target_deployment_id: 2,
      config_file_id: 3,
      keyword: undefined,
      limit: 50,
    });
    expect(dialog.cloneSourceInstanceId.value).toBeNull();
    expect(dialog.cloneSourceKind.value).toBe("draft");
    expect(dialog.cloneSources.value).toEqual(cloneSources);
    expect(dialog.cloneNextCursor.value).toBe(30);
    expect(dialog.cloneInstancesLoading.value).toBe(false);
  });

  it("appends load-more results using the current cursor and keyword", async () => {
    mockState.listCloneSources
      .mockResolvedValueOnce({
        items: [cloneSources[0]],
        next_cursor: 30,
      })
      .mockResolvedValueOnce({
        items: [cloneSources[1]],
        next_cursor: null,
      });
    const { dialog } = createDialog();

    await dialog.searchCloneSources("store");
    await dialog.loadMoreCloneSources();

    expect(mockState.listCloneSources).toHaveBeenLastCalledWith({
      project_id: 1,
      target_deployment_id: 2,
      config_file_id: 3,
      keyword: "store",
      limit: 50,
      cursor: 30,
    });
    expect(dialog.cloneSources.value).toEqual(cloneSources);
    expect(dialog.cloneNextCursor.value).toBeNull();
  });

  it("debounces remote search", async () => {
    vi.useFakeTimers();
    mockState.listCloneSources.mockResolvedValue({
      items: [],
      next_cursor: null,
    });
    const { dialog } = createDialog();

    dialog.handleCloneRemoteSearch("s");
    dialog.handleCloneRemoteSearch("store");
    await vi.advanceTimersByTimeAsync(299);
    expect(mockState.listCloneSources).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(1);
    await flushPromises();

    expect(mockState.listCloneSources).toHaveBeenCalledTimes(1);
    expect(mockState.listCloneSources).toHaveBeenCalledWith(
      expect.objectContaining({ keyword: "store" }),
    );
  });

  it("clones the selected source and applies the returned draft", async () => {
    mockState.cloneDraft.mockResolvedValue({
      draft: clonedDraft,
      source_deployment_instance_id: 11,
      source_kind: "latest_release",
    });
    const { dialog, applyDraft, refreshPreviewStatus } = createDialog();

    dialog.cloneDialogVisible.value = true;
    dialog.cloneSources.value = cloneSources;
    dialog.cloneSourceInstanceId.value = 11;
    dialog.cloneSourceKind.value = "latest_release";
    await dialog.handleCloneFromInstance();

    expect(mockState.cloneDraft).toHaveBeenCalledWith(2, 3, {
      source_deployment_instance_id: 11,
      source_kind: "latest_release",
    });
    expect(applyDraft).toHaveBeenCalledWith(clonedDraft);
    expect(dialog.cloneDialogVisible.value).toBe(false);
    expect(mockState.success).toHaveBeenCalledWith(
      "toast.drafts.clonedFromInstance",
    );
    expect(refreshPreviewStatus).toHaveBeenCalledWith(2);
  });

  it("asks before overwriting dirty content and surfaces clone API errors", async () => {
    mockState.confirm.mockResolvedValue(undefined);
    mockState.cloneDraft.mockRejectedValue(
      new ApiRequestError(404, {
        code: "release_not_found",
        message: "Release not found",
      }),
    );
    const { dialog } = createDialog({ dirty: true });

    dialog.cloneSources.value = cloneSources;
    dialog.cloneSourceInstanceId.value = 11;
    dialog.cloneSourceKind.value = "latest_release";
    await dialog.handleCloneFromInstance();

    expect(mockState.confirm).toHaveBeenCalledWith(
      "drafts.cloneDialog.overwritePrompt",
      "drafts.cloneDialog.overwriteTitle",
      expect.objectContaining({
        confirmButtonText: "drafts.cloneDialog.overwriteConfirm",
        cancelButtonText: "common.cancel",
      }),
    );
    expect(mockState.error).toHaveBeenCalledWith(
      "drafts.cloneDialog.error.sourceReleaseNotFound",
    );
  });
});
