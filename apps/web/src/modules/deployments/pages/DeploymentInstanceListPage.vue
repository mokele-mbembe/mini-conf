<template>
  <div class="deployment-instance-list-page page-container">
    <NotFoundState
      v-if="projectError && projectError.status === 404"
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

    <template v-else>
      <template v-if="project">
        <PageHeader
          :title="project.name"
          :subtitle="project.description ?? undefined"
        >
          <template #actions>
            <StatusBadge :status="project.status" />
          </template>
        </PageHeader>
      </template>
      <template v-else-if="projectLoading">
        <div class="deployment-instance-list-page__header-skeleton">
          <el-skeleton :rows="1" animated />
        </div>
      </template>

      <ProjectTabs />

      <!-- Top-level toolbar: create button + environment notice -->
      <div class="deployment-instance-list-page__top-toolbar">
        <el-button
          v-if="isAdmin"
          type="primary"
          :disabled="activeEnvironmentCount === 0"
          @click="openCreateDialog"
        >
          {{ t("deployments.create") }}
        </el-button>
      </div>

      <el-alert
        v-if="!environmentsLoading && activeEnvironmentCount === 0"
        :title="t('deployments.emptyEnvironmentNotice')"
        type="warning"
        :closable="false"
        show-icon
        style="margin-bottom: 16px"
      >
        <template #default>
          <el-button
            v-if="isAdmin"
            link
            type="primary"
            @click="goToEnvironmentPage"
          >
            {{ t("deployments.goToEnvironmentManagement") }}
          </el-button>
        </template>
      </el-alert>

      <!-- ==================== Template Section ==================== -->
      <div class="deployment-instance-list-page__section">
        <h3 class="deployment-instance-list-page__section-title">
          {{ t("deployments.section.templates") }}
        </h3>
        <p class="deployment-instance-list-page__section-desc">
          {{ t("deployments.section.templatesDesc") }}
        </p>

        <div class="deployment-instance-list-page__toolbar">
          <div class="deployment-instance-list-page__filters">
            <el-input
              v-model="templateList.keyword.value"
              :placeholder="t('deployments.filter.keywordPlaceholder')"
              clearable
              style="width: 220px"
              @keyup.enter="templateList.search"
              @clear="templateList.search"
            />

            <el-select
              v-model="templateList.environmentId.value"
              :placeholder="t('deployments.filter.environmentPlaceholder')"
              clearable
              :loading="environmentsLoading"
              style="width: 220px"
              @change="templateList.filterChange"
            >
              <el-option
                :label="t('deployments.filter.allEnvironments')"
                value=""
              />
              <el-option
                v-for="item in environments"
                :key="item.id"
                :label="`${item.name} (${item.code})`"
                :value="String(item.id)"
              />
            </el-select>

            <el-button @click="templateList.search">
              {{ t("deployments.filter.search") }}
            </el-button>

            <el-button text @click="templateList.resetFilters">
              {{ t("deployments.filter.reset") }}
            </el-button>
          </div>
        </div>

        <ErrorState
          v-if="templateList.error.value"
          :title="t('deployments.page.loadError')"
          :subtitle="
            getErrorMessage(
              templateList.error.value.code,
              templateList.error.value.message,
            )
          "
          @retry="templateList.load"
        />

        <EmptyState
          v-else-if="
            !templateList.loading.value && templateList.items.value.length === 0
          "
          :description="
            environments.length === 0
              ? t('deployments.emptyNeedEnvironment')
              : t('deployments.section.templatesEmpty')
          "
        >
          <el-button
            v-if="isAdmin && environments.length === 0"
            type="primary"
            @click="goToEnvironmentPage"
          >
            {{ t("deployments.goToEnvironmentManagement") }}
          </el-button>
        </EmptyState>

        <template v-else>
          <div class="deployment-instance-list-page__table page-table-shell">
            <el-table
              v-loading="templateList.loading.value"
              :data="templateList.items.value"
              stripe
              style="width: 100%"
            >
              <el-table-column
                prop="environment_code"
                :label="t('deployments.column.environment')"
                min-width="180"
              >
                <template #default="{ row }">
                  <el-tag size="small" type="info">
                    {{ row.environment_name }} ({{ row.environment_code }})
                  </el-tag>
                </template>
              </el-table-column>

              <el-table-column
                prop="deployment_key"
                :label="t('deployments.column.deploymentKey')"
                min-width="180"
              >
                <template #default="{ row }">
                  <span class="deployment-instance-list-page__code">
                    {{ row.deployment_key }}
                  </span>
                </template>
              </el-table-column>

              <el-table-column
                prop="name"
                :label="t('deployments.column.name')"
                min-width="170"
              />

              <el-table-column
                :label="t('deployments.column.description')"
                min-width="180"
                show-overflow-tooltip
              >
                <template #default="{ row }">
                  {{ row.description || t("deployments.emptyDescription") }}
                </template>
              </el-table-column>

              <el-table-column
                :label="t('deployments.column.actions')"
                width="220"
                align="center"
                fixed="right"
              >
                <template #default="{ row }">
                  <div class="deployment-instance-list-page__actions">
                    <el-button
                      text
                      type="primary"
                      size="small"
                      @click="openDetail(row)"
                    >
                      {{ t("deployments.action.view") }}
                    </el-button>
                    <el-button
                      v-if="isAdmin"
                      text
                      type="primary"
                      size="small"
                      :disabled="activeEnvironmentCount === 0"
                      @click="openCloneDialog(row)"
                    >
                      {{ t("deployments.action.cloneFromTemplate") }}
                    </el-button>
                  </div>
                </template>
              </el-table-column>
            </el-table>
          </div>

          <div class="deployment-instance-list-page__pagination">
            <el-pagination
              v-model:current-page="templateList.page.value"
              v-model:page-size="templateList.pageSize.value"
              :page-sizes="[10, 20, 50, 100]"
              :total="templateList.total.value"
              layout="total, sizes, prev, pager, next"
              background
              @current-change="templateList.load"
              @size-change="templateList.pageSizeChange"
            />
          </div>
        </template>
      </div>

      <!-- ==================== Instance Section ==================== -->
      <div class="deployment-instance-list-page__section">
        <h3 class="deployment-instance-list-page__section-title">
          {{ t("deployments.section.instances") }}
        </h3>
        <p class="deployment-instance-list-page__section-desc">
          {{ t("deployments.section.instancesDesc") }}
        </p>

        <div class="deployment-instance-list-page__toolbar">
          <div class="deployment-instance-list-page__filters">
            <el-input
              v-model="instanceList.keyword.value"
              :placeholder="t('deployments.filter.keywordPlaceholder')"
              clearable
              style="width: 220px"
              @keyup.enter="instanceList.search"
              @clear="instanceList.search"
            />

            <el-select
              v-model="instanceList.environmentId.value"
              :placeholder="t('deployments.filter.environmentPlaceholder')"
              clearable
              :loading="environmentsLoading"
              style="width: 220px"
              @change="instanceList.filterChange"
            >
              <el-option
                :label="t('deployments.filter.allEnvironments')"
                value=""
              />
              <el-option
                v-for="item in environments"
                :key="item.id"
                :label="`${item.name} (${item.code})`"
                :value="String(item.id)"
              />
            </el-select>

            <el-select
              v-model="instanceList.status.value"
              :placeholder="t('deployments.filter.allStatuses')"
              clearable
              style="width: 140px"
              @change="instanceList.filterChange"
            >
              <el-option :label="t('deployments.filter.all')" value="" />
              <el-option :label="t('status.active')" value="active" />
              <el-option :label="t('status.inactive')" value="inactive" />
            </el-select>

            <el-button @click="instanceList.search">
              {{ t("deployments.filter.search") }}
            </el-button>

            <el-button text @click="instanceList.resetFilters">
              {{ t("deployments.filter.reset") }}
            </el-button>
          </div>

          <el-button
            v-if="isAdmin"
            text
            type="info"
            @click="archivedDrawerVisible = true"
          >
            {{ t("deployments.action.viewArchived") }}
          </el-button>
        </div>

        <ErrorState
          v-if="instanceList.error.value"
          :title="t('deployments.page.loadError')"
          :subtitle="
            getErrorMessage(
              instanceList.error.value.code,
              instanceList.error.value.message,
            )
          "
          @retry="instanceList.load"
        />

        <EmptyState
          v-else-if="
            !instanceList.loading.value && instanceList.items.value.length === 0
          "
          :description="
            environments.length === 0
              ? t('deployments.emptyNeedEnvironment')
              : t('deployments.section.instancesEmpty')
          "
        >
          <el-button
            v-if="isAdmin && environments.length === 0"
            type="primary"
            @click="goToEnvironmentPage"
          >
            {{ t("deployments.goToEnvironmentManagement") }}
          </el-button>
          <el-button
            v-else-if="isAdmin"
            type="primary"
            :disabled="activeEnvironmentCount === 0"
            @click="openCreateDialog"
          >
            {{ t("deployments.create") }}
          </el-button>
        </EmptyState>

        <template v-else>
          <div class="deployment-instance-list-page__table page-table-shell">
            <el-table
              v-loading="instanceList.loading.value"
              :data="instanceList.items.value"
              stripe
              style="width: 100%"
            >
              <el-table-column
                prop="environment_code"
                :label="t('deployments.column.environment')"
                min-width="180"
              >
                <template #default="{ row }">
                  <el-tag size="small" type="info">
                    {{ row.environment_name }} ({{ row.environment_code }})
                  </el-tag>
                </template>
              </el-table-column>

              <el-table-column
                prop="deployment_key"
                :label="t('deployments.column.deploymentKey')"
                min-width="180"
              >
                <template #default="{ row }">
                  <span class="deployment-instance-list-page__code">
                    {{ row.deployment_key }}
                  </span>
                </template>
              </el-table-column>

              <el-table-column
                prop="name"
                :label="t('deployments.column.name')"
                min-width="170"
              />

              <el-table-column
                :label="t('deployments.column.status')"
                width="110"
                align="center"
              >
                <template #default="{ row }">
                  <StatusBadge :status="row.status" />
                </template>
              </el-table-column>

              <el-table-column
                :label="t('deployments.column.description')"
                min-width="180"
                show-overflow-tooltip
              >
                <template #default="{ row }">
                  {{ row.description || t("deployments.emptyDescription") }}
                </template>
              </el-table-column>

              <el-table-column
                :label="t('deployments.column.actions')"
                width="330"
                align="center"
                fixed="right"
              >
                <template #default="{ row }">
                  <div class="deployment-instance-list-page__actions">
                    <el-button
                      text
                      type="primary"
                      size="small"
                      @click="openDetail(row)"
                    >
                      {{ t("deployments.action.view") }}
                    </el-button>
                    <el-button
                      v-if="isAdmin && row.status === 'inactive'"
                      text
                      type="success"
                      size="small"
                      :loading="isActionLoading(row.id, 'activate')"
                      @click="handleActivate(row)"
                    >
                      {{ t("deployments.action.activate") }}
                    </el-button>
                    <el-button
                      v-if="isAdmin && row.status === 'active'"
                      text
                      type="warning"
                      size="small"
                      :loading="isActionLoading(row.id, 'deactivate')"
                      @click="handleDeactivate(row)"
                    >
                      {{ t("deployments.action.deactivate") }}
                    </el-button>
                    <el-button
                      v-if="isAdmin && row.status === 'active'"
                      text
                      type="primary"
                      size="small"
                      :loading="isActionLoading(row.id, 'reset-token')"
                      @click="handleResetToken(row)"
                    >
                      {{ t("deployments.action.resetToken") }}
                    </el-button>
                    <el-button
                      v-if="canArchiveRow(row)"
                      text
                      type="warning"
                      size="small"
                      :loading="isActionLoading(row.id, 'archive')"
                      @click="handleArchive(row)"
                    >
                      {{ t("deployments.action.archive") }}
                    </el-button>
                  </div>
                </template>
              </el-table-column>
            </el-table>
          </div>

          <div class="deployment-instance-list-page__pagination">
            <el-pagination
              v-model:current-page="instanceList.page.value"
              v-model:page-size="instanceList.pageSize.value"
              :page-sizes="[10, 20, 50, 100]"
              :total="instanceList.total.value"
              layout="total, sizes, prev, pager, next"
              background
              @current-change="instanceList.load"
              @size-change="instanceList.pageSizeChange"
            />
          </div>
        </template>
      </div>
    </template>

    <DeploymentInstanceCreateDialog
      v-model:visible="dialogVisible"
      :project-id="projectId"
      @success="handleCreateSuccess"
    />

    <DeploymentInstanceCloneDialog
      v-model:visible="cloneDialogVisible"
      :project-id="projectId"
      :template="cloneTarget"
      @success="handleCloneSuccess"
    />

    <DeploymentTokenDialog
      v-model:visible="tokenDialogVisible"
      :payload="tokenPayload"
      :mode="tokenDialogMode"
    />

    <ArchivedInstancesDrawer
      v-model:visible="archivedDrawerVisible"
      :project-id="projectId"
      @restored="handleArchivedDrawerRestored"
      @deleted="instanceList.load"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { useRoute, useRouter } from "vue-router";
