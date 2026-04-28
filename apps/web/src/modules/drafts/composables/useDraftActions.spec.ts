import { computed, ref } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ApiRequestError } from "@/api/error";
import type { ConfigFileSummary } from "@/api/types/config-file";
import type { DeploymentBundlePreviewResponse } from "@/api/types/deployment-instance";
import type { DraftResponse } from "@/api/types/draft";
import type { ReleaseSummary } from "@/api/types/release";

const mockState = vi.hoisted(() => ({
  updateDraft: vi.fn(),
  deleteDraft: vi.fn(),
  cloneDraft: vi.fn(),
  previewDeploymentBundle: vi.fn(),
  publishRelease: vi.fn(),
  confirm: vi.fn(),
  prompt: vi.fn(),
  success: vi.fn(),
  error: vi.fn(),
  warning: vi.fn(),
}));

vi.mock("@/api/drafts", () => ({
  updateDraft: mockState.updateDraft,
  deleteDraft: mockState.deleteDraft,
  cloneDraft: mockState.cloneDraft,
}));

vi.mock("@/api/deployment-instances", () => ({
  previewDeploymentBundle: mockState.previewDeploymentBundle,
}));

vi.mock("@/api/releases", () => ({
  publishRelease: mockState.publishRelease,
}));

vi.mock("@/shared/constants/error-messages", () => ({
  getErrorMessage: (code: string, detail?: string) =>
    `api-error:${code}:${detail}`,
}));

vi.mock("element-plus", () => ({
  ElMessage: {
    success: mockState.success,
    error: mockState.error,
    warning: mockState.warning,
  },
  ElMessageBox: {
    confirm: mockState.confirm,
    prompt: mockState.prompt,
  },
}));

import { useDraftActions } from "./useDraftActions";

const configFile: ConfigFileSummary = {
  id: 3,
  project_id: 1,
  code: "app",
  name: "App Config",
  format: "yaml",
  sensitivity: "normal",
  is_required: true,
  status: "active",
  description: null,
  secret_paths: null,
};

const draft: DraftResponse = {
  deployment_instance_id: 2,
  config_file_id: 3,
  format: "yaml",
  content: "greeting: saved",
  version: 7,
  updated_at: "2026-04-28T10:00:00Z",
};

const release: ReleaseSummary = {
  id: 99,
  project_id: 1,
  deployment_instance_id: 2,
  config_file_id: 3,
  revision: "r9",
  content_hash: "sha256:abc",
  format: "yaml",
  change_summary: "ship it",
  apply_mode: "immediate",
  published_by: 4,
  published_at: "2026-04-28T10:00:00Z",
};

const preview: DeploymentBundlePreviewResponse = {
  deployment_instance_id: 2,
  items: [
    {
      config_file_id: 3,
      code: "app",
      name: "App Config",
      is_required: true,
      source: "latest_release",
      status: "ok",
      format: "yaml",
      content: "greeting: release",
      revision: "r8",
    },
  ],
  open_bundle_preview: {
    project: "demo",
    environment: "prod",
    deployment: {
      key: "store-001",
      name: "Store 001",
    },
    configs: [],
  },
};

function createActions(options?: {
  dirty?: boolean;
  draft?: DraftResponse | null;
}) {
  const projectId = ref(1);
  const deploymentId = ref(2);
  const configFileId = ref(3);
  const currentConfigFile = ref<ConfigFileSummary | null>(configFile);
  const currentDraft = ref<DraftResponse | null>(options?.draft ?? draft);
  const content = ref("greeting: edited");
  const isDirty = ref(options?.dirty ?? false);
  const applyDraft = vi.fn((value: DraftResponse) => {
    currentDraft.value = value;
    content.value = value.content;
  });
  const markDraftMissing = vi.fn((fallbackContent: string) => {
    currentDraft.value = null;
    content.value = fallbackContent;
  });
  const loadSavedVersions = vi.fn().mockResolvedValue(undefined);
  const refreshPreviewStatus = vi.fn();
  const onReleasePublished = vi.fn();

  const actions = useDraftActions({
    projectId: computed(() => projectId.value),
    deploymentId: computed(() => deploymentId.value),
    configFileId: computed(() => configFileId.value),
    configFile: computed(() => currentConfigFile.value),
    draft: computed(() => currentDraft.value),
    content: computed(() => content.value),
    isDirty: computed(() => isDirty.value),
    applyDraft,
    markDraftMissing,
    loadSavedVersions,
    refreshPreviewStatus,
    onReleasePublished,
    t: (key, params) => (params ? `${key}:${JSON.stringify(params)}` : key),
  });

  return {
    actions,
    applyDraft,
    markDraftMissing,
    loadSavedVersions,
    refreshPreviewStatus,
    onReleasePublished,
    currentConfigFile,
    currentDraft,
    content,
    isDirty,
  };
}

