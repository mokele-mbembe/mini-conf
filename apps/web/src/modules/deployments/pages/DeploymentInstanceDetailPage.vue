<template>
  <div class="deployment-instance-detail-page page-container">
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
      <PageHeader :title="detailTitle" :subtitle="detailSubtitle">
        <template #actions>
          <div
            v-if="deployment"
            class="deployment-instance-detail-page__header-actions"
          >
            <el-button
              v-if="canPreview"
              text
              type="primary"
              size="small"
              @click="openPreview"
            >
              {{ t("deployments.action.previewBundle") }}
            </el-button>
            <el-button
              v-if="canActivate"
              type="success"
              size="small"
              :loading="actionLoading === 'activate'"
              @click="handleActivate"
            >
              {{ t("deployments.action.activate") }}
            </el-button>
            <el-button
              v-if="canDeactivate"
              type="warning"
              size="small"
              :loading="actionLoading === 'deactivate'"
              @click="handleDeactivate"
            >
              {{ t("deployments.action.deactivate") }}
            </el-button>
            <el-button
              v-if="canResetToken"
              type="primary"
              size="small"
              :loading="actionLoading === 'reset-token'"
              @click="handleResetToken"
            >
              {{ t("deployments.action.resetToken") }}
            </el-button>
            <el-button
              v-if="canCloneFromTemplate"
              type="primary"
              size="small"
              @click="openCloneDialog"
            >
              {{ t("deployments.action.cloneFromTemplate") }}
            </el-button>
            <el-button
              v-if="canArchive"
              type="warning"
              size="small"
              :loading="actionLoading === 'archive'"
              @click="handleArchive"
            >
              {{ t("deployments.action.archive") }}
            </el-button>
            <el-button
              v-if="canRestore"
              type="primary"
              size="small"
              :loading="actionLoading === 'restore'"
              @click="handleRestore"
            >
              {{ t("deployments.action.restore") }}
            </el-button>
            <el-button
              v-if="canPermanentDelete"
              type="danger"
              size="small"
              :loading="actionLoading === 'delete'"
              @click="handleDelete"
            >
              {{ t("deployments.action.permanentDelete") }}
            </el-button>
            <StatusBadge :status="deployment.status" />
          </div>
        </template>
      </PageHeader>

      <ProjectTabs />

      <div class="deployment-instance-detail-page__section">
        <el-button text type="primary" @click="backToList">
          {{ t("deployments.detail.back") }}
        </el-button>

        <LoadingState v-if="detailLoading" />

        <NotFoundState
          v-else-if="isProjectMismatch || detailError?.status === 404"
          :title="t('deployments.notFound.title')"
          :subtitle="t('deployments.notFound.subtitle')"
        />

        <ForbiddenState
          v-else-if="detailError?.status === 403"
          :subtitle="t('project.forbidden.subtitle')"
        />

        <ErrorState
          v-else-if="detailError"
          :title="t('deployments.detail.loadError')"
          :subtitle="getErrorMessage(detailError.code, detailError.message)"
          @retry="loadDeploymentInstance"
        />

        <el-alert
          v-if="deployment?.is_archived"
          type="warning"
          :title="t('deployments.notice.archived')"
          :closable="false"
          show-icon
          style="margin-bottom: var(--spacing-md)"
        />

        <template v-if="deployment">
          <div class="deployment-instance-detail-page__summary">
            <div>
              <div class="deployment-instance-detail-page__eyebrow">
                {{ t("deployments.column.deploymentKey") }}
              </div>
              <div class="deployment-instance-detail-page__key">
                {{ deployment.deployment_key }}
              </div>
            </div>

            <div class="deployment-instance-detail-page__badges">
              <el-tag v-if="deployment.is_template" size="small" type="warning">
                {{ t("deployments.type.template") }}
              </el-tag>
              <el-tag v-else size="small" type="success">
                {{ t("deployments.type.instance") }}
              </el-tag>
              <el-tag v-if="deployment.is_archived" size="small" type="info">
                {{ t("deployments.badge.archived") }}
              </el-tag>
              <StatusBadge :status="deployment.status" />
            </div>
          </div>

          <el-descriptions :column="2" border>
            <el-descriptions-item :label="t('deployments.field.id')">
              {{ deployment.id }}
            </el-descriptions-item>

            <el-descriptions-item :label="t('deployments.field.environment')">
              <el-tag size="small" type="info">
                {{ deployment.environment_name }} ({{
                  deployment.environment_code
                }})
              </el-tag>
            </el-descriptions-item>

            <el-descriptions-item :label="t('deployments.field.deploymentKey')">
              <span class="deployment-instance-detail-page__code">
                {{ deployment.deployment_key }}
              </span>
            </el-descriptions-item>

            <el-descriptions-item :label="t('deployments.field.name')">
              {{ deployment.name }}
            </el-descriptions-item>

            <el-descriptions-item :label="t('deployments.field.type')">
              {{ deploymentTypeLabel }}
            </el-descriptions-item>

            <el-descriptions-item :label="t('deployments.field.status')">
              <StatusBadge :status="deployment.status" />
            </el-descriptions-item>

            <el-descriptions-item
              :label="t('deployments.field.templateSource')"
            >
              {{ templateSourceLabel }}
            </el-descriptions-item>

            <el-descriptions-item
              :label="t('deployments.field.description')"
              :span="2"
            >
              {{ deployment.description || t("deployments.emptyDescription") }}
            </el-descriptions-item>
          </el-descriptions>

          <div class="deployment-instance-detail-page__configs">
            <div class="deployment-instance-detail-page__configs-heading">
              <div>
                <h3>{{ t("deployments.configs.title") }}</h3>
                <p>{{ t("deployments.configs.subtitle") }}</p>
              </div>
              <el-button
                v-if="canPreview"
                text
                type="primary"
                @click="openPreview"
              >
                {{ t("deployments.action.previewBundle") }}
              </el-button>
            </div>

            <LoadingState v-if="configListLoading" />

            <ErrorState
              v-else-if="configListError"
              :title="t('configFiles.page.loadError')"
              :subtitle="
                getErrorMessage(configListError.code, configListError.message)
              "
              @retry="loadConfigFiles"
            />

            <EmptyState
              v-else-if="configFiles.length === 0"
              :description="t('deployments.configs.empty')"
            />

            <div v-else class="page-table-shell">
              <el-table :data="configFiles" stripe style="width: 100%">
                <el-table-column
                  prop="code"
                  :label="t('configFiles.column.code')"
                  min-width="150"
                >
                  <template #default="{ row }">
                    <span class="deployment-instance-detail-page__code">
                      {{ row.code }}
                    </span>
                  </template>
                </el-table-column>

                <el-table-column
                  prop="name"
                  :label="t('configFiles.column.name')"
                  min-width="160"
                />

                <el-table-column
                  prop="format"
                  :label="t('configFiles.column.format')"
                  width="90"
                >
                  <template #default="{ row }">
                    <el-tag size="small" type="info">{{ row.format }}</el-tag>
                  </template>
                </el-table-column>

                <el-table-column
                  :label="t('configFiles.column.required')"
                  width="90"
                  align="center"
                >
                  <template #default="{ row }">
                    <el-tag v-if="row.is_required" size="small" type="danger">
                      {{ t("configFiles.required") }}
                    </el-tag>
                    <span v-else class="deployment-instance-detail-page__muted">
                      —
                    </span>
                  </template>
                </el-table-column>

                <el-table-column
                  :label="t('deployments.column.actions')"
                  width="140"
                  align="center"
                  fixed="right"
                >
                  <template #default="{ row }">
                    <el-button
                      v-if="canEditDraft"
                      text
                      type="primary"
                      size="small"
                      @click="openDraft(row.id)"
                    >
                      {{ t("deployments.action.editDraft") }}
                    </el-button>
                    <span v-else class="deployment-instance-detail-page__muted">
                      —
                    </span>
                  </template>
                </el-table-column>
              </el-table>
            </div>
          </div>
        </template>
      </div>
    </template>

    <DeploymentTokenDialog
      v-model:visible="tokenDialogVisible"
      :payload="tokenPayload"
      :mode="tokenDialogMode"
    />

    <DeploymentInstanceCloneDialog
      v-model:visible="cloneDialogVisible"
      :project-id="projectId"
      :template="deployment"
      @success="handleCloneSuccess"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { useRoute, useRouter } from "vue-router";
