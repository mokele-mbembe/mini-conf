<template>
  <div class="draft-editor-page page-container">
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
          <div class="draft-editor-page__header-actions">
            <el-button v-if="canEdit" text type="primary" @click="goToPreview">
              {{ t("drafts.action.preview") }}
            </el-button>
            <el-button
              v-if="canPublish"
              type="success"
              :loading="publishing"
              :disabled="resourceLoading || !draftReady"
              @click="handlePublish"
            >
              {{ t("drafts.action.publish") }}
            </el-button>
            <el-button
              v-if="canEdit"
              type="primary"
              :loading="saving"
              :disabled="resourceLoading"
              @click="handleSave"
            >
              {{ t("common.save") }}
            </el-button>
          </div>
        </template>
      </PageHeader>

      <ProjectTabs />

      <div class="draft-editor-page__section">
        <div class="draft-editor-page__nav">
          <el-button text type="primary" @click="backToDeployment">
            {{ t("drafts.action.backToDeployment") }}
          </el-button>
        </div>

        <LoadingState v-if="resourceLoading" />

        <NotFoundState
          v-else-if="resourceNotFound"
          :title="t('drafts.notFound.title')"
          :subtitle="t('drafts.notFound.subtitle')"
        />

        <ForbiddenState
          v-else-if="!canEdit || resourceForbidden"
          :subtitle="t('state.forbidden.subtitle')"
        />

        <ErrorState
          v-else-if="resourceError"
          :title="t('drafts.page.loadError')"
          :subtitle="getErrorMessage(resourceError.code, resourceError.message)"
          @retry="loadDraftResources"
        />

        <template v-else-if="deployment && configFile">
          <!-- Config file switcher -->
          <div
            v-if="configFiles.length > 1"
            class="draft-editor-page__config-switcher"
          >
            <div class="draft-editor-page__config-switcher-label">
              {{ t("drafts.configSwitcher.title") }}
            </div>
            <el-radio-group
              :model-value="configFileId"
              @update:model-value="switchConfigFile"
            >
              <el-radio-button
                v-for="cf in configFiles"
                :key="cf.id"
                :value="cf.id"
              >
                <span>{{ cf.code }}</span>
                <el-tag
                  v-if="previewStatusMap[cf.id]"
                  size="small"
                  :type="configStatusTagType(previewStatusMap[cf.id])"
                  class="draft-editor-page__config-status-tag"
                >
                  {{ configStatusLabel(previewStatusMap[cf.id]) }}
                </el-tag>
              </el-radio-button>
            </el-radio-group>
          </div>

          <div class="draft-editor-page__meta">
            <el-descriptions :column="3" border>
              <el-descriptions-item :label="t('deployments.field.name')">
                {{ deployment.name }}
              </el-descriptions-item>
              <el-descriptions-item
                :label="t('deployments.field.deploymentKey')"
              >
                <span class="draft-editor-page__code">
                  {{ deployment.deployment_key }}
                </span>
              </el-descriptions-item>
              <el-descriptions-item :label="t('deployments.field.type')">
                {{
                  deployment.is_template
                    ? t("deployments.type.template")
                    : t("deployments.type.instance")
                }}
              </el-descriptions-item>
              <el-descriptions-item :label="t('configFiles.column.code')">
                <span class="draft-editor-page__code">
                  {{ configFile.code }}
                </span>
              </el-descriptions-item>
              <el-descriptions-item :label="t('configFiles.column.format')">
                <el-tag size="small" type="info">
                  {{ configFile.format }}
                </el-tag>
              </el-descriptions-item>
              <el-descriptions-item :label="t('drafts.field.version')">
                {{ versionLabel }}
              </el-descriptions-item>
            </el-descriptions>
          </div>

          <!-- Draft actions toolbar -->
          <div v-if="canEdit" class="draft-editor-page__draft-actions">
            <el-button
              v-if="draftReady"
              text
              type="danger"
              size="small"
              :loading="discarding"
              @click="handleDiscard"
            >
              {{ t("drafts.action.discard") }}
            </el-button>
            <el-button
              text
              type="primary"
              size="small"
              :loading="restoring"
              @click="handleRestoreFromRelease"
            >
              {{ t("drafts.action.restoreFromRelease") }}
            </el-button>
            <el-button
              text
              type="primary"
              size="small"
              @click="cloneDialogVisible = true"
            >
              {{ t("drafts.action.cloneFromInstance") }}
            </el-button>
          </div>

          <el-alert
            v-if="deployment.is_template"
            :title="t('drafts.notice.template')"
            type="info"
            show-icon
            :closable="false"
            class="draft-editor-page__notice"
          />

          <el-alert
            v-if="isDirty"
            :title="t('drafts.notice.unsaved')"
            type="warning"
            show-icon
            :closable="false"
            class="draft-editor-page__notice"
          />

          <el-input
            v-model="content"
            type="textarea"
            :rows="24"
            :placeholder="t('drafts.editor.placeholder')"
            class="draft-editor-page__editor"
          />
        </template>
      </div>
    </template>

    <!-- Clone from other instance dialog -->
    <el-dialog
      v-model="cloneDialogVisible"
      :title="t('drafts.cloneDialog.title')"
      width="480px"
      destroy-on-close
    >
      <el-form label-position="top">
        <el-form-item :label="t('drafts.cloneDialog.sourceInstance')">
          <el-select
            v-model="cloneSourceInstanceId"
            :placeholder="t('drafts.cloneDialog.selectInstance')"
            filterable
            style="width: 100%"
            :loading="cloneInstancesLoading"
          >
            <el-option
              v-for="inst in cloneableInstances"
              :key="inst.id"
              :label="`${inst.name} (${inst.deployment_key})`"
              :value="inst.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('drafts.cloneDialog.sourceKind')">
          <el-radio-group v-model="cloneSourceKind">
            <el-radio value="draft">
              {{ t("drafts.cloneDialog.kindDraft") }}
            </el-radio>
            <el-radio value="latest_release">
              {{ t("drafts.cloneDialog.kindRelease") }}
            </el-radio>
          </el-radio-group>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="cloneDialogVisible = false">
          {{ t("common.cancel") }}
        </el-button>
        <el-button
          type="primary"
          :loading="cloning"
          :disabled="!cloneSourceInstanceId"
          @click="handleCloneFromInstance"
        >
          {{ t("drafts.cloneDialog.submit") }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { onBeforeRouteLeave, useRoute, useRouter } from "vue-router";
import { useProjectContext } from "@/modules/projects/composables/useProjectContext";
import ProjectTabs from "@/modules/projects/components/ProjectTabs.vue";
import PageHeader from "@/shared/components/PageHeader.vue";
import LoadingState from "@/shared/states/LoadingState.vue";
import ErrorState from "@/shared/states/ErrorState.vue";
import ForbiddenState from "@/shared/states/ForbiddenState.vue";
import NotFoundState from "@/shared/states/NotFoundState.vue";
import { ROUTE_NAMES } from "@/shared/constants/routes";
import * as deploymentInstancesApi from "@/api/deployment-instances";
import * as configFilesApi from "@/api/config-files";
import * as draftsApi from "@/api/drafts";
import * as releasesApi from "@/api/releases";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type { ConfigFileSummary } from "@/api/types/config-file";
import type { DeploymentInstanceSummary } from "@/api/types/deployment-instance";
import type { DraftResponse } from "@/api/types/draft";
import { useI18nText } from "@/shared/i18n";

const route = useRoute();
const router = useRouter();
const { t } = useI18nText();

const {
  project,
  loading: projectLoading,
  error: projectError,
  fetchProject,
} = useProjectContext();

const projectId = computed(() => Number(route.params.projectId));
const deploymentId = computed(() => Number(route.params.deploymentId));
const configFileId = computed(() => Number(route.params.configFileId));
const canEdit = computed(() => {
  const role = project.value?.current_user_role;
  return role === "admin" || role === "editor";
});

const deployment = ref<DeploymentInstanceSummary | null>(null);
const configFile = ref<ConfigFileSummary | null>(null);
const configFiles = ref<ConfigFileSummary[]>([]);
const draft = ref<DraftResponse | null>(null);
const content = ref("");
const savedContent = ref("");
const resourceLoading = ref(false);
const resourceError = ref<ApiRequestError | null>(null);
const saving = ref(false);
const publishing = ref(false);
const discarding = ref(false);
const restoring = ref(false);
const draftWasMissing = ref(false);

// Preview status per config (for switcher badges)
const previewStatusMap = ref<Record<number, string>>({});

// Clone from instance dialog
const cloneDialogVisible = ref(false);
const cloneSourceInstanceId = ref<number | null>(null);
const cloneSourceKind = ref<"draft" | "latest_release">("draft");
const cloneableInstances = ref<DeploymentInstanceSummary[]>([]);
const cloneInstancesLoading = ref(false);
const cloning = ref(false);

const resourceNotFound = computed(() => resourceError.value?.status === 404);
const resourceForbidden = computed(() => resourceError.value?.status === 403);
const draftReady = computed(() => draft.value !== null);
const isDirty = computed(() => content.value !== savedContent.value);
const canPublish = computed(
  () =>
    canEdit.value && deployment.value !== null && !deployment.value.is_template,
);
const pageTitle = computed(() => {
  if (configFile.value) {
    return t("drafts.page.title", { config: configFile.value.code });
  }

  return t("drafts.page.fallbackTitle");
});
const pageSubtitle = computed(() => {
  if (!deployment.value) {
    return undefined;
  }

  return `${deployment.value.name} / ${deployment.value.deployment_key}`;
});
const versionLabel = computed(() => {
  if (draft.value) {
    return String(draft.value.version);
  }

  return draftWasMissing.value
    ? t("drafts.field.newDraft")
    : t("drafts.field.unknownVersion");
});

function configStatusTagType(source: string) {
  switch (source) {
    case "draft":
      return "warning";
    case "latest_release":
      return "success";
    case "missing_required":
      return "danger";
    default:
      return "info";
  }
}

function configStatusLabel(source: string) {
  switch (source) {
    case "draft":
      return t("preview.source.draft");
    case "latest_release":
      return t("preview.source.latest_release");
    case "missing_required":
      return t("preview.status.missing_required");
    case "missing_optional":
      return t("preview.status.missing_optional");
    default:
      return t("preview.source.none");
  }
}

async function confirmIfDirty(): Promise<boolean> {
  if (!isDirty.value) return true;
  try {
    await ElMessageBox.confirm(
      t("drafts.navigate.prompt"),
      t("drafts.navigate.title"),
      {
        confirmButtonText: t("drafts.navigate.confirm"),
        cancelButtonText: t("common.cancel"),
        type: "warning",
      },
    );
    return true;
  } catch {
    return false;
  }
}

async function switchConfigFile(cfId: number) {
  if (!(await confirmIfDirty())) return;
  router.push({
    name: ROUTE_NAMES.DRAFT_EDITOR,
    params: {
      projectId: route.params.projectId,
      deploymentId: route.params.deploymentId,
      configFileId: String(cfId),
    },
  });
}

async function loadDraftResources() {
  const did = deploymentId.value;
  const cid = configFileId.value;
  if (Number.isNaN(did) || Number.isNaN(cid)) return;

  resourceLoading.value = true;
  resourceError.value = null;
  deployment.value = null;
  configFile.value = null;
  draft.value = null;
  content.value = "";
  savedContent.value = "";
  draftWasMissing.value = false;

  try {
    const [deploymentResult, configResult, configListResult] =
      await Promise.all([
        deploymentInstancesApi.getDeploymentInstance(did),
        configFilesApi.getConfigFile(cid),
        configFilesApi.listConfigFiles({
          project_id: Number(route.params.projectId),
          status: "active",
        }),
      ]);
    deployment.value = deploymentResult;
    configFile.value = configResult;
    configFiles.value = configListResult.items;

    if (
      deploymentResult.project_id !== projectId.value ||
      configResult.project_id !== projectId.value
    ) {
      resourceError.value = new ApiRequestError(404, {
        code: "resource_not_found",
        message: "resource not found",
      });
      return;
    }

    if (!canEdit.value) {
      return;
    }

    // Load preview status for config switcher badges (non-blocking)
    loadPreviewStatus(did);

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
        message: t("drafts.page.loadError"),
      });
    }
  } finally {
    resourceLoading.value = false;
  }
}

