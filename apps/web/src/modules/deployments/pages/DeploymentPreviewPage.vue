<template>
  <div class="deployment-preview-page page-container">
    <LoadingState v-if="projectLoading" />

    <NotFoundState
      v-else-if="projectError && projectError.status === 404"
      :title="t('project.notFound.title')"
      :subtitle="t('project.notFound.subtitle')"
    />

    <ForbiddenState
      v-else-if="projectError && projectError.status === 403"
      :subtitle="t('project.forbidden.subtitle')"
    />

    <ErrorState
      v-else-if="projectError"
      :title="projectError.message"
      @retry="loadAll"
    />

    <template v-else-if="project">
      <PageHeader :title="pageTitle" :subtitle="pageSubtitle">
        <template #actions>
          <div class="deployment-preview-page__header-actions">
            <el-button text type="primary" @click="backToDeployment">
              {{ t("preview.action.backToDeployment") }}
            </el-button>
            <el-button
              type="primary"
              :disabled="!preview"
              @click="copyBundleJson"
            >
              {{ t("preview.action.copyBundle") }}
            </el-button>
          </div>
        </template>
      </PageHeader>

      <ProjectTabs />

      <div class="deployment-preview-page__section">
        <LoadingState v-if="resourceLoading" />

        <ForbiddenState
          v-else-if="!canPreview || resourceError?.status === 403"
          :subtitle="t('state.forbidden.subtitle')"
        />

        <NotFoundState
          v-else-if="resourceError?.status === 404"
          :title="t('deployments.notFound.title')"
          :subtitle="t('deployments.notFound.subtitle')"
        />

        <ErrorState
          v-else-if="resourceError"
          :title="t('preview.page.loadError')"
          :subtitle="getErrorMessage(resourceError.code, resourceError.message)"
          @retry="loadPreviewResources"
        />

        <template v-else-if="deployment && preview">
          <el-alert
            v-if="missingRequiredCount > 0"
            :title="
              t('preview.notice.missingRequired', {
                count: missingRequiredCount,
              })
            "
            type="warning"
            show-icon
            :closable="false"
            class="deployment-preview-page__notice"
          />

          <el-descriptions
            :column="3"
            border
            class="deployment-preview-page__meta"
          >
            <el-descriptions-item :label="t('deployments.field.name')">
              {{ deployment.name }}
            </el-descriptions-item>
            <el-descriptions-item :label="t('deployments.field.deploymentKey')">
              <span class="deployment-preview-page__code">
                {{ deployment.deployment_key }}
              </span>
            </el-descriptions-item>
            <el-descriptions-item :label="t('preview.field.bundleConfigs')">
              {{ preview.open_bundle_preview.configs.length }}
            </el-descriptions-item>
          </el-descriptions>

          <div class="deployment-preview-page__table page-table-shell">
            <el-table :data="preview.items" stripe style="width: 100%">
              <el-table-column
                prop="code"
                :label="t('configFiles.column.code')"
                min-width="150"
              >
                <template #default="{ row }">
                  <span class="deployment-preview-page__code">
                    {{ row.code }}
                  </span>
                </template>
              </el-table-column>

              <el-table-column
                prop="name"
                :label="t('configFiles.column.name')"
                min-width="150"
              />

              <el-table-column
                :label="t('configFiles.column.required')"
                width="90"
                align="center"
              >
                <template #default="{ row }">
                  <el-tag v-if="row.is_required" size="small" type="danger">
                    {{ t("configFiles.required") }}
                  </el-tag>
                  <span v-else class="deployment-preview-page__muted">—</span>
                </template>
              </el-table-column>

              <el-table-column
                :label="t('preview.column.source')"
                width="150"
                align="center"
              >
                <template #default="{ row }">
                  <el-tag size="small" :type="sourceTagType(row.source)">
                    {{ sourceLabel(row.source) }}
                  </el-tag>
                </template>
              </el-table-column>

              <el-table-column
                :label="t('preview.column.status')"
                width="160"
                align="center"
              >
                <template #default="{ row }">
                  <el-tag size="small" :type="statusTagType(row.status)">
                    {{ previewStatusLabel(row.status) }}
                  </el-tag>
                </template>
              </el-table-column>

              <el-table-column
                prop="revision"
                :label="t('preview.column.revision')"
                min-width="130"
              >
                <template #default="{ row }">
                  {{ row.revision ?? t("preview.emptyRevision") }}
                </template>
              </el-table-column>

              <el-table-column
                :label="t('deployments.column.actions')"
                width="340"
                align="center"
                fixed="right"
              >
                <template #default="{ row }">
                  <div class="deployment-preview-page__row-actions">
                    <el-button
                      text
                      type="primary"
                      size="small"
                      @click="openDraft(row.config_file_id)"
                    >
                      {{ t("deployments.action.editCurrentDraft") }}
                    </el-button>
                    <el-button
                      text
                      type="primary"
                      size="small"
                      @click="openReleases(row.config_file_id)"
                    >
                      {{ t("preview.action.viewReleases") }}
                    </el-button>
                    <el-button
                      v-if="latestReleaseMap[row.config_file_id]"
                      text
                      type="warning"
                      size="small"
                      :loading="restoringReleaseConfigId === row.config_file_id"
                      @click="handleRestoreLatestRelease(row)"
                    >
                      {{ t("preview.action.restoreLatestRelease") }}
                    </el-button>
                    <el-button
                      v-if="row.source === 'draft'"
                      text
                      type="danger"
                      size="small"
                      :loading="discardingConfigId === row.config_file_id"
                      @click="handleDiscardDraft(row)"
                    >
                      {{ t("drafts.action.discard") }}
                    </el-button>
                  </div>
                </template>
              </el-table-column>
            </el-table>
          </div>

          <div class="deployment-preview-page__bundle">
            <div class="deployment-preview-page__bundle-header">
              <h3>{{ t("preview.bundle.title") }}</h3>
              <el-button text type="primary" @click="copyBundleJson">
                {{ t("common.copy") }}
              </el-button>
            </div>
            <el-input
              :model-value="bundleJson"
              type="textarea"
              :rows="14"
              readonly
              class="deployment-preview-page__json"
            />
          </div>
        </template>
      </div>
    </template>

    <DraftEditorOverlay
      v-if="draftOverlayVisible"
      :visible="draftOverlayVisible"
      :config-file-id="draftOverlayConfigFileId"
      @request-close="closeDraftOverlay"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, ref, watch } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { useRoute, useRouter } from "vue-router";
