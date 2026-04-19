<template>
  <div class="release-list-page page-container">
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
        :title="t('releases.page.title')"
        :subtitle="t('releases.page.subtitle', { project: project.name })"
      >
        <template #actions>
          <StatusBadge :status="project.status" />
        </template>
      </PageHeader>

      <ProjectTabs />

      <div class="release-list-page__section">
        <LoadingState v-if="listLoading" />

        <ErrorState
          v-else-if="listError"
          :title="t('releases.page.loadError')"
          :subtitle="getErrorMessage(listError.code, listError.message)"
          @retry="loadReleases"
        />

        <EmptyState
          v-else-if="releases.length === 0"
          :description="t('releases.empty')"
        />

        <div v-else class="release-list-page__table page-table-shell">
          <el-table :data="releases" stripe style="width: 100%">
            <el-table-column
              prop="revision"
              :label="t('releases.column.revision')"
              min-width="160"
            >
              <template #default="{ row }">
                <span class="release-list-page__code">
                  {{ row.revision }}
                </span>
              </template>
            </el-table-column>

            <el-table-column
              :label="t('deployments.column.deploymentKey')"
              min-width="180"
            >
              <template #default="{ row }">
                {{ deploymentLabel(row.deployment_instance_id) }}
              </template>
            </el-table-column>

            <el-table-column
              :label="t('configFiles.column.code')"
              min-width="150"
            >
              <template #default="{ row }">
                {{ configLabel(row.config_file_id) }}
              </template>
            </el-table-column>

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
              prop="change_summary"
              :label="t('releases.column.changeSummary')"
              min-width="220"
              show-overflow-tooltip
            >
              <template #default="{ row }">
                {{ row.change_summary || t("releases.emptyChangeSummary") }}
              </template>
            </el-table-column>

            <el-table-column
              prop="published_at"
              :label="t('releases.column.publishedAt')"
              min-width="180"
            />
          </el-table>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
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
import * as releasesApi from "@/api/releases";
import * as deploymentInstancesApi from "@/api/deployment-instances";
import * as configFilesApi from "@/api/config-files";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type { ConfigFileSummary } from "@/api/types/config-file";
import type { DeploymentInstanceSummary } from "@/api/types/deployment-instance";
import type { ReleaseSummary } from "@/api/types/release";
import { useI18nText } from "@/shared/i18n";

const route = useRoute();
const { t } = useI18nText();

const {
  project,
  loading: projectLoading,
  error: projectError,
  fetchProject,
} = useProjectContext();

const projectId = computed(() => Number(route.params.projectId));
const deploymentFilter = computed(() =>
  typeof route.query.deployment_instance_id === "string"
    ? Number(route.query.deployment_instance_id)
    : undefined,
);
const configFilter = computed(() =>
  typeof route.query.config_file_id === "string"
    ? Number(route.query.config_file_id)
    : undefined,
);

const releases = ref<ReleaseSummary[]>([]);
const deployments = ref<DeploymentInstanceSummary[]>([]);
const configFiles = ref<ConfigFileSummary[]>([]);
const listLoading = ref(false);
const listError = ref<ApiRequestError | null>(null);

async function loadReleases() {
  listLoading.value = true;
  listError.value = null;
  try {
    const [releaseResult, deploymentResult, configResult] = await Promise.all([
      releasesApi.listReleases({
        project_id: projectId.value,
        deployment_instance_id: Number.isNaN(deploymentFilter.value ?? NaN)
          ? undefined
          : deploymentFilter.value,
        config_file_id: Number.isNaN(configFilter.value ?? NaN)
          ? undefined
          : configFilter.value,
      }),
      deploymentInstancesApi.listDeploymentInstances({
        project_id: projectId.value,
        page: 1,
        page_size: 100,
      }),
      configFilesApi.listConfigFiles({ project_id: projectId.value }),
    ]);
    releases.value = releaseResult.items;
    deployments.value = deploymentResult.items;
    configFiles.value = configResult.items;
  } catch (err) {
    if (err instanceof ApiRequestError) {
      listError.value = err;
    } else {
      listError.value = new ApiRequestError(0, {
        code: "unknown_error",
        message: t("releases.page.loadError"),
      });
    }
  } finally {
    listLoading.value = false;
  }
}

async function loadAll() {
  const pid = projectId.value;
  if (Number.isNaN(pid)) return;
  await fetchProject(pid);
  await loadReleases();
}

function deploymentLabel(id: number): string {
  const deployment = deployments.value.find((item) => item.id === id);
  return deployment
    ? `${deployment.name} / ${deployment.deployment_key}`
    : String(id);
}

function configLabel(id: number): string {
  const config = configFiles.value.find((item) => item.id === id);
  return config ? `${config.name} / ${config.code}` : String(id);
}

onMounted(loadAll);

watch(
  () => [
    route.params.projectId,
    route.query.deployment_instance_id,
    route.query.config_file_id,
  ],
  () => loadAll(),
);
</script>

<style scoped>
.release-list-page {
  width: 100%;
}

.release-list-page__section {
  margin-top: var(--spacing-md);
}

.release-list-page__code {
  font-family: monospace;
}
</style>