import { useProjectContext } from "@/modules/projects/composables/useProjectContext";
import ProjectTabs from "@/modules/projects/components/ProjectTabs.vue";
import DeploymentInstanceCloneDialog from "../components/DeploymentInstanceCloneDialog.vue";
import DeploymentTokenDialog from "../components/DeploymentTokenDialog.vue";
import PageHeader from "@/shared/components/PageHeader.vue";
import StatusBadge from "@/shared/components/StatusBadge.vue";
import LoadingState from "@/shared/states/LoadingState.vue";
import EmptyState from "@/shared/states/EmptyState.vue";
import ErrorState from "@/shared/states/ErrorState.vue";
import ForbiddenState from "@/shared/states/ForbiddenState.vue";
import NotFoundState from "@/shared/states/NotFoundState.vue";
import { ROUTE_NAMES } from "@/shared/constants/routes";
import * as deploymentInstancesApi from "@/api/deployment-instances";
import * as configFilesApi from "@/api/config-files";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type { ConfigFileSummary } from "@/api/types/config-file";
import type {
  DeploymentInstanceSummary,
  DeploymentTokenResponse,
} from "@/api/types/deployment-instance";
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
const isAdmin = computed(() => project.value?.current_user_role === "admin");
const canEditDraft = computed(() => {
  const role = project.value?.current_user_role;
  return (
    (role === "admin" || role === "editor") &&
    deployment.value !== null &&
    !deployment.value.is_archived
  );
});