describe("useDraftActions", () => {
  beforeEach(() => {
    mockState.updateDraft.mockReset();
    mockState.deleteDraft.mockReset();
    mockState.cloneDraft.mockReset();
    mockState.previewDeploymentBundle.mockReset();
    mockState.publishRelease.mockReset();
    mockState.confirm.mockReset();
    mockState.prompt.mockReset();
    mockState.success.mockReset();
    mockState.error.mockReset();
    mockState.warning.mockReset();
  });

  it("saves the current draft content and reloads saved versions", async () => {
    mockState.updateDraft.mockResolvedValue({ ...draft, version: 8 });
    const { actions, applyDraft, loadSavedVersions } = createActions();

    await actions.handleSave();

    expect(mockState.updateDraft).toHaveBeenCalledWith(2, 3, {
      content: "greeting: edited",
      format: "yaml",
      base_version: 7,
    });
    expect(applyDraft).toHaveBeenCalledWith({ ...draft, version: 8 });
    expect(loadSavedVersions).toHaveBeenCalledWith({ keepSelection: false });
    expect(mockState.success).toHaveBeenCalledWith("toast.drafts.saved");
    expect(actions.saving.value).toBe(false);
  });

  it("discards the draft and replaces editor content with preview fallback", async () => {
    mockState.confirm.mockResolvedValue(undefined);
    mockState.deleteDraft.mockResolvedValue(undefined);
    mockState.previewDeploymentBundle.mockResolvedValue(preview);
    const { actions, markDraftMissing, refreshPreviewStatus } = createActions();

    await actions.handleDiscard();

    expect(mockState.confirm).toHaveBeenCalledWith(
      "drafts.discard.prompt",
      "drafts.discard.title",
      expect.objectContaining({
        confirmButtonText: "drafts.discard.confirm",
        cancelButtonText: "common.cancel",
      }),
    );
    expect(mockState.deleteDraft).toHaveBeenCalledWith(2, 3);
    expect(markDraftMissing).toHaveBeenCalledWith("greeting: release");
    expect(refreshPreviewStatus).toHaveBeenCalledWith(2);
    expect(mockState.success).toHaveBeenCalledWith("toast.drafts.discarded");
    expect(actions.discarding.value).toBe(false);
  });

  it("restores the draft from the latest release", async () => {
    mockState.confirm.mockResolvedValue(undefined);
    mockState.cloneDraft.mockResolvedValue({
      draft: { ...draft, content: "greeting: restored", version: 9 },
      source_deployment_instance_id: 2,
      source_kind: "latest_release",
    });
    const { actions, applyDraft, refreshPreviewStatus } = createActions();

    await actions.handleRestoreFromRelease();

    expect(mockState.cloneDraft).toHaveBeenCalledWith(2, 3, {
      source_deployment_instance_id: 2,
      source_kind: "latest_release",
    });
    expect(applyDraft).toHaveBeenCalledWith({
      ...draft,
      content: "greeting: restored",
      version: 9,
    });
    expect(refreshPreviewStatus).toHaveBeenCalledWith(2);
    expect(mockState.success).toHaveBeenCalledWith(
      "toast.drafts.restoredFromRelease",
    );
    expect(actions.restoring.value).toBe(false);
  });

  it("requires a saved draft before publishing", async () => {
    const { actions } = createActions({ dirty: true });

    await actions.handlePublish();

    expect(mockState.warning).toHaveBeenCalledWith(
      "drafts.notice.saveBeforePublish",
    );
    expect(mockState.prompt).not.toHaveBeenCalled();
    expect(mockState.publishRelease).not.toHaveBeenCalled();
  });

  it("publishes the saved draft and reports the release", async () => {
    mockState.prompt.mockResolvedValue({ value: "ship it" });
    mockState.publishRelease.mockResolvedValue(release);
    const { actions, onReleasePublished } = createActions();

    await actions.handlePublish();

    expect(mockState.prompt).toHaveBeenCalledWith(
      "drafts.publish.prompt",
      "drafts.publish.title",
      expect.objectContaining({
        inputType: "textarea",
        inputPlaceholder: "drafts.publish.placeholder",
        confirmButtonText: "common.publish",
        cancelButtonText: "common.cancel",
      }),
    );
    expect(mockState.publishRelease).toHaveBeenCalledWith({
      project_id: 1,
      deployment_instance_id: 2,
      config_file_id: 3,
      change_summary: "ship it",
    });
    expect(mockState.success).toHaveBeenCalledWith(
      'toast.releases.published:{"revision":"r9"}',
    );
    expect(onReleasePublished).toHaveBeenCalledWith(release);
    expect(actions.publishing.value).toBe(false);
  });

  it("surfaces API errors through the shared error mapper", async () => {
    mockState.updateDraft.mockRejectedValue(
      new ApiRequestError(409, {
        code: "draft_version_conflict",
        message: "conflict",
      }),
    );
    const { actions } = createActions();

    await actions.handleSave();

    expect(mockState.error).toHaveBeenCalledWith(
      "api-error:draft_version_conflict:conflict",
    );
    expect(actions.saving.value).toBe(false);
  });
});