import { useProjectContext } from "@/modules/projects/composables/useProjectContext";
import ProjectTabs from "@/modules/projects/components/ProjectTabs.vue";
import PageHeader from "@/shared/components/PageHeader.vue";
import LoadingState from "@/shared/states/LoadingState.vue";
import ErrorState from "@/shared/states/ErrorState.vue";
import ForbiddenState from "@/shared/states/ForbiddenState.vue";
import NotFoundState from "@/shared/states/NotFoundState.vue";
import { ROUTE_NAMES } from "@/shared/constants/routes";
import * as deploymentInstancesApi from "@/api/deployment-instances";
import * as draftsApi from "@/api/drafts";
import * as releasesApi from "@/api/releases";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type {
  DeploymentBundlePreviewResponse,
  DeploymentInstanceSummary,
  DeploymentPreviewItem,
} from "@/api/types/deployment-instance";
import type { ReleaseSummary } from "@/api/types/release";
import { useI18nText } from "@/shared/i18n";

const route = useRoute();
const router = useRouter();
const { t } = useI18nText();
const DraftEditorOverlay = defineAsyncComponent(
  () => import("@/modules/drafts/components/DraftEditorOverlay.vue"),
);

const {
  project,
  loading: projectLoading,
  error: projectError,
  fetchProject,
} = useProjectContext();

const projectId = computed(() => Number(route.params.projectId));
const deploymentId = computed(() => Number(route.params.deploymentId));
const canPreview = computed(() => {
  const role = project.value?.current_user_role;
  return role === "admin" || role === "editor";
});