import { useProjectContext } from "@/modules/projects/composables/useProjectContext";
import { useProjectEnvironments } from "@/modules/project-environments/composables/useProjectEnvironments";
import { useDeploymentInstanceList } from "../composables/useDeploymentInstanceList";
import ProjectTabs from "@/modules/projects/components/ProjectTabs.vue";
import DeploymentInstanceCreateDialog from "../components/DeploymentInstanceCreateDialog.vue";
import DeploymentInstanceCloneDialog from "../components/DeploymentInstanceCloneDialog.vue";
import DeploymentTokenDialog from "../components/DeploymentTokenDialog.vue";
import ArchivedInstancesDrawer from "../components/ArchivedInstancesDrawer.vue";
import PageHeader from "@/shared/components/PageHeader.vue";
import StatusBadge from "@/shared/components/StatusBadge.vue";
import EmptyState from "@/shared/states/EmptyState.vue";
import ErrorState from "@/shared/states/ErrorState.vue";
import ForbiddenState from "@/shared/states/ForbiddenState.vue";
import NotFoundState from "@/shared/states/NotFoundState.vue";
import { ROUTE_NAMES } from "@/shared/constants/routes";
import * as deploymentInstancesApi from "@/api/deployment-instances";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
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
const isAdmin = computed(() => project.value?.current_user_role === "admin");

