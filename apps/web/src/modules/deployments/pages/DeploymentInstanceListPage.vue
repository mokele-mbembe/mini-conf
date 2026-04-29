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
              row-key="id"
              stripe
              style="width: 100%"
              @expand-change="handleInstanceExpandChange"
            >
              <el-table-column type="expand" width="44">
                <template #default="{ row }">
                  <div class="deployment-instance-list-page__expanded">
                    <div class="deployment-instance-list-page__expanded-meta">
                      <span
                        class="deployment-instance-list-page__expanded-label"
                      >
                        {{ t("deployments.expanded.meta") }}
                      </span>
                      <span class="deployment-instance-list-page__code">
                        {{ row.deployment_key }}
                      </span>
                      <span>{{ row.name }}</span>
                      <el-tag size="small" type="info">
                        {{ row.environment_name }} ({{ row.environment_code }})
                      </el-tag>
                      <StatusBadge :status="row.status" />
                      <span
                        class="deployment-instance-list-page__expanded-pair"
                      >
                        {{ t("deployments.expanded.token") }}
                        <el-tag
                          size="small"
                          :type="row.status === 'active' ? 'success' : 'info'"
                        >
                          {{ tokenStatusLabel(row) }}
                        </el-tag>
                      </span>
                      <span
                        class="deployment-instance-list-page__expanded-pair"
                      >
                        {{ t("deployments.expanded.updatedAt") }}
                        <span>{{ deploymentUpdatedAt(row) }}</span>
                      </span>
                    </div>

                    <LoadingState
                      v-if="expandedDetail(row.id)?.loading"
                      class="deployment-instance-list-page__expanded-state"
                    />

                    <ErrorState
                      v-else-if="expandedDetail(row.id)?.error"
                      :title="t('deployments.expanded.configLoadError')"
                      :subtitle="expandedDetailErrorMessage(row.id)"
                      @retry="loadExpandedDeploymentDetail(row, true)"
                    />

                    <EmptyState
                      v-else-if="isExpandedDetailEmpty(row.id)"
                      :description="t('deployments.configs.empty')"
                    />

                    <div
                      v-else-if="expandedDetail(row.id)"
                      class="deployment-instance-list-page__expanded-configs"
                    >
                      <div
                        class="deployment-instance-list-page__expanded-label"
                      >
                        {{ t("deployments.expanded.configs") }}
                      </div>
                      <div
                        v-for="configFile in expandedDetail(row.id)
                          ?.configFiles"
                        :key="configFile.id"
                        class="deployment-instance-list-page__config-row"
                      >
                        <div class="deployment-instance-list-page__config-main">
                          <span class="deployment-instance-list-page__code">
                            {{ configFile.code }}
                          </span>
                          <span>{{ configFile.name }}</span>
                          <el-tag size="small" type="info">
                            {{ configFile.format }}
                          </el-tag>
                          <el-tag
                            v-if="configFile.is_required"
                            size="small"
                            type="danger"
                          >
                            {{ t("configFiles.required") }}
                          </el-tag>
                        </div>

                        <div
                          class="deployment-instance-list-page__config-hints"
                        >
                          <el-tag
                            size="small"
                            :type="configStateTagType(row.id, configFile.id)"
                          >
                            {{ configStateLabel(row.id, configFile.id) }}
                          </el-tag>
                          <el-tag
                            size="small"
                            :type="
                              configReadinessTagType(row.id, configFile.id)
                            "
                          >
                            {{ configReadinessLabel(row.id, configFile.id) }}
                          </el-tag>
                          <span class="deployment-instance-list-page__muted">
                            {{
                              configSavedVersionsLabel(row.id, configFile.id)
                            }}
                          </span>
                          <span class="deployment-instance-list-page__muted">
                            {{
                              configLatestReleaseLabel(row.id, configFile.id)
                            }}
                          </span>
                        </div>

                        <el-button
                          v-if="canOpenWorkspace(row)"
                          text
                          type="primary"
                          size="small"
                          @click="openDraftOverlay(row, configFile.id)"
                        >
                          {{ t("deployments.expanded.openWorkspace") }}
                        </el-button>
                        <span
                          v-else
                          class="deployment-instance-list-page__muted"
                        >
                          {{ t("common.notAvailable") }}
                        </span>
                      </div>
                    </div>
                  </div>
                </template>
              </el-table-column>

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

    <DraftEditorOverlay
      v-if="draftOverlayVisible"
      :visible="draftOverlayVisible"
      :deployment-id="draftOverlayDeploymentId"
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
import LoadingState from "@/shared/states/LoadingState.vue";
import NotFoundState from "@/shared/states/NotFoundState.vue";
import { ROUTE_NAMES } from "@/shared/constants/routes";
import * as deploymentInstancesApi from "@/api/deployment-instances";
import * as configFilesApi from "@/api/config-files";
import * as releasesApi from "@/api/releases";
import * as savedVersionsApi from "@/api/saved-versions";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type { ConfigFileSummary } from "@/api/types/config-file";
import type {
  DeploymentPreviewItem,
  DeploymentInstanceSummary,
  DeploymentTokenResponse,
} from "@/api/types/deployment-instance";
import { useI18nText } from "@/shared/i18n";