const deployment = ref<DeploymentInstanceSummary | null>(null);
const detailLoading = ref(false);
const detailError = ref<ApiRequestError | null>(null);
const configFiles = ref<ConfigFileSummary[]>([]);
const configListLoading = ref(false);
const configListError = ref<ApiRequestError | null>(null);
type DeploymentActionLoading =
  | "activate"
  | "deactivate"
  | "reset-token"
  | "archive"
  | "restore"
  | "delete";

const actionLoading = ref<DeploymentActionLoading | null>(null);
const tokenDialogVisible = ref(false);
const tokenDialogMode = ref<"activate" | "reset">("activate");
const tokenPayload = ref<DeploymentTokenResponse | null>(null);
const cloneDialogVisible = ref(false);
const detailTitle = computed(
  () => deployment.value?.name ?? t("deployments.detail.title"),
);
const detailSubtitle = computed(
  () => deployment.value?.description ?? undefined,
);
const deploymentTypeLabel = computed(() => {
  if (!deployment.value) {
    return "";
  }

  return deployment.value.is_template
    ? t("deployments.type.template")
    : t("deployments.type.instance");
});
const canActivate = computed(() => {
  if (!deployment.value || !isAdmin.value) {
    return false;
  }

  return (
    !deployment.value.is_template &&
    !deployment.value.is_archived &&
    deployment.value.status === "inactive"
  );
});
const canDeactivate = computed(() => {
  if (!deployment.value || !isAdmin.value) {
    return false;
  }

  return (
    !deployment.value.is_template &&
    !deployment.value.is_archived &&
    deployment.value.status === "active"
  );
});
const canResetToken = computed(() => canDeactivate.value);
const canPreview = computed(
  () =>
    canEditDraft.value &&
    deployment.value !== null &&
    !deployment.value.is_archived,
);
const canArchive = computed(() => {
  if (!deployment.value || !isAdmin.value) return false;
  return (
    !deployment.value.is_template &&
    !deployment.value.is_archived &&
    deployment.value.status === "inactive"
  );
});
const canRestore = computed(() => {
  if (!deployment.value || !isAdmin.value) return false;
  return !deployment.value.is_template && deployment.value.is_archived;
});
const canPermanentDelete = computed(() => {
  if (!deployment.value || !isAdmin.value) return false;
  return !deployment.value.is_template && deployment.value.is_archived;
});
const canCloneFromTemplate = computed(() => {
  if (!deployment.value || !isAdmin.value) {
    return false;
  }

  return deployment.value.is_template && !deployment.value.is_archived;
});
const templateSourceLabel = computed(() => {
  if (!deployment.value) {
    return "";
  }

  return deployment.value.template_source_id == null
    ? t("deployments.emptyTemplateSource")
    : String(deployment.value.template_source_id);
});
const isProjectMismatch = computed(
  () =>
    deployment.value !== null &&
    !Number.isNaN(projectId.value) &&
    deployment.value.project_id !== projectId.value,
);