const {
  environments,
  loading: environmentsLoading,
  load: loadEnvironments,
} = useProjectEnvironments(() => projectId.value);

const activeEnvironmentCount = computed(
  () => environments.value.filter((item) => item.status === "active").length,
);

const templateList = useDeploymentInstanceList({
  getProjectId: () => projectId.value,
  isTemplate: true,
  withStatusFilter: false,
});

const instanceList = useDeploymentInstanceList({
  getProjectId: () => projectId.value,
  isTemplate: false,
  withStatusFilter: true,
});

const dialogVisible = ref(false);
const cloneDialogVisible = ref(false);
const cloneTarget = ref<DeploymentInstanceSummary | null>(null);
const tokenDialogVisible = ref(false);
const tokenDialogMode = ref<"activate" | "reset">("activate");
const tokenPayload = ref<DeploymentTokenResponse | null>(null);
const actionTarget = ref<{ id: number; action: string } | null>(null);

async function loadAll() {
  const id = projectId.value;
  if (Number.isNaN(id)) return;
  await Promise.all([
    fetchProject(id),
    loadEnvironments(),
    templateList.load(),
    instanceList.load(),
  ]);
}

function openCreateDialog() {
  if (activeEnvironmentCount.value === 0) {
    return;
  }
  dialogVisible.value = true;
}