interface ConfigHistoryHint {
  savedVersionsCount: number;
  latestReleaseRevision: string | null;
}

interface ExpandedDeploymentDetail {
  loading: boolean;
  error: ApiRequestError | null;
  configFiles: ConfigFileSummary[];
  previewStatusMap: Record<number, DeploymentPreviewItem>;
  configHistoryMap: Record<number, ConfigHistoryHint>;
}

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
const expandedDetails = ref<Record<number, ExpandedDeploymentDetail>>({});
const draftOverlayDeploymentId = ref<number | null>(null);
const draftOverlayConfigFileId = ref<number | null>(null);
const draftOverlayVisible = computed(
  () =>
    draftOverlayDeploymentId.value !== null &&
    draftOverlayConfigFileId.value !== null,
);

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

function canOpenWorkspace(row: DeploymentInstanceSummary) {
  const role = project.value?.current_user_role;
  return (
    (role === "admin" || role === "editor") &&
    !row.is_template &&
    !row.is_archived
  );
}

function expandedDetail(id: number) {
  return expandedDetails.value[id];
}

function isExpandedDetailEmpty(id: number) {
  const detail = expandedDetails.value[id];
  return detail !== undefined && detail.configFiles.length === 0;
}

function setExpandedDetail(id: number, detail: ExpandedDeploymentDetail) {
  expandedDetails.value = {
    ...expandedDetails.value,
    [id]: detail,
  };
}

function emptyExpandedDetail(): ExpandedDeploymentDetail {
  return {
    loading: false,
    error: null,
    configFiles: [],
    previewStatusMap: {},
    configHistoryMap: {},
  };
}

async function handleInstanceExpandChange(
  row: DeploymentInstanceSummary,
  expandedRows: DeploymentInstanceSummary[],
) {
  if (!expandedRows.some((item) => item.id === row.id)) {
    return;
  }

  await loadExpandedDeploymentDetail(row);
}

async function loadExpandedDeploymentDetail(
  row: DeploymentInstanceSummary,
  force = false,
) {
  const existing = expandedDetails.value[row.id];
  if (
    existing &&
    !force &&
    (existing.loading || existing.configFiles.length > 0 || existing.error)
  ) {
    return;
  }

  const detail = existing ?? emptyExpandedDetail();
  setExpandedDetail(row.id, { ...detail, loading: true, error: null });

  try {
    const configFilesPromise = configFilesApi.listConfigFiles({
      project_id: projectId.value,
      status: "active",
    });

    if (!canOpenWorkspace(row)) {
      const configResult = await configFilesPromise;
      setExpandedDetail(row.id, {
        ...emptyExpandedDetail(),
        configFiles: configResult.items,
      });
      return;
    }

    const [configResult, preview, savedVersions, releases] = await Promise.all([
      configFilesPromise,
      deploymentInstancesApi.previewDeploymentBundle(row.id),
      savedVersionsApi.listSavedVersions({
        deployment_instance_id: row.id,
      }),
      releasesApi.listReleases({
        project_id: projectId.value,
        deployment_instance_id: row.id,
      }),
    ]);

    const previewStatusMap: Record<number, DeploymentPreviewItem> = {};
    for (const item of preview.items) {
      previewStatusMap[item.config_file_id] = item;
    }

    const configHistoryMap: Record<number, ConfigHistoryHint> = {};
    for (const configFile of configResult.items) {
      configHistoryMap[configFile.id] = {
        savedVersionsCount: 0,
        latestReleaseRevision: null,
      };
    }

    for (const item of savedVersions.items) {
      const hint =
        configHistoryMap[item.config_file_id] ??
        (configHistoryMap[item.config_file_id] = {
          savedVersionsCount: 0,
          latestReleaseRevision: null,
        });
      hint.savedVersionsCount += 1;
    }

    for (const item of releases.items) {
      const hint =
        configHistoryMap[item.config_file_id] ??
        (configHistoryMap[item.config_file_id] = {
          savedVersionsCount: 0,
          latestReleaseRevision: null,
        });
      hint.latestReleaseRevision ??= item.revision;
    }

    setExpandedDetail(row.id, {
      loading: false,
      error: null,
      configFiles: configResult.items,
      previewStatusMap,
      configHistoryMap,
    });
  } catch (err) {
    const error =
      err instanceof ApiRequestError
        ? err
        : new ApiRequestError(0, {
            code: "unknown_error",
            message: t("deployments.expanded.configLoadError"),
          });
    setExpandedDetail(row.id, {
      ...emptyExpandedDetail(),
      error,
    });
  }
}