async function loadDeploymentInstance() {
  const id = deploymentId.value;
  if (Number.isNaN(id)) return;

  detailLoading.value = true;
  detailError.value = null;
  deployment.value = null;
  try {
    deployment.value = await deploymentInstancesApi.getDeploymentInstance(id);
  } catch (err) {
    if (err instanceof ApiRequestError) {
      detailError.value = err;
    } else {
      detailError.value = new ApiRequestError(0, {
        code: "unknown_error",
        message: t("deployments.detail.loadError"),
      });
    }
  } finally {
    detailLoading.value = false;
  }
}

async function loadConfigFiles() {
  configListLoading.value = true;
  configListError.value = null;
  try {
    const res = await configFilesApi.listConfigFiles({
      project_id: projectId.value,
      status: "active",
    });
    configFiles.value = res.items;
  } catch (err) {
    if (err instanceof ApiRequestError) {
      configListError.value = err;
    } else {
      configListError.value = new ApiRequestError(0, {
        code: "unknown_error",
        message: t("configFiles.page.loadError"),
      });
    }
  } finally {
    configListLoading.value = false;
  }
}

async function loadAll() {
  const id = projectId.value;
  if (Number.isNaN(id)) return;
  await fetchProject(id);
  await Promise.all([loadDeploymentInstance(), loadConfigFiles()]);
}

function openTokenDialog(
  payload: DeploymentTokenResponse,
  mode: "activate" | "reset",
) {
  tokenPayload.value = payload;
  tokenDialogMode.value = mode;
  tokenDialogVisible.value = true;
}

async function handleActivate() {
  if (!deployment.value) return;

  try {
    await ElMessageBox.confirm(
      t("deployments.dialog.activateConfirm", { name: deployment.value.name }),
      t("deployments.dialog.activateTitle"),
      { type: "warning" },
    );
    actionLoading.value = "activate";
    const payload = await deploymentInstancesApi.activateDeploymentInstance(
      deployment.value.id,
    );
    ElMessage.success(t("toast.deployments.activated"));
    openTokenDialog(payload, "activate");
    await loadDeploymentInstance();
  } catch (err) {
    if (err === "cancel" || err === "close") return;
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    actionLoading.value = null;
  }
}

async function handleDeactivate() {
  if (!deployment.value) return;

  try {
    await ElMessageBox.confirm(
      t("deployments.dialog.deactivateConfirm", {
        name: deployment.value.name,
      }),
      t("deployments.dialog.deactivateTitle"),
      { type: "warning" },
    );
    actionLoading.value = "deactivate";
    await deploymentInstancesApi.deactivateDeploymentInstance(
      deployment.value.id,
    );
    ElMessage.success(t("toast.deployments.deactivated"));
    await loadDeploymentInstance();
  } catch (err) {
    if (err === "cancel" || err === "close") return;
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    actionLoading.value = null;
  }
}

async function handleResetToken() {
  if (!deployment.value) return;

  try {
    await ElMessageBox.confirm(
      t("deployments.dialog.resetTokenConfirm", {
        name: deployment.value.name,
      }),
      t("deployments.dialog.resetTokenTitle"),
      { type: "warning" },
    );
    actionLoading.value = "reset-token";
    const payload = await deploymentInstancesApi.resetDeploymentToken(
      deployment.value.id,
    );
    ElMessage.success(t("toast.deployments.tokenReset"));
    openTokenDialog(payload, "reset");
    await loadDeploymentInstance();
  } catch (err) {
    if (err === "cancel" || err === "close") return;
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    actionLoading.value = null;
  }
}

function backToList() {
  router.push({
    name: ROUTE_NAMES.DEPLOYMENT_LIST,
    params: { projectId: route.params.projectId },
  });
}

async function handleArchive() {
  if (!deployment.value) return;

  try {
    await ElMessageBox.confirm(
      t("deployments.dialog.archiveConfirm", { name: deployment.value.name }),
      t("deployments.dialog.archiveTitle"),
      { type: "warning" },
    );
    actionLoading.value = "archive";
    await deploymentInstancesApi.archiveDeploymentInstance(deployment.value.id);
    ElMessage.success(t("toast.deployments.archived"));
    await loadDeploymentInstance();
  } catch (err) {
    if (err === "cancel" || err === "close") return;
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    actionLoading.value = null;
  }
}