function openCloneDialog(row: DeploymentInstanceSummary) {
  cloneTarget.value = row;
  cloneDialogVisible.value = true;
}

function isActionLoading(id: number, action: string) {
  return actionTarget.value?.id === id && actionTarget.value.action === action;
}

function canArchiveRow(row: DeploymentInstanceSummary) {
  return (
    isAdmin.value &&
    !row.is_template &&
    !row.is_archived &&
    row.status === "inactive"
  );
}

function openTokenDialog(
  payload: DeploymentTokenResponse,
  mode: "activate" | "reset",
) {
  tokenPayload.value = payload;
  tokenDialogMode.value = mode;
  tokenDialogVisible.value = true;
}

function handleCreateSuccess(item?: DeploymentInstanceSummary) {
  if (item && item.is_template) {
    templateList.page.value = 1;
    templateList.load();
  } else {
    instanceList.status.value = "";
    instanceList.page.value = 1;
    instanceList.load();
  }
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

async function handleActivate(row: DeploymentInstanceSummary) {
  try {
    await ElMessageBox.confirm(
      t("deployments.dialog.activateConfirm", { name: row.name }),
      t("deployments.dialog.activateTitle"),
      { type: "warning" },
    );
    actionTarget.value = { id: row.id, action: "activate" };
    const payload = await deploymentInstancesApi.activateDeploymentInstance(
      row.id,
    );
    ElMessage.success(t("toast.deployments.activated"));
    openTokenDialog(payload, "activate");
    await instanceList.load();
  } catch (err) {
    if (err === "cancel" || err === "close") return;
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    actionTarget.value = null;
  }
}

async function handleDeactivate(row: DeploymentInstanceSummary) {
  try {
    await ElMessageBox.confirm(
      t("deployments.dialog.deactivateConfirm", { name: row.name }),
      t("deployments.dialog.deactivateTitle"),
      { type: "warning" },
    );
    actionTarget.value = { id: row.id, action: "deactivate" };
    await deploymentInstancesApi.deactivateDeploymentInstance(row.id);
    ElMessage.success(t("toast.deployments.deactivated"));
    await instanceList.load();
  } catch (err) {
    if (err === "cancel" || err === "close") return;
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    actionTarget.value = null;
  }
}

async function handleResetToken(row: DeploymentInstanceSummary) {
  try {
    await ElMessageBox.confirm(
      t("deployments.dialog.resetTokenConfirm", { name: row.name }),
      t("deployments.dialog.resetTokenTitle"),
      { type: "warning" },
    );
    actionTarget.value = { id: row.id, action: "reset-token" };
    const payload = await deploymentInstancesApi.resetDeploymentToken(row.id);
    ElMessage.success(t("toast.deployments.tokenReset"));
    openTokenDialog(payload, "reset");
    await instanceList.load();
  } catch (err) {
    if (err === "cancel" || err === "close") return;
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    actionTarget.value = null;
  }
}

const archivedDrawerVisible = ref(false);

async function handleArchive(row: DeploymentInstanceSummary) {
  try {
    await ElMessageBox.confirm(
      t("deployments.dialog.archiveConfirm", { name: row.name }),
      t("deployments.dialog.archiveTitle"),
      { type: "warning" },
    );
    actionTarget.value = { id: row.id, action: "archive" };
    await deploymentInstancesApi.archiveDeploymentInstance(row.id);
    ElMessage.success(t("toast.deployments.archived"));
    await instanceList.load();
  } catch (err) {
    if (err === "cancel" || err === "close") return;
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
    }
  } finally {
    actionTarget.value = null;
  }
}