const deployment = ref<DeploymentInstanceSummary | null>(null);
const preview = ref<DeploymentBundlePreviewResponse | null>(null);
const resourceLoading = ref(false);
const resourceError = ref<ApiRequestError | null>(null);
const discardingConfigId = ref<number | null>(null);
const restoringReleaseConfigId = ref<number | null>(null);
const latestReleaseMap = ref<Record<number, ReleaseSummary>>({});
const draftOverlayConfigFileId = computed(() => {
  const raw = route.query.draftConfigFileId;
  const value = Array.isArray(raw) ? raw[0] : raw;
  if (typeof value !== "string") {
    return null;
  }

  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
});
const draftOverlayVisible = computed(
  () => draftOverlayConfigFileId.value !== null,
);

const pageTitle = computed(() => t("preview.page.title"));
const pageSubtitle = computed(() => {
  if (!deployment.value) return undefined;
  return `${deployment.value.name} / ${deployment.value.deployment_key}`;
});
const missingRequiredCount = computed(
  () =>
    preview.value?.items.filter((item) => item.status === "missing_required")
      .length ?? 0,
);
const bundleJson = computed(() =>
  preview.value
    ? JSON.stringify(preview.value.open_bundle_preview, null, 2)
    : "",
);

async function loadPreviewResources() {
  const did = deploymentId.value;
  if (Number.isNaN(did)) return;

  resourceLoading.value = true;
  resourceError.value = null;
  deployment.value = null;
  preview.value = null;
  latestReleaseMap.value = {};

  try {
    const deploymentResult =
      await deploymentInstancesApi.getDeploymentInstance(did);
    deployment.value = deploymentResult;

    if (deploymentResult.project_id !== projectId.value) {
      resourceError.value = new ApiRequestError(404, {
        code: "deployment_instance_not_found",
        message: "deployment instance not found",
      });
      return;
    }

    if (!canPreview.value) {
      return;
    }

    preview.value = await deploymentInstancesApi.previewDeploymentBundle(did);
    await loadLatestReleaseHints();
  } catch (err) {
    if (err instanceof ApiRequestError) {
      resourceError.value = err;
    } else {
      resourceError.value = new ApiRequestError(0, {
        code: "unknown_error",
        message: t("preview.page.loadError"),
      });
    }
  } finally {
    resourceLoading.value = false;
  }
}

async function loadLatestReleaseHints() {
  latestReleaseMap.value = {};
  const did = deploymentId.value;
  if (Number.isNaN(did) || !canPreview.value) return;

  try {
    const releaseResult = await releasesApi.listReleases({
      project_id: projectId.value,
      deployment_instance_id: did,
    });
    const map: Record<number, ReleaseSummary> = {};
    for (const release of releaseResult.items) {
      map[release.config_file_id] ??= release;
    }
    latestReleaseMap.value = map;
  } catch {
    // Non-critical row actions; keep preview usable even if release hints fail.
  }
}

async function loadAll() {
  const pid = projectId.value;
  if (Number.isNaN(pid)) return;
  await fetchProject(pid);
  await loadPreviewResources();
}

function sourceLabel(source: string): string {
  return t(`preview.source.${source}`);
}

function previewStatusLabel(status: string): string {
  return t(`preview.status.${status}`);
}

function sourceTagType(source: string) {
  if (source === "draft") return "warning";
  if (source === "latest_release") return "success";
  return "info";
}

function statusTagType(status: string) {
  if (status === "missing_required") return "danger";
  if (status === "missing_optional") return "info";
  return "success";
}

async function copyBundleJson() {
  if (!bundleJson.value) return;

  try {
    const clipboard = globalThis.navigator?.clipboard;
    if (!clipboard) {
      throw new Error("clipboard_unavailable");
    }
    await clipboard.writeText(bundleJson.value);
    ElMessage.success(t("toast.preview.bundleCopied"));
  } catch {
    ElMessage.error(t("toast.operationFailed"));
  }
}

