<template>
  <div class="project-environment-list-page">
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

      <div class="project-environment-list-page__section">
        <div class="project-environment-list-page__toolbar">
          <div class="project-environment-list-page__hint">
            {{ t("projectEnvironments.page.hint") }}
          </div>

          <el-button v-if="isAdmin" type="primary" @click="openCreateDialog">
            {{ t("projectEnvironments.create") }}
          </el-button>
        </div>

        <LoadingState v-if="listLoading" />

        <ErrorState
          v-else-if="listError"
          :title="t('projectEnvironments.page.loadError')"
          :subtitle="getErrorMessage(listError.code, listError.message)"
          @retry="loadEnvironments"
        />

        <EmptyState
          v-else-if="environments.length === 0"
          :description="t('projectEnvironments.empty')"
        >
          <el-button v-if="isAdmin" type="primary" @click="openCreateDialog">
            {{ t("projectEnvironments.create") }}
          </el-button>
        </EmptyState>

        <el-table v-else :data="environments" stripe style="width: 100%">
          <el-table-column
            prop="code"
            :label="t('projectEnvironments.column.code')"
            min-width="150"
          />
          <el-table-column
            prop="name"
            :label="t('projectEnvironments.column.name')"
            min-width="180"
          />
          <el-table-column
            :label="t('projectEnvironments.column.status')"
            width="100"
            align="center"
          >
            <template #default="{ row }">
              <StatusBadge :status="row.status" />
            </template>
          </el-table-column>
          <el-table-column
            prop="sort_order"
            :label="t('projectEnvironments.column.sortOrder')"
            width="100"
            align="center"
          />
          <el-table-column
            prop="deployment_count"
            :label="t('projectEnvironments.column.deploymentCount')"
            width="120"
            align="center"
          />
          <el-table-column
            :label="t('projectEnvironments.column.description')"
            min-width="220"
            show-overflow-tooltip
          >
            <template #default="{ row }">
              {{ row.description || t("projectEnvironments.emptyDescription") }}
            </template>
          </el-table-column>
          <el-table-column
            v-if="isAdmin"
            :label="t('projectEnvironments.column.actions')"
            width="130"
            align="center"
            fixed="right"
          >
            <template #default="{ row }">
              <el-button
                text
                type="primary"
                size="small"
                @click="openEditDialog(row)"
              >
                {{ t("projectEnvironments.action.edit") }}
              </el-button>
              <el-button
                text
                type="danger"
                size="small"
                @click="handleDelete(row)"
              >
                {{ t("projectEnvironments.action.delete") }}
              </el-button>
            </template>
          </el-table-column>
        </el-table>
      </div>
    </template>

    <ProjectEnvironmentFormDialog
      v-model:visible="dialogVisible"
      :project-id="projectId"
      :edit-target="editTarget"
      @success="handleDialogSuccess"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { useRoute } from "vue-router";
import { useProjectContext } from "@/modules/projects/composables/useProjectContext";
import ProjectTabs from "@/modules/projects/components/ProjectTabs.vue";
import ProjectEnvironmentFormDialog from "../components/ProjectEnvironmentFormDialog.vue";
import PageHeader from "@/shared/components/PageHeader.vue";
import StatusBadge from "@/shared/components/StatusBadge.vue";
import LoadingState from "@/shared/states/LoadingState.vue";
import EmptyState from "@/shared/states/EmptyState.vue";
import ErrorState from "@/shared/states/ErrorState.vue";
import ForbiddenState from "@/shared/states/ForbiddenState.vue";
import NotFoundState from "@/shared/states/NotFoundState.vue";
import * as projectEnvironmentsApi from "@/api/project-environments";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type { ProjectEnvironmentSummary } from "@/api/types/project-environment";
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
const isAdmin = computed(() => project.value?.current_user_role === "admin");

const environments = ref<ProjectEnvironmentSummary[]>([]);
const listLoading = ref(false);
const listError = ref<ApiRequestError | null>(null);
const dialogVisible = ref(false);
const editTarget = ref<ProjectEnvironmentSummary | null>(null);

async function loadEnvironments() {
  listLoading.value = true;
  listError.value = null;
  try {
    const res = await projectEnvironmentsApi.listProjectEnvironments(
      projectId.value,
    );
    environments.value = [...res.items].sort((a, b) => {
      if (a.sort_order !== b.sort_order) return a.sort_order - b.sort_order;
      return a.code.localeCompare(b.code);
    });
  } catch (err) {
    if (err instanceof ApiRequestError) {
      listError.value = err;
    } else {
      listError.value = new ApiRequestError(0, {
        code: "unknown_error",
        message: t("projectEnvironments.page.loadError"),
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
  await loadEnvironments();
}

function openCreateDialog() {
  editTarget.value = null;
  dialogVisible.value = true;
}

function openEditDialog(row: ProjectEnvironmentSummary) {
  editTarget.value = row;
  dialogVisible.value = true;
}

async function handleDelete(row: ProjectEnvironmentSummary) {
  try {
    await ElMessageBox.confirm(
      t("projectEnvironments.dialog.deleteConfirm", { code: row.code }),
      t("projectEnvironments.dialog.deleteTitle"),
      { type: "warning" },
    );
    await projectEnvironmentsApi.deleteProjectEnvironment(
      projectId.value,
      row.id,
    );
    ElMessage.success(t("toast.projectEnvironments.deleted"));
    await loadEnvironments();
  } catch (err) {
    if (err === "cancel") return;
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else if (err !== "cancel" && err !== "close") {
      ElMessage.error(t("toast.operationFailed"));
    }
  }
}

function handleDialogSuccess() {
  loadEnvironments();
}

onMounted(loadAll);

watch(
  () => route.params.projectId,
  () => loadAll(),
);
</script>

<style scoped>
.project-environment-list-page {
  max-width: 1200px;
  margin: 0 auto;
  padding: var(--spacing-lg);
}

.project-environment-list-page__section {
  margin-top: var(--spacing-md);
}

.project-environment-list-page__toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-md);
  margin-bottom: var(--spacing-md);
}

.project-environment-list-page__hint {
  color: var(--color-text-secondary);
}
</style>