async function loadPreviewStatus(did: number) {
  try {
    const preview = await deploymentInstancesApi.previewDeploymentBundle(did);
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
  } catch {
    // Non-critical; silently ignore
  }
}

async function loadAll() {
  const pid = projectId.value;
  if (Number.isNaN(pid)) return;
  await fetchProject(pid);
  await loadDraftResources();
}

function applyDraft(value: DraftResponse) {
  draft.value = value;
  draftWasMissing.value = false;
  content.value = value.content;
  savedContent.value = value.content;
}

async function handleSave() {
  if (!configFile.value) return;

  saving.value = true;
  try {
    const result = await draftsApi.updateDraft(
      deploymentId.value,
      configFileId.value,
      {
        content: content.value,
        format: configFile.value.format,
        base_version: draft.value?.version ?? 0,
      },
    );
    applyDraft(result);
    ElMessage.success(t("toast.drafts.saved"));
  } catch (err) {
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    saving.value = false;
  }
}

async function handleDiscard() {
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

  discarding.value = true;
  try {
    await draftsApi.deleteDraft(deploymentId.value, configFileId.value);
    draft.value = null;
    draftWasMissing.value = true;

    // Load fallback content so the editor shows what the instance uses now
    // instead of a misleading empty state.
    try {
      const preview = await deploymentInstancesApi.previewDeploymentBundle(
        deploymentId.value,
      );
      const item = preview.items.find(
        (i) => i.config_file_id === configFileId.value,
      );
      content.value = item?.content ?? "";
      savedContent.value = content.value;
    } catch {
      content.value = "";
      savedContent.value = "";
    }

    ElMessage.success(t("toast.drafts.discarded"));
    loadPreviewStatus(deploymentId.value);
  } catch (err) {
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    discarding.value = false;
  }
}

