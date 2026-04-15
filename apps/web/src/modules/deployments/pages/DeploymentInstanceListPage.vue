<template>
  <div class="deployment-instance-list-page">
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

      <div class="deployment-instance-list-page__section">
        <div class="deployment-instance-list-page__toolbar">
          <div class="deployment-instance-list-page__filters">
            <el-input
              v-model="keywordFilter"
              :placeholder="t('deployments.filter.keywordPlaceholder')"
              clearable
              style="width: 220px"
              @keyup.enter="handleSearch"
              @clear="handleSearch"
            />

            <el-input
              v-model="environmentFilter"
              :placeholder="t('deployments.filter.environmentPlaceholder')"
              clearable
              style="width: 180px"
              @keyup.enter="handleSearch"
              @clear="handleSearch"
            />

            <el-select
              v-model="statusFilter"
              :placeholder="t('deployments.filter.allStatuses')"
              clearable
              style="width: 140px"
              @change="handleFilterChange"
            >
              <el-option :label="t('deployments.filter.all')" value="" />
              <el-option :label="t('status.active')" value="active" />
              <el-option :label="t('status.inactive')" value="inactive" />
            </el-select>

            <el-button @click="handleSearch">
              {{ t("deployments.filter.search") }}
            </el-button>

            <el-button text @click="resetFilters">
              {{ t("deployments.filter.reset") }}
            </el-button>
          </div>

          <el-button v-if="isAdmin" type="primary" @click="openCreateDialog">
            {{ t("deployments.create") }}
          </el-button>
        </div>

        <LoadingState v-if="listLoading" />

        <ErrorState
          v-else-if="listError"
          :title="t('deployments.page.loadError')"
          :subtitle="getErrorMessage(listError.code, listError.message)"
          @retry="loadDeploymentInstances"
        />

        <EmptyState
          v-else-if="deployments.length === 0"
          :description="t('deployments.empty')"
        >
          <el-button v-if="isAdmin" type="primary" @click="openCreateDialog">
            {{ t("deployments.create") }}
          </el-button>
        </EmptyState>

        <template v-else>
          <el-table :data="deployments" stripe style="width: 100%">
            <el-table-column
              prop="environment"
              :label="t('deployments.column.environment')"
              width="130"
            >
              <template #default="{ row }">
                <el-tag size="small" type="info">{{ row.environment }}</el-tag>
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
              :label="t('deployments.column.type')"
              width="110"
              align="center"
            >
              <template #default="{ row }">
                <el-tag v-if="row.is_template" size="small" type="warning">
                  {{ t("deployments.type.template") }}
                </el-tag>
                <el-tag v-else size="small" type="success">
                  {{ t("deployments.type.instance") }}
                </el-tag>
              </template>
            </el-table-column>

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
              width="90"
              align="center"
              fixed="right"
            >
              <template #default="{ row }">
                <el-button
                  text
                  type="primary"
                  size="small"
                  @click="openDetail(row)"
                >
                  {{ t("deployments.action.view") }}
                </el-button>
              </template>
            </el-table-column>
          </el-table>

          <div class="deployment-instance-list-page__pagination">
            <el-pagination
              v-model:current-page="page"
              v-model:page-size="pageSize"
              :page-sizes="[10, 20, 50, 100]"
              :total="total"
              layout="total, sizes, prev, pager, next"
              background
              @current-change="loadDeploymentInstances"
              @size-change="handlePageSizeChange"
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
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useProjectContext } from "@/modules/projects/composables/useProjectContext";
import ProjectTabs from "@/modules/projects/components/ProjectTabs.vue";
import DeploymentInstanceCreateDialog from "../components/DeploymentInstanceCreateDialog.vue";
import PageHeader from "@/shared/components/PageHeader.vue";
import StatusBadge from "@/shared/components/StatusBadge.vue";
import LoadingState from "@/shared/states/LoadingState.vue";
import EmptyState from "@/shared/states/EmptyState.vue";
import ErrorState from "@/shared/states/ErrorState.vue";
import ForbiddenState from "@/shared/states/ForbiddenState.vue";
import NotFoundState from "@/shared/states/NotFoundState.vue";
import { ROUTE_NAMES } from "@/shared/constants/routes";
import * as deploymentInstancesApi from "@/api/deployment-instances";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type {
  DeploymentInstanceStatus,
  DeploymentInstanceSummary,
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

const deployments = ref<DeploymentInstanceSummary[]>([]);
const listLoading = ref(false);
const listError = ref<ApiRequestError | null>(null);
const keywordFilter = ref("");
const environmentFilter = ref("");
const statusFilter = ref<DeploymentInstanceStatus | "">("");
const page = ref(1);
const pageSize = ref(20);
const total = ref(0);

const dialogVisible = ref(false);

async function loadDeploymentInstances() {
  listLoading.value = true;
  listError.value = null;
  try {
    const res = await deploymentInstancesApi.listDeploymentInstances({
      project_id: projectId.value,
      keyword: keywordFilter.value.trim() || undefined,
      environment: environmentFilter.value.trim() || undefined,
      status: statusFilter.value || undefined,
      page: page.value,
      page_size: pageSize.value,
    });
    deployments.value = res.items;
    total.value = res.total;
    page.value = res.page;
    pageSize.value = res.page_size;
  } catch (err) {
    if (err instanceof ApiRequestError) {
      listError.value = err;
    } else {
      listError.value = new ApiRequestError(0, {
        code: "unknown_error",
        message: t("deployments.page.loadError"),
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
  await loadDeploymentInstances();
}

function handleSearch() {
  page.value = 1;
  loadDeploymentInstances();
}

function handleFilterChange() {
  page.value = 1;
  loadDeploymentInstances();
}

function resetFilters() {
  keywordFilter.value = "";
  environmentFilter.value = "";
  statusFilter.value = "";
  page.value = 1;
  loadDeploymentInstances();
}

function handlePageSizeChange() {
  page.value = 1;
  loadDeploymentInstances();
}

function openCreateDialog() {
  dialogVisible.value = true;
}

function handleCreateSuccess() {
  statusFilter.value = "";
  page.value = 1;
  loadDeploymentInstances();
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
  max-width: 1200px;
  margin: 0 auto;
  padding: var(--spacing-lg);
}

.deployment-instance-list-page__section {
  margin-top: var(--spacing-md);
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

@media (max-width: 768px) {
  .deployment-instance-list-page {
    padding: var(--spacing-md);
  }

  .deployment-instance-list-page__toolbar {
    flex-direction: column;
  }

  .deployment-instance-list-page__filters {
    width: 100%;
  }
}
</style>
