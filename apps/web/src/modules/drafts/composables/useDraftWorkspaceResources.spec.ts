import { computed, ref } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ApiRequestError } from "@/api/error";
import type { ConfigFileSummary } from "@/api/types/config-file";
import type {
  DeploymentBundlePreviewResponse,
  DeploymentInstanceSummary,
} from "@/api/types/deployment-instance";
import type { DraftResponse } from "@/api/types/draft";

const mockState = vi.hoisted(() => ({
  getDeploymentInstance: vi.fn(),
  previewDeploymentBundle: vi.fn(),
  getConfigFile: vi.fn(),
  listConfigFiles: vi.fn(),
  getDraft: vi.fn(),
}));

vi.mock("@/api/deployment-instances", () => ({
  getDeploymentInstance: mockState.getDeploymentInstance,
  previewDeploymentBundle: mockState.previewDeploymentBundle,
}));

vi.mock("@/api/config-files", () => ({
  getConfigFile: mockState.getConfigFile,
  listConfigFiles: mockState.listConfigFiles,
}));

vi.mock("@/api/drafts", () => ({
  getDraft: mockState.getDraft,
}));

import { useDraftWorkspaceResources } from "./useDraftWorkspaceResources";

const deployment: DeploymentInstanceSummary = {
  id: 2,
  deployment_uid: "dep_2",
  project_id: 1,
  environment_id: 10,
  environment_code: "prod",
  environment_name: "Production",
  deployment_key: "store-001",
  name: "Store 001",
  description: null,
  is_template: false,
  template_source_id: null,
  status: "active",
  is_archived: false,
  archived_at: null,
  archived_by: null,
  archive_reason: null,
  deleted_at: null,
  deleted_by: null,
  delete_reason: null,
};

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