async function handleRestoreLatestRelease(row: DeploymentPreviewItem) {
  try {
    await ElMessageBox.confirm(
      t("preview.restoreLatestRelease.prompt"),
      t("preview.restoreLatestRelease.title"),
      {
        confirmButtonText: t("preview.restoreLatestRelease.confirm"),
        cancelButtonText: t("common.cancel"),
        type: "warning",
      },
    );
  } catch {
    return;
  }

  restoringReleaseConfigId.value = row.config_file_id;
  try {
    await draftsApi.cloneDraft(deploymentId.value, row.config_file_id, {
      source_deployment_instance_id: deploymentId.value,
      source_kind: "latest_release",
    });
    ElMessage.success(t("toast.preview.restoredLatestRelease"));
    await loadPreviewResources();
  } catch (err) {
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    restoringReleaseConfigId.value = null;
  }
}

async function handleDiscardDraft(row: DeploymentPreviewItem) {
  try {
    await ElMessageBox.confirm(
      t("drafts.discard.prompt"),
      t("drafts.discard.title"),
      {
        confirmButtonText: t("drafts.discard.confirm"),
        cancelButtonText: t("common.cancel"),
        type: "warning",
      },
    );
  } catch {
    return;
  }

  discardingConfigId.value = row.config_file_id;
  try {
    await draftsApi.deleteDraft(deploymentId.value, row.config_file_id);
    ElMessage.success(t("toast.drafts.discarded"));
    await loadPreviewResources();
  } catch (err) {
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    discardingConfigId.value = null;
  }
}

function backToDeployment() {
  router.push({
    name: ROUTE_NAMES.DEPLOYMENT_DETAIL,
    params: {
      projectId: route.params.projectId,
      deploymentId: route.params.deploymentId,
    },
  });
}

function openDraft(configFileId: number) {
  router.push({
    name: ROUTE_NAMES.DEPLOYMENT_PREVIEW,
    params: {
      projectId: route.params.projectId,
      deploymentId: route.params.deploymentId,
    },
    query: {
      ...route.query,
      draftConfigFileId: String(configFileId),
    },
  });
}

function openReleases(configFileId: number) {
  router.push({
    name: ROUTE_NAMES.RELEASE_LIST,
    params: {
      projectId: route.params.projectId,
    },
    query: {
      deployment_instance_id: String(deploymentId.value),
      config_file_id: String(configFileId),
    },
  });
}

async function closeDraftOverlay() {
  const query = { ...route.query };
  delete query.draftConfigFileId;
  await router.push({
    name: ROUTE_NAMES.DEPLOYMENT_PREVIEW,
    params: {
      projectId: route.params.projectId,
      deploymentId: route.params.deploymentId,
    },
    query,
  });
  await loadPreviewResources();
}

onMounted(loadAll);

watch(
  () => [route.params.projectId, route.params.deploymentId],
  () => loadAll(),
);
</script>

<style scoped>
.deployment-preview-page {
  width: 100%;
}

.deployment-preview-page__section {
  margin-top: var(--spacing-md);
}

.deployment-preview-page__header-actions,
.deployment-preview-page__bundle-header,
.deployment-preview-page__row-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: flex-end;
  gap: var(--spacing-sm);
}

.deployment-preview-page__row-actions {
  justify-content: center;
  gap: var(--spacing-xs);
}

.deployment-preview-page__notice,
.deployment-preview-page__meta,
.deployment-preview-page__table {
  margin-bottom: var(--spacing-md);
}

.deployment-preview-page__code {
  font-family: monospace;
}

.deployment-preview-page__muted {
  color: var(--color-text-secondary);
}

.deployment-preview-page__bundle {
  margin-top: var(--spacing-lg);
}

.deployment-preview-page__bundle-header {
  justify-content: space-between;
  margin-bottom: var(--spacing-sm);
}

.deployment-preview-page__bundle-header h3 {
  font-size: var(--font-size-lg);
  font-weight: 600;
}

.deployment-preview-page__json :deep(textarea) {
  font-family:
    ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono",
    "Courier New", monospace;
  line-height: 1.55;
}
</style>
