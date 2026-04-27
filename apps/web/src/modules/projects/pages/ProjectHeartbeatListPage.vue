<template>
  <div class="project-heartbeat-list-page page-container">
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

      <div class="project-heartbeat-list-page__section">
        <el-card class="project-heartbeat-list-page__filters" shadow="never">
          <el-form :model="filters" layout="inline">
            <el-form-item>
              <el-select
                v-model="filters.deployment_instance_id"
                :placeholder="t('heartbeats.filter.deployment')"
                clearable
                filterable
                class="project-heartbeat-list-page__filter"
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
                :placeholder="t('heartbeats.filter.configFile')"
                clearable
                filterable
                class="project-heartbeat-list-page__filter"
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
              <el-button type="primary" @click="loadHeartbeats">
                {{ t("heartbeats.filter.search") }}
              </el-button>
              <el-button @click="resetFilters">
                {{ t("heartbeats.filter.reset") }}
              </el-button>
            </el-form-item>
          </el-form>
        </el-card>

        <div class="project-heartbeat-list-page__hint">
          {{ t("heartbeats.page.hint") }}
        </div>

        <LoadingState v-if="listLoading" />

        <ErrorState
          v-else-if="listError"
          :title="t('heartbeats.page.loadError')"
          :subtitle="getErrorMessage(listError.code, listError.message)"
          @retry="loadHeartbeats"
        />

        <EmptyState
          v-else-if="heartbeats.length === 0"
          :description="t('heartbeats.empty')"
        />

        <div v-else class="page-table-shell">
          <el-table :data="heartbeats" stripe style="width: 100%">
            <el-table-column type="expand">
              <template #default="{ row }">
                <div class="project-heartbeat-list-page__detail">
                  <div>
                    <span class="project-heartbeat-list-page__detail-label">
                      {{ t("heartbeats.field.deploymentId") }}
                    </span>
                    {{ row.deployment_instance_id }}
                  </div>
                  <div>
                    <span class="project-heartbeat-list-page__detail-label">
                      {{ t("heartbeats.field.configFileId") }}
                    </span>
                    {{ row.config_file_id }}
                  </div>
                  <pre
                    v-if="row.metadata"
                    class="project-heartbeat-list-page__json"
                  ><code>{{ formatJson(row.metadata) }}</code></pre>
                  <div v-else class="project-heartbeat-list-page__empty">
                    {{ t("heartbeats.emptyMetadata") }}
                  </div>
                </div>
              </template>
            </el-table-column>

            <el-table-column
              prop="config"
              :label="t('heartbeats.column.config')"
              min-width="140"
            />
            <el-table-column
              :label="t('heartbeats.column.reportedAt')"
              width="190"
            >
              <template #default="{ row }">
                {{ formatDate(row.reported_at) }}
              </template>
            </el-table-column>
            <el-table-column
              :label="t('heartbeats.column.metadata')"
              min-width="260"
              show-overflow-tooltip
            >
              <template #default="{ row }">
                {{ metadataSummary(row.metadata) }}
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
import * as heartbeatsApi from "@/api/deployment-heartbeats";
import * as deploymentsApi from "@/api/deployment-instances";
import * as configFilesApi from "@/api/config-files";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type { ConfigFileSummary } from "@/api/types/config-file";
import type { DeploymentInstanceSummary } from "@/api/types/deployment-instance";
import type { DeploymentHeartbeatSummary } from "@/api/types/deployment-heartbeat";
import { useI18nText } from "@/shared/i18n";

interface FilterState {
  deployment_instance_id?: number;
  config_file_id?: number;
}

const route = useRoute();
const { t } = useI18nText();
const {
  project,
  loading: projectLoading,
  error: projectError,
  fetchProject,
} = useProjectContext();

const projectId = computed(() => Number(route.params.projectId));

const filters = reactive<FilterState>({});
const heartbeats = ref<DeploymentHeartbeatSummary[]>([]);
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

async function loadHeartbeats() {
  listLoading.value = true;
  listError.value = null;
  try {
    const res = await heartbeatsApi.listDeploymentHeartbeats({
      project_id: projectId.value,
      deployment_instance_id: filters.deployment_instance_id,
      config_file_id: filters.config_file_id,
    });
    heartbeats.value = res.items;
  } catch (err) {
    if (err instanceof ApiRequestError) {
      listError.value = err;
    } else {
      listError.value = new ApiRequestError(0, {
        code: "unknown_error",
        message: t("heartbeats.page.loadError"),
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
    // The heartbeat request below owns the visible failure state.
  }
  await loadHeartbeats();
}

function resetFilters() {
  filters.deployment_instance_id = undefined;
  filters.config_file_id = undefined;
  loadHeartbeats();
}

function deploymentOptionLabel(deployment: DeploymentInstanceSummary): string {
  return `${deployment.environment_code} / ${deployment.deployment_key}`;
}

function configFileOptionLabel(configFile: ConfigFileSummary): string {
  return `${configFile.code} - ${configFile.name}`;
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString();
}

function formatJson(value: unknown): string {
  return JSON.stringify(value, null, 2);
}

function metadataSummary(value: unknown): string {
  if (!value) {
    return t("heartbeats.emptyMetadata");
  }

  if (typeof value === "object" && !Array.isArray(value)) {
    const entries = Object.entries(value as Record<string, unknown>).slice(
      0,
      3,
    );
    return entries
      .map(([key, entryValue]) => `${key}: ${String(entryValue)}`)
      .join(" / ");
  }

  return String(value);
}

onMounted(loadAll);

watch(
  () => route.params.projectId,
  () => loadAll(),
);
</script>

<style scoped>
.project-heartbeat-list-page {
  width: 100%;
}

.project-heartbeat-list-page__section {
  margin-top: var(--spacing-md);
}

.project-heartbeat-list-page__filters {
  margin-bottom: var(--spacing-md);
}

.project-heartbeat-list-page__filter {
  width: 220px;
}

.project-heartbeat-list-page__hint {
  margin-bottom: var(--spacing-md);
  color: var(--color-text-secondary);
}

.project-heartbeat-list-page__detail {
  padding: var(--spacing-md) var(--spacing-xl);
  color: var(--color-text-primary);
  display: grid;
  gap: var(--spacing-sm);
}

.project-heartbeat-list-page__detail-label {
  display: inline-block;
  min-width: 120px;
  color: var(--color-text-secondary);
}

.project-heartbeat-list-page__json {
  margin: var(--spacing-sm) 0 0;
  padding: var(--spacing-md);
  border-radius: var(--radius-sm);
  background: var(--color-bg-subtle);
  color: var(--color-text-primary);
  overflow: auto;
  line-height: 1.5;
}

.project-heartbeat-list-page__empty {
  color: var(--color-text-secondary);
}
</style>