const preview: DeploymentBundlePreviewResponse = {
  deployment_instance_id: 2,
  items: [
    {
      config_file_id: 3,
      code: "app",
      name: "App Config",
      is_required: true,
      source: "draft",
      status: "ok",
      format: "yaml",
      content: "greeting: saved",
      revision: null,
    },
    {
      config_file_id: 4,
      code: "secret",
      name: "Secret Config",
      is_required: true,
      source: "missing",
      status: "missing_required",
      format: "json",
      content: null,
      revision: null,
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

function flushPromises() {
  return Promise.resolve().then(() => Promise.resolve());
}

function createResources(options?: { canEdit?: boolean }) {
  const projectId = ref(1);
  const deploymentId = ref(2);
  const configFileId = ref(3);
  const canEdit = ref(options?.canEdit ?? true);
  const canViewSavedVersions = computed(() => canEdit.value);
  const resetSavedVersions = vi.fn();
  const loadSavedVersions = vi.fn().mockResolvedValue(undefined);

  const resources = useDraftWorkspaceResources({
    projectId: computed(() => projectId.value),
    deploymentId: computed(() => deploymentId.value),
    configFileId: computed(() => configFileId.value),
    canEdit: computed(() => canEdit.value),
    canViewSavedVersions,
    t: (key) => key,
  });

  return {
    resources,
    projectId,
    deploymentId,
    configFileId,
    canEdit,
    resetSavedVersions,
    loadSavedVersions,
  };
}

describe("useDraftWorkspaceResources", () => {
  beforeEach(() => {
    mockState.getDeploymentInstance.mockReset();
    mockState.previewDeploymentBundle.mockReset();
    mockState.getConfigFile.mockReset();
    mockState.listConfigFiles.mockReset();
    mockState.getDraft.mockReset();
  });

  it("loads deployment, config metadata, saved versions, preview status, and draft", async () => {
    mockState.getDeploymentInstance.mockResolvedValue(deployment);
    mockState.getConfigFile.mockResolvedValue(configFile);
    mockState.listConfigFiles.mockResolvedValue({ items: [configFile] });
    mockState.previewDeploymentBundle.mockResolvedValue(preview);
    mockState.getDraft.mockResolvedValue(draft);
    const { resources, resetSavedVersions, loadSavedVersions } =
      createResources();

    await resources.loadDraftResources({
      resetSavedVersions,
      loadSavedVersions,
    });
    await flushPromises();

    expect(mockState.listConfigFiles).toHaveBeenCalledWith({
      project_id: 1,
      status: "active",
    });
    expect(resetSavedVersions).toHaveBeenCalledOnce();
    expect(loadSavedVersions).toHaveBeenCalledWith({ keepSelection: false });
    expect(resources.deployment.value).toEqual(deployment);
    expect(resources.configFile.value).toEqual(configFile);
    expect(resources.configFiles.value).toEqual([configFile]);
    expect(resources.draft.value).toEqual(draft);
    expect(resources.content.value).toBe("greeting: saved");
    expect(resources.savedContent.value).toBe("greeting: saved");
    expect(resources.previewStatusMap.value).toEqual({
      3: "draft",
      4: "missing_required",
    });
    expect(resources.resourceLoading.value).toBe(false);
  });

  it("treats cross-project resources as not found", async () => {
    mockState.getDeploymentInstance.mockResolvedValue({
      ...deployment,
      project_id: 99,
    });
    mockState.getConfigFile.mockResolvedValue(configFile);
    mockState.listConfigFiles.mockResolvedValue({ items: [configFile] });
    const { resources, resetSavedVersions, loadSavedVersions } =
      createResources();

    await resources.loadDraftResources({
      resetSavedVersions,
      loadSavedVersions,
    });

    expect(resources.resourceNotFound.value).toBe(true);
    expect(resources.resourceError.value?.code).toBe("resource_not_found");
    expect(mockState.getDraft).not.toHaveBeenCalled();
    expect(loadSavedVersions).not.toHaveBeenCalled();
  });

  it("marks a missing draft as a new draft state", async () => {
    mockState.getDeploymentInstance.mockResolvedValue(deployment);
    mockState.getConfigFile.mockResolvedValue(configFile);
    mockState.listConfigFiles.mockResolvedValue({ items: [configFile] });
    mockState.previewDeploymentBundle.mockResolvedValue(preview);
    mockState.getDraft.mockRejectedValue(
      new ApiRequestError(404, {
        code: "draft_not_found",
        message: "Draft not found",
      }),
    );
    const { resources, resetSavedVersions, loadSavedVersions } =
      createResources();

    await resources.loadDraftResources({
      resetSavedVersions,
      loadSavedVersions,
    });

    expect(resources.draft.value).toBeNull();
    expect(resources.draftWasMissing.value).toBe(true);
    expect(resources.content.value).toBe("");
    expect(resources.savedContent.value).toBe("");
    expect(resources.versionLabel.value).toBe("drafts.field.newDraft");
    expect(resources.resourceError.value).toBeNull();
  });

  it("does not load edit-only resources for viewers", async () => {
    mockState.getDeploymentInstance.mockResolvedValue(deployment);
    mockState.getConfigFile.mockResolvedValue(configFile);
    mockState.listConfigFiles.mockResolvedValue({ items: [configFile] });
    const { resources, resetSavedVersions, loadSavedVersions } =
      createResources({ canEdit: false });

    await resources.loadDraftResources({
      resetSavedVersions,
      loadSavedVersions,
    });

    expect(resources.deployment.value).toEqual(deployment);
    expect(resources.configFile.value).toEqual(configFile);
    expect(mockState.previewDeploymentBundle).not.toHaveBeenCalled();
    expect(mockState.getDraft).not.toHaveBeenCalled();
    expect(loadSavedVersions).not.toHaveBeenCalled();
  });

  it("tracks dirty state and can mark the current draft as missing", () => {
    const { resources } = createResources();

    resources.applyDraft(draft);
    resources.content.value = "greeting: edited";

    expect(resources.isDirty.value).toBe(true);

    resources.markDraftMissing("greeting: release");

    expect(resources.draft.value).toBeNull();
    expect(resources.draftWasMissing.value).toBe(true);
    expect(resources.content.value).toBe("greeting: release");
    expect(resources.savedContent.value).toBe("greeting: release");
    expect(resources.isDirty.value).toBe(false);
  });
});