function handleArchivedDrawerRestored() {
  instanceList.page.value = 1;
  instanceList.load();
}

function goToEnvironmentPage() {
  router.push({
    name: ROUTE_NAMES.PROJECT_ENVIRONMENT_LIST,
    params: { projectId: route.params.projectId },
  });
}

function openDetail(row: DeploymentInstanceSummary) {
  router.push({
    name: ROUTE_NAMES.DEPLOYMENT_DETAIL,
    params: {
      projectId: route.params.projectId,
      deploymentId: row.id,
    },
  });
}

onMounted(loadAll);

watch(
  () => route.params.projectId,
  () => loadAll(),
);
</script>

<style scoped>
.deployment-instance-list-page {
  width: 100%;
}

.deployment-instance-list-page__header-skeleton {
  margin-bottom: var(--spacing-md);
}

.deployment-instance-list-page__top-toolbar {
  display: flex;
  justify-content: flex-end;
  margin-top: var(--spacing-md);
  margin-bottom: var(--spacing-md);
}

.deployment-instance-list-page__section {
  margin-top: var(--spacing-lg);
}

.deployment-instance-list-page__section-title {
  margin: 0 0 4px 0;
  font-size: 1.1em;
  font-weight: 600;
}

.deployment-instance-list-page__section-desc {
  margin: 0 0 var(--spacing-md) 0;
  font-size: 0.9em;
  color: var(--el-text-color-secondary);
}

.deployment-instance-list-page__toolbar {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--spacing-md);
  margin-bottom: var(--spacing-md);
}

.deployment-instance-list-page__filters {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--spacing-sm);
}

.deployment-instance-list-page__code {
  font-family: monospace;
  font-size: 0.9em;
}

.deployment-instance-list-page__pagination {
  display: flex;
  justify-content: flex-end;
  margin-top: var(--spacing-md);
}

.deployment-instance-list-page__actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 4px 8px;
}

@media (max-width: 768px) {
  .deployment-instance-list-page__toolbar {
    flex-direction: column;
  }

  .deployment-instance-list-page__filters {
    width: 100%;
  }

  .deployment-instance-list-page__actions {
    justify-content: flex-start;
  }
}
</style>