async function handleRestoreFromRelease() {
  try {
    await ElMessageBox.confirm(
      t("drafts.restore.prompt"),
      t("drafts.restore.title"),
      {
        confirmButtonText: t("drafts.restore.confirm"),
        cancelButtonText: t("common.cancel"),
        type: "warning",
      },
    );
  } catch {
    return;
  }

  restoring.value = true;
  try {
    const result = await draftsApi.cloneDraft(
      deploymentId.value,
      configFileId.value,
      {
        source_deployment_instance_id: deploymentId.value,
        source_kind: "latest_release",
      },
    );
    applyDraft(result.draft);
    ElMessage.success(t("toast.drafts.restoredFromRelease"));
    loadPreviewStatus(deploymentId.value);
  } catch (err) {
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    restoring.value = false;
  }
}

async function handlePublish() {
  if (!draft.value || isDirty.value) {
    ElMessage.warning(t("drafts.notice.saveBeforePublish"));
    return;
  }

  try {
    const { value } = await ElMessageBox.prompt(
      t("drafts.publish.prompt"),
      t("drafts.publish.title"),
      {
        inputType: "textarea",
        inputPlaceholder: t("drafts.publish.placeholder"),
        confirmButtonText: t("common.publish"),
        cancelButtonText: t("common.cancel"),
      },
    );
    publishing.value = true;
    const release = await releasesApi.publishRelease({
      project_id: projectId.value,
      deployment_instance_id: deploymentId.value,
      config_file_id: configFileId.value,
      change_summary: value || null,
    });
    ElMessage.success(
      t("toast.releases.published", { revision: release.revision }),
    );
    router.push({
      name: ROUTE_NAMES.RELEASE_LIST,
      params: { projectId: route.params.projectId },
      query: {
        deployment_instance_id: String(deploymentId.value),
        config_file_id: String(configFileId.value),
      },
    });
  } catch (err) {
    if (err === "cancel" || err === "close") return;
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    publishing.value = false;
  }
}

