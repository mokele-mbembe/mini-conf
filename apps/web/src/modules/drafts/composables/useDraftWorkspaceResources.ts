import { computed, ref } from "vue";
import * as configFilesApi from "@/api/config-files";
import * as deploymentInstancesApi from "@/api/deployment-instances";
import * as draftsApi from "@/api/drafts";
import { ApiRequestError } from "@/api/error";
import type { ConfigFileSummary } from "@/api/types/config-file";
import type { DeploymentInstanceSummary } from "@/api/types/deployment-instance";
import type { DraftResponse } from "@/api/types/draft";

type ReadableRef<T> = {
  readonly value: T;
};

interface UseDraftWorkspaceResourcesOptions {
  projectId: ReadableRef<number>;
  deploymentId: ReadableRef<number>;
  configFileId: ReadableRef<number>;
  canEdit: ReadableRef<boolean>;
  canViewSavedVersions: ReadableRef<boolean>;
  t: (key: string) => string;
}

interface LoadDraftResourcesOptions {
  resetSavedVersions: () => void;
  loadSavedVersions: (options?: { keepSelection?: boolean }) => Promise<void>;
}

export function useDraftWorkspaceResources(
  options: UseDraftWorkspaceResourcesOptions,
) {
  const deployment = ref<DeploymentInstanceSummary | null>(null);
  const configFile = ref<ConfigFileSummary | null>(null);
  const configFiles = ref<ConfigFileSummary[]>([]);
  const draft = ref<DraftResponse | null>(null);
  const content = ref("");
  const savedContent = ref("");
  const resourceLoading = ref(false);
  const resourceError = ref<ApiRequestError | null>(null);
  const draftWasMissing = ref(false);
  const previewStatusMap = ref<Record<number, string>>({});
  const shellProjectId = ref<number | null>(null);
  const shellDeploymentId = ref<number | null>(null);
  const previewStatusDeploymentId = ref<number | null>(null);

  const resourceNotFound = computed(() => resourceError.value?.status === 404);
  const resourceForbidden = computed(() => resourceError.value?.status === 403);
  const draftReady = computed(() => draft.value !== null);
  const isDirty = computed(() => content.value !== savedContent.value);
  const draftVersion = computed(() => draft.value?.version ?? null);
  const versionLabel = computed(() => {
    if (draft.value) {
      return String(draft.value.version);
    }

    return draftWasMissing.value
      ? options.t("drafts.field.newDraft")
      : options.t("drafts.field.unknownVersion");
  });

  async function loadDraftResources(context: LoadDraftResourcesOptions) {
    const pid = options.projectId.value;
    const did = options.deploymentId.value;
    const cid = options.configFileId.value;
    if (Number.isNaN(pid) || Number.isNaN(did) || Number.isNaN(cid)) return;

    resourceLoading.value = true;
    resourceError.value = null;
    configFile.value = null;
    draft.value = null;
    content.value = "";
    savedContent.value = "";
    draftWasMissing.value = false;
    context.resetSavedVersions();

    try {
      await loadWorkspaceShell(pid, did);

      if (deployment.value === null) {
        resourceError.value = new ApiRequestError(404, {
          code: "resource_not_found",
          message: "resource not found",
        });
        return;
      }

      const configResult =
        configFiles.value.find((item) => item.id === cid) ??
        (await configFilesApi.getConfigFile(cid));
      configFile.value = configResult;

      if (
        deployment.value.project_id !== pid ||
        configResult.project_id !== pid
      ) {
        resourceError.value = new ApiRequestError(404, {
          code: "resource_not_found",
          message: "resource not found",
        });
        return;
      }

      if (!options.canEdit.value) {
        return;
      }

      void loadPreviewStatusIfNeeded(did);
      if (options.canViewSavedVersions.value) {
        await context.loadSavedVersions({ keepSelection: false });
      }

      try {
        const draftResult = await draftsApi.getDraft(did, cid);
        applyDraft(draftResult);
      } catch (err) {
        if (err instanceof ApiRequestError && err.code === "draft_not_found") {
          draftWasMissing.value = true;
          content.value = "";
          savedContent.value = "";
          return;
        }
        throw err;
      }
    } catch (err) {
      if (err instanceof ApiRequestError) {
        resourceError.value = err;
      } else {
        resourceError.value = new ApiRequestError(0, {
          code: "unknown_error",
          message: options.t("drafts.page.loadError"),
        });
      }
    } finally {
      resourceLoading.value = false;
    }
  }

  async function loadWorkspaceShell(projectId: number, deploymentId: number) {
    if (
      shellProjectId.value === projectId &&
      shellDeploymentId.value === deploymentId &&
      deployment.value !== null &&
      configFiles.value.length > 0
    ) {
      return;
    }

    deployment.value = null;
    configFiles.value = [];
    previewStatusMap.value = {};
    previewStatusDeploymentId.value = null;

    const [deploymentResult, configListResult] = await Promise.all([
      deploymentInstancesApi.getDeploymentInstance(deploymentId),
      configFilesApi.listConfigFiles({
        project_id: projectId,
        status: "active",
      }),
    ]);

    deployment.value = deploymentResult;
    configFiles.value = configListResult.items;
    shellProjectId.value = projectId;
    shellDeploymentId.value = deploymentId;
  }

  async function loadPreviewStatusIfNeeded(deploymentId: number) {
    if (previewStatusDeploymentId.value === deploymentId) {
      return;
    }

    await loadPreviewStatus(deploymentId);
  }

  async function loadPreviewStatus(deploymentId: number) {
    try {
      const preview =
        await deploymentInstancesApi.previewDeploymentBundle(deploymentId);
      const map: Record<number, string> = {};
      for (const item of preview.items) {
        if (
          item.status === "missing_required" ||
          item.status === "missing_optional"
        ) {
          map[item.config_file_id] = item.status;
        } else {
          map[item.config_file_id] = item.source;
        }
      }
      previewStatusMap.value = map;
      previewStatusDeploymentId.value = deploymentId;
    } catch {
      // Non-critical; silently ignore.
    }
  }

  function applyDraft(value: DraftResponse) {
    draft.value = value;
    draftWasMissing.value = false;
    content.value = value.content;
    savedContent.value = value.content;
  }

  function markDraftMissing(fallbackContent: string) {
    draft.value = null;
    draftWasMissing.value = true;
    content.value = fallbackContent;
    savedContent.value = fallbackContent;
  }

  return {
    deployment,
    configFile,
    configFiles,
    draft,
    content,
    savedContent,
    resourceLoading,
    resourceError,
    draftWasMissing,
    previewStatusMap,
    resourceNotFound,
    resourceForbidden,
    draftReady,
    isDirty,
    draftVersion,
    versionLabel,
    loadDraftResources,
    loadPreviewStatus,
    applyDraft,
    markDraftMissing,
  };
}