async function handleRestore() {
  if (!deployment.value) return;

  try {
    await ElMessageBox.confirm(
      t("deployments.dialog.restoreConfirm", { name: deployment.value.name }),
      t("deployments.dialog.restoreTitle"),
      { type: "warning" },
    );
    actionLoading.value = "restore";
    await deploymentInstancesApi.restoreDeploymentInstance(deployment.value.id);
    ElMessage.success(t("toast.deployments.restored"));
    await loadDeploymentInstance();
  } catch (err) {
    if (err === "cancel" || err === "close") return;
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    actionLoading.value = null;
  }
}

async function handleDelete() {
  if (!deployment.value) return;
  const deploymentKey = deployment.value.deployment_key;
  const deploymentId = deployment.value.id;

  try {
    await ElMessageBox.prompt(
      t("deployments.dialog.deletePrompt", { key: deploymentKey }),
      t("deployments.dialog.deleteTitle"),
      {
        type: "error",
        inputPlaceholder: t("deployments.dialog.deleteInputPlaceholder"),
        inputPattern: new RegExp(
          `^${deploymentKey.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`,
        ),
        inputErrorMessage: t("deployments.dialog.deleteKeyMismatch"),
        confirmButtonText: t("deployments.action.permanentDelete"),
        cancelButtonText: t("common.cancel"),
      },
    );
    actionLoading.value = "delete";
    await deploymentInstancesApi.deleteDeploymentInstance(deploymentId);
    ElMessage.success(t("toast.deployments.deleted"));
    backToList();
  } catch (err) {
    if (err === "cancel" || err === "close") return;
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    actionLoading.value = null;
  }
}

function openCloneDialog() {
  cloneDialogVisible.value = true;
}

function openPreview() {
  router.push({
    name: ROUTE_NAMES.DEPLOYMENT_PREVIEW,
    params: {
      projectId: route.params.projectId,
      deploymentId: route.params.deploymentId,
    },
  });
}

function openDraft(configFileId: number) {
  router.push({
    name: ROUTE_NAMES.DRAFT_EDITOR,
    params: {
      projectId: route.params.projectId,
      deploymentId: route.params.deploymentId,
      configFileId,
    },
  });
}

function handleCloneSuccess(item: DeploymentInstanceSummary) {
  router.push({
    name: ROUTE_NAMES.DEPLOYMENT_DETAIL,
    params: {
      projectId: route.params.projectId,
      deploymentId: item.id,
    },
  });
}

onMounted(loadAll);

watch(
  () => [route.params.projectId, route.params.deploymentId],
  () => loadAll(),
);
</script>

<style scoped>
.deployment-instance-detail-page {
  width: 100%;
}

.deployment-instance-detail-page__section {
  margin-top: var(--spacing-md);
}

.deployment-instance-detail-page__summary {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--spacing-md);
  margin: var(--spacing-md) 0;
  padding-bottom: var(--spacing-md);
  border-bottom: 1px solid var(--color-border-light);
}

.deployment-instance-detail-page__eyebrow {
  margin-bottom: 4px;
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.deployment-instance-detail-page__key,
.deployment-instance-detail-page__code {
  font-family: monospace;
}

.deployment-instance-detail-page__key {
  font-size: var(--font-size-lg);
  font-weight: 600;
}

.deployment-instance-detail-page__badges {
  display: flex;
  flex-wrap: wrap;
  gap: var(--spacing-sm);
  justify-content: flex-end;
}

.deployment-instance-detail-page__header-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: flex-end;
  gap: var(--spacing-sm);
}

.deployment-instance-detail-page__configs {
  margin-top: var(--spacing-lg);
}

.deployment-instance-detail-page__configs-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--spacing-md);
  margin-bottom: var(--spacing-md);
}

.deployment-instance-detail-page__configs-heading h3 {
  font-size: var(--font-size-lg);
  font-weight: 600;
  margin-bottom: 4px;
}

.deployment-instance-detail-page__configs-heading p,
.deployment-instance-detail-page__muted {
  color: var(--color-text-secondary);
}

@media (max-width: 768px) {
  .deployment-instance-detail-page__summary {
    flex-direction: column;
  }

  .deployment-instance-detail-page__header-actions {
    justify-content: flex-start;
  }

  .deployment-instance-detail-page__configs-heading {
    flex-direction: column;
  }
}
</style>