// Clone from other instance
async function loadCloneableInstances() {
  cloneInstancesLoading.value = true;
  try {
    const result = await deploymentInstancesApi.listDeploymentInstances({
      project_id: projectId.value,
      page_size: 200,
    });
    cloneableInstances.value = result.items.filter(
      (inst) => inst.id !== deploymentId.value,
    );
  } catch {
    cloneableInstances.value = [];
  } finally {
    cloneInstancesLoading.value = false;
  }
}

watch(cloneDialogVisible, (visible) => {
  if (visible) {
    cloneSourceInstanceId.value = null;
    cloneSourceKind.value = "draft";
    loadCloneableInstances();
  }
});

async function handleCloneFromInstance() {
  if (!cloneSourceInstanceId.value) return;

  if (isDirty.value) {
    try {
      await ElMessageBox.confirm(
        t("drafts.cloneDialog.overwritePrompt"),
        t("drafts.cloneDialog.overwriteTitle"),
        {
          confirmButtonText: t("drafts.cloneDialog.overwriteConfirm"),
          cancelButtonText: t("common.cancel"),
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
      deploymentId.value,
      configFileId.value,
      {
        source_deployment_instance_id: cloneSourceInstanceId.value,
        source_kind: cloneSourceKind.value,
      },
    );
    applyDraft(result.draft);
    cloneDialogVisible.value = false;
    ElMessage.success(t("toast.drafts.clonedFromInstance"));
    loadPreviewStatus(deploymentId.value);
  } catch (err) {
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    cloning.value = false;
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

function goToPreview() {
  router.push({
    name: ROUTE_NAMES.DEPLOYMENT_PREVIEW,
    params: {
      projectId: route.params.projectId,
      deploymentId: route.params.deploymentId,
    },
  });
}

onMounted(loadAll);

watch(
  () => [
    route.params.projectId,
    route.params.deploymentId,
    route.params.configFileId,
  ],
  () => loadAll(),
);

// ---- Global route-leave & browser-close guards ----

onBeforeRouteLeave(async () => {
  if (!isDirty.value) return true;
  try {
    await ElMessageBox.confirm(
      t("drafts.navigate.prompt"),
      t("drafts.navigate.title"),
      {
        confirmButtonText: t("drafts.navigate.confirm"),
        cancelButtonText: t("common.cancel"),
        type: "warning",
      },
    );
    return true;
  } catch {
    return false;
  }
});

function onBeforeUnloadHandler(e: {
  preventDefault: () => void;
  returnValue?: string;
}) {
  if (isDirty.value) {
    e.preventDefault();
    e.returnValue = "";
  }
}

onMounted(() => {
  globalThis.addEventListener("beforeunload", onBeforeUnloadHandler);
});
onBeforeUnmount(() => {
  globalThis.removeEventListener("beforeunload", onBeforeUnloadHandler);
});
</script>

<style scoped>
.draft-editor-page {
  width: 100%;
}

.draft-editor-page__section {
  margin-top: var(--spacing-md);
}

.draft-editor-page__header-actions,
.draft-editor-page__nav {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-sm);
  justify-content: flex-end;
}

.draft-editor-page__nav {
  justify-content: flex-start;
  margin-bottom: var(--spacing-md);
}

.draft-editor-page__config-switcher {
  margin-bottom: var(--spacing-md);
}

.draft-editor-page__config-switcher-label {
  font-size: 13px;
  color: var(--el-text-color-secondary);
  margin-bottom: var(--spacing-xs, 4px);
}

.draft-editor-page__config-status-tag {
  margin-left: 4px;
}

.draft-editor-page__draft-actions {
  display: flex;
  gap: var(--spacing-sm);
  margin-bottom: var(--spacing-md);
}

.draft-editor-page__meta,
.draft-editor-page__notice {
  margin-bottom: var(--spacing-md);
}

.draft-editor-page__code {
  font-family: monospace;
}

.draft-editor-page__editor :deep(textarea) {
  font-family:
    ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono",
    "Courier New", monospace;
  line-height: 1.55;
}
</style>