function expandedDetailErrorMessage(id: number) {
  const error = expandedDetails.value[id]?.error;
  if (!error) return undefined;
  return getErrorMessage(error.code, error.message);
}

function previewItem(rowId: number, configFileId: number) {
  return expandedDetails.value[rowId]?.previewStatusMap[configFileId] ?? null;
}

function historyHint(rowId: number, configFileId: number) {
  return expandedDetails.value[rowId]?.configHistoryMap[configFileId] ?? null;
}

function configStateTagType(rowId: number, configFileId: number) {
  const item = previewItem(rowId, configFileId);
  if (!item) return "info";
  if (item.source === "draft") return "warning";
  if (item.source === "latest_release") return "success";
  return "info";
}

function configStateLabel(rowId: number, configFileId: number) {
  const item = previewItem(rowId, configFileId);
  if (!item) return t("preview.source.none");
  if (item.source === "draft" || item.source === "latest_release") {
    return t(`preview.source.${item.source}`);
  }
  return t("preview.source.none");
}

function configReadinessTagType(rowId: number, configFileId: number) {
  const item = previewItem(rowId, configFileId);
  if (!item) return "info";
  if (item.status === "missing_required") return "danger";
  if (item.status === "missing_optional") return "info";
  return "success";
}

function configReadinessLabel(rowId: number, configFileId: number) {
  const item = previewItem(rowId, configFileId);
  if (!item) return t("preview.source.none");
  return t(`preview.status.${item.status}`);
}

function configSavedVersionsLabel(rowId: number, configFileId: number) {
  const hint = historyHint(rowId, configFileId);
  return t("deployments.configs.savedVersionsCount", {
    count: hint?.savedVersionsCount ?? 0,
  });
}

function configLatestReleaseLabel(rowId: number, configFileId: number) {
  const revision = historyHint(rowId, configFileId)?.latestReleaseRevision;
  if (!revision) return t("deployments.configs.noRelease");
  return t("deployments.configs.latestRelease", { revision });
}

function tokenStatusLabel(row: DeploymentInstanceSummary) {
  return row.status === "active"
    ? t("deployments.expanded.tokenActive")
    : t("deployments.expanded.tokenInactive");
}

function deploymentUpdatedAt(row: DeploymentInstanceSummary) {
  const value = (row as DeploymentInstanceSummary & { updated_at?: string })
    .updated_at;
  if (!value) return t("deployments.expanded.noUpdatedAt");

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString();
}

function openDraftOverlay(
  row: DeploymentInstanceSummary,
  configFileId: number,
) {
  draftOverlayDeploymentId.value = row.id;
  draftOverlayConfigFileId.value = configFileId;
}

async function closeDraftOverlay() {
  const deploymentId = draftOverlayDeploymentId.value;
  const row =
    deploymentId === null
      ? null
      : instanceList.items.value.find((item) => item.id === deploymentId);

  draftOverlayDeploymentId.value = null;
  draftOverlayConfigFileId.value = null;

  await instanceList.load();
  if (row) {
    await loadExpandedDeploymentDetail(row, true);
  }
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

.deployment-instance-list-page__muted {
  color: var(--el-text-color-secondary);
}

.deployment-instance-list-page__expanded {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
  padding: 10px 12px 12px 44px;
  background: var(--el-fill-color-lighter);
}

.deployment-instance-list-page__expanded-meta,
.deployment-instance-list-page__config-row,
.deployment-instance-list-page__config-main,
.deployment-instance-list-page__config-hints {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
}

.deployment-instance-list-page__expanded-meta {
  gap: 6px 10px;
  color: var(--el-text-color-regular);
  font-size: 13px;
}

.deployment-instance-list-page__expanded-label {
  color: var(--el-text-color-secondary);
  font-size: 12px;
  font-weight: 600;
}

.deployment-instance-list-page__expanded-pair {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.deployment-instance-list-page__expanded-state {
  padding: var(--spacing-sm) 0;
}

.deployment-instance-list-page__expanded-configs {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.deployment-instance-list-page__config-row {
  justify-content: space-between;
  gap: 8px 12px;
  padding: 8px 0;
  border-top: 1px solid var(--color-border-light);
}

.deployment-instance-list-page__config-main {
  min-width: 220px;
  flex: 1;
  gap: 6px;
}

.deployment-instance-list-page__config-hints {
  min-width: 320px;
  gap: 6px;
  font-size: 12px;
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

  .deployment-instance-list-page__expanded {
    padding-left: 12px;
  }

  .deployment-instance-list-page__config-row {
    align-items: flex-start;
    flex-direction: column;
  }

  .deployment-instance-list-page__config-hints {
    min-width: 0;
  }
}
</style>
