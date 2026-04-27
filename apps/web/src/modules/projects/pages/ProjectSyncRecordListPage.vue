<template>
  <div class="project-sync-record-list-page page-container">
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
      <PageHeader
        :title="project.name"
        :subtitle="project.description ?? undefined"
      >
        <template #actions>
          <StatusBadge :status="project.status" />
        </template>
      </PageHeader>

      <ProjectTabs />

      <div class="project-sync-record-list-page__section">
        <el-card class="project-sync-record-list-page__filters" shadow="never">
          <el-form :model="filters" layout="inline">
            <el-form-item>
              <el-select
                v-model="filters.deployment_instance_id"
                :placeholder="t('syncRecords.filter.deployment')"
                clearable
                filterable
                class="project-sync-record-list-page__filter"
              >
                <el-option
                  v-for="deployment in deployments"
                  :key="deployment.id"
                  :label="deploymentOptionLabel(deployment)"
                  :value="deployment.id"
                />
              </el-select>
            </el-form-item>

            <el-form-item>
              <el-select
                v-model="filters.config_file_id"
                :placeholder="t('syncRecords.filter.configFile')"
                clearable
                filterable
                class="project-sync-record-list-page__filter"
              >
                <el-option
                  v-for="configFile in configFiles"
                  :key="configFile.id"
                  :label="configFileOptionLabel(configFile)"
                  :value="configFile.id"
                />
              </el-select>
            </el-form-item>

            <el-form-item>
              <el-select
                v-model="filters.action"
                :placeholder="t('syncRecords.filter.action')"
                clearable
                class="project-sync-record-list-page__filter"
              >
                <el-option
                  v-for="action in actions"
                  :key="action"
                  :label="actionLabel(action)"
                  :value="action"
                />
              </el-select>
            </el-form-item>

            <el-form-item>
              <el-select
                v-model="filters.status"
                :placeholder="t('syncRecords.filter.status')"
                clearable
                class="project-sync-record-list-page__filter"
              >
                <el-option
                  v-for="status in statuses"
                  :key="status"
                  :label="statusLabel(status)"
                  :value="status"
                />
              </el-select>
            </el-form-item>

            <el-form-item>
              <el-button type="primary" @click="loadRecords">
                {{ t("syncRecords.filter.search") }}
              </el-button>
              <el-button @click="resetFilters">
                {{ t("syncRecords.filter.reset") }}
              </el-button>
            </el-form-item>
          </el-form>
        </el-card>

        <LoadingState v-if="listLoading" />

        <ErrorState
          v-else-if="listError"
          :title="t('syncRecords.page.loadError')"
          :subtitle="getErrorMessage(listError.code, listError.message)"
          @retry="loadRecords"
        />

        <EmptyState
          v-else-if="records.length === 0"
          :description="t('syncRecords.empty')"
        />

        <div v-else class="page-table-shell">
          <el-table :data="records" stripe style="width: 100%">
            <el-table-column type="expand">
              <template #default="{ row }">
                <div class="project-sync-record-list-page__detail">
                  <div>
                    <span class="project-sync-record-list-page__detail-label">
                      {{ t("syncRecords.field.deploymentId") }}
                    </span>
                    {{ row.deployment_instance_id }}
                  </div>
                  <div>
                    <span class="project-sync-record-list-page__detail-label">
                      {{ t("syncRecords.field.configFileId") }}
                    </span>
                    {{ row.config_file_id }}
                  </div>
                  <div>
                    <span class="project-sync-record-list-page__detail-label">
                      {{ t("syncRecords.field.releaseId") }}
                    </span>
                    {{ row.release_id ?? t("syncRecords.emptyValue") }}
                  </div>
                  <pre
                    v-if="row.detail"
                    class="project-sync-record-list-page__json"
                  ><code>{{ formatJson(row.detail) }}</code></pre>
                  <div v-else class="project-sync-record-list-page__empty">
                    {{ t("syncRecords.emptyDetail") }}
                  </div>
                </div>
              </template>
            </el-table-column>

            <el-table-column
              prop="config"
              :label="t('syncRecords.column.config')"
              min-width="140"
            />
            <el-table-column
              :label="t('syncRecords.column.revision')"
              min-width="150"
              show-overflow-tooltip
            >
              <template #default="{ row }">
                {{ row.revision ?? t("syncRecords.emptyRevision") }}
              </template>
            </el-table-column>
            <el-table-column
              :label="t('syncRecords.column.action')"
              width="130"
            >
              <template #default="{ row }">
                {{ actionLabel(row.action) }}
              </template>
            </el-table-column>
            <el-table-column
              :label="t('syncRecords.column.status')"
              width="110"
              align="center"
            >
              <template #default="{ row }">
                <el-tag :type="statusTagType(row.status)">
                  {{ statusLabel(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column
              :label="t('syncRecords.column.message')"
              min-width="220"
              show-overflow-tooltip
            >
              <template #default="{ row }">
                {{ row.message ?? t("syncRecords.emptyMessage") }}
              </template>
            </el-table-column>
            <el-table-column
              :label="t('syncRecords.column.reportedAt')"
              width="190"
            >
              <template #default="{ row }">
                {{ formatDate(row.reported_at) }}
              </template>
            </el-table-column>
          </el-table>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { useRoute } from "vue-router";
import { useProjectContext } from "@/modules/projects/composables/useProjectContext";
import ProjectTabs from "@/modules/projects/components/ProjectTabs.vue";
import PageHeader from "@/shared/components/PageHeader.vue";
import StatusBadge from "@/shared/components/StatusBadge.vue";
import LoadingState from "@/shared/states/LoadingState.vue";
import EmptyState from "@/shared/states/EmptyState.vue";
import ErrorState from "@/shared/states/ErrorState.vue";
import ForbiddenState from "@/shared/states/ForbiddenState.vue";
import NotFoundState from "@/shared/states/NotFoundState.vue";
import * as syncRecordsApi from "@/api/deployment-sync-records";
import * as deploymentsApi from "@/api/deployment-instances";
import * as configFilesApi from "@/api/config-files";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type { ConfigFileSummary } from "@/api/types/config-file";
import type { DeploymentInstanceSummary } from "@/api/types/deployment-instance";
import type {
  DeploymentSyncAction,
  DeploymentSyncRecordSummary,
  DeploymentSyncStatus,
} from "@/api/types/deployment-sync-record";
import { useI18nText } from "@/shared/i18n";

type TagType = "success" | "info" | "warning" | "danger" | "primary";

interface FilterState {
  deployment_instance_id?: number;
  config_file_id?: number;
  action?: DeploymentSyncAction;
  status?: DeploymentSyncStatus;
}

const route = useRoute();
const { t } = useI18nText();
const {
  project,
  loading: projectLoading,
  error: projectError,
  fetchProject,
} = useProjectContext();

const actions: DeploymentSyncAction[] = [
  "version_check",
  "fetch",
  "apply",
  "heartbeat",
];
const statuses: DeploymentSyncStatus[] = ["success", "noop", "failed"];
const projectId = computed(() => Number(route.params.projectId));

const filters = reactive<FilterState>({});
const records = ref<DeploymentSyncRecordSummary[]>([]);
const deployments = ref<DeploymentInstanceSummary[]>([]);
const configFiles = ref<ConfigFileSummary[]>([]);
const listLoading = ref(false);
const listError = ref<ApiRequestError | null>(null);

async function loadFilterOptions() {
  const [deploymentResponse, configFileResponse] = await Promise.all([
    deploymentsApi.listDeploymentInstances({
      project_id: projectId.value,
      visibility_filter: "current",
      page: 1,
      page_size: 100,
    }),
    configFilesApi.listConfigFiles({
      project_id: projectId.value,
    }),
  ]);

  deployments.value = deploymentResponse.items;
  configFiles.value = configFileResponse.items;
}

async function loadRecords() {
  listLoading.value = true;
  listError.value = null;
  try {
    const res = await syncRecordsApi.listDeploymentSyncRecords({
      project_id: projectId.value,
      deployment_instance_id: filters.deployment_instance_id,
      config_file_id: filters.config_file_id,
      action: filters.action,
      status: filters.status,
    });
    records.value = res.items;
  } catch (err) {
    if (err instanceof ApiRequestError) {
      listError.value = err;
    } else {
      listError.value = new ApiRequestError(0, {
        code: "unknown_error",
        message: t("syncRecords.page.loadError"),
      });
    }
  } finally {
    listLoading.value = false;
  }
}

async function loadAll() {
  const id = projectId.value;
  if (Number.isNaN(id)) return;
  await fetchProject(id);

  if (projectError.value) {
    return;
  }

  try {
    await loadFilterOptions();
  } catch {
    // The records request below owns the visible failure state.
  }
  await loadRecords();
}

function resetFilters() {
  filters.deployment_instance_id = undefined;
  filters.config_file_id = undefined;
  filters.action = undefined;
  filters.status = undefined;
  loadRecords();
}

function deploymentOptionLabel(deployment: DeploymentInstanceSummary): string {
  return `${deployment.environment_code} / ${deployment.deployment_key}`;
}

function configFileOptionLabel(configFile: ConfigFileSummary): string {
  return `${configFile.code} - ${configFile.name}`;
}

function actionLabel(action: DeploymentSyncAction): string {
  return t(`syncRecords.action.${action}`);
}

function statusLabel(status: DeploymentSyncStatus): string {
  return t(`syncRecords.status.${status}`);
}

function statusTagType(status: DeploymentSyncStatus): TagType {
  switch (status) {
    case "success":
      return "success";
    case "noop":
      return "info";
    case "failed":
      return "danger";
  }
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString();
}

function formatJson(value: unknown): string {
  return JSON.stringify(value, null, 2);
}

onMounted(loadAll);

watch(
  () => route.params.projectId,
  () => loadAll(),
);
</script>

<style scoped>
.project-sync-record-list-page {
  width: 100%;
}

.project-sync-record-list-page__section {
  margin-top: var(--spacing-md);
}

.project-sync-record-list-page__filters {
  margin-bottom: var(--spacing-md);
}

.project-sync-record-list-page__filter {
  width: 210px;
}

.project-sync-record-list-page__detail {
  padding: var(--spacing-md) var(--spacing-xl);
  color: var(--color-text-primary);
  display: grid;
  gap: var(--spacing-sm);
}

.project-sync-record-list-page__detail-label {
  display: inline-block;
  min-width: 120px;
  color: var(--color-text-secondary);
}

.project-sync-record-list-page__json {
  margin: var(--spacing-sm) 0 0;
  padding: var(--spacing-md);
  border-radius: var(--radius-sm);
  background: var(--color-bg-subtle);
  color: var(--color-text-primary);
  overflow: auto;
  line-height: 1.5;
}

.project-sync-record-list-page__empty {
  color: var(--color-text-secondary);
}
</style>
