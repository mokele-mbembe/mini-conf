<template>
  <div class="config-file-list-page">
    <!-- Project context loading/error -->
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
      <!-- Project header -->
      <PageHeader
        :title="project.name"
        :subtitle="project.description ?? undefined"
      >
        <template #actions>
          <StatusBadge :status="project.status" />
        </template>
      </PageHeader>

      <!-- Tabs navigation -->
      <ProjectTabs />

      <!-- Config files section -->
      <div class="config-file-list-page__section">
        <div class="config-file-list-page__toolbar">
          <div class="config-file-list-page__filters">
            <el-select
              v-model="statusFilter"
              :placeholder="t('configFiles.filter.allStatuses')"
              clearable
              style="width: 140px"
              @change="handleStatusFilterChange"
            >
              <el-option :label="t('configFiles.filter.all')" value="" />
              <el-option :label="t('status.active')" value="active" />
              <el-option :label="t('status.archived')" value="archived" />
            </el-select>
          </div>

          <el-button v-if="isAdmin" type="primary" @click="openCreateDialog">
            {{ t("configFiles.create") }}
          </el-button>
        </div>

        <!-- List loading -->
        <LoadingState v-if="listLoading" />

        <ErrorState
          v-else-if="listError"
          :title="t('configFiles.page.loadError')"
          :subtitle="getErrorMessage(listError.code, listError.message)"
          @retry="loadConfigFiles"
        />

        <EmptyState
          v-else-if="configFiles.length === 0"
          :description="t('configFiles.empty')"
        >
          <el-button v-if="isAdmin" type="primary" @click="openCreateDialog">
            {{ t("configFiles.create") }}
          </el-button>
        </EmptyState>

        <el-table v-else :data="configFiles" stripe style="width: 100%">
          <el-table-column
            prop="code"
            :label="t('configFiles.column.code')"
            min-width="140"
          >
            <template #default="{ row }">
              <span class="config-file-list-page__code">{{ row.code }}</span>
            </template>
          </el-table-column>

          <el-table-column
            prop="name"
            :label="t('configFiles.column.name')"
            min-width="150"
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
            width="80"
            align="center"
          >
            <template #default="{ row }">
              <el-tag v-if="row.is_required" size="small" type="danger">
                {{ t("configFiles.required") }}
              </el-tag>
              <span v-else class="config-file-list-page__optional">—</span>
            </template>
          </el-table-column>

          <el-table-column
            :label="t('configFiles.column.sensitivity')"
            width="100"
            align="center"
          >
            <template #default="{ row }">
              <el-tag
                v-if="row.sensitivity === 'secret'"
                size="small"
                type="warning"
              >
                {{ t("configFiles.secret") }}
              </el-tag>
              <el-tag v-else size="small" type="info">
                {{ t("configFiles.normal") }}
              </el-tag>
            </template>
          </el-table-column>

          <el-table-column
            :label="t('configFiles.column.status')"
            width="100"
            align="center"
          >
            <template #default="{ row }">
              <StatusBadge :status="row.status" />
            </template>
          </el-table-column>

          <el-table-column
            v-if="isAdmin"
            :label="t('configFiles.column.actions')"
            width="80"
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
                {{ t("configFiles.action.edit") }}
              </el-button>
            </template>
          </el-table-column>
        </el-table>
      </div>
    </template>

    <!-- Form dialog -->
    <ConfigFileFormDialog
      v-model:visible="dialogVisible"
      :project-id="projectId"
      :edit-target="editTarget"
      @success="handleFormSuccess"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { useRoute } from "vue-router";
import { useProjectContext } from "@/modules/projects/composables/useProjectContext";
import ProjectTabs from "@/modules/projects/components/ProjectTabs.vue";
import ConfigFileFormDialog from "../components/ConfigFileFormDialog.vue";
import PageHeader from "@/shared/components/PageHeader.vue";
import StatusBadge from "@/shared/components/StatusBadge.vue";
import LoadingState from "@/shared/states/LoadingState.vue";
import EmptyState from "@/shared/states/EmptyState.vue";
import ErrorState from "@/shared/states/ErrorState.vue";
import ForbiddenState from "@/shared/states/ForbiddenState.vue";
import NotFoundState from "@/shared/states/NotFoundState.vue";
import * as configFilesApi from "@/api/config-files";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type { ConfigFileSummary } from "@/api/types/config-file";
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

// Config files list state
const configFiles = ref<ConfigFileSummary[]>([]);
const listLoading = ref(false);
const listError = ref<ApiRequestError | null>(null);
const statusFilter = ref("");
const isAdmin = computed(() => project.value?.current_user_role === "admin");

// Dialog state
const dialogVisible = ref(false);
const editTarget = ref<ConfigFileSummary | null>(null);

async function loadConfigFiles() {
  listLoading.value = true;
  listError.value = null;
  try {
    const res = await configFilesApi.listConfigFiles({
      project_id: projectId.value,
      status: statusFilter.value || undefined,
    });
    configFiles.value = res.items;
  } catch (err) {
    if (err instanceof ApiRequestError) {
      listError.value = err;
    } else {
      listError.value = new ApiRequestError(0, {
        code: "unknown_error",
        message: t("configFiles.page.loadError"),
      });
    }
  } finally {
    listLoading.value = false;
  }
}

async function loadAll() {
  const id = projectId.value;
  if (isNaN(id)) return;
  await fetchProject(id);
  await loadConfigFiles();
}

function handleStatusFilterChange() {
  loadConfigFiles();
}

function openCreateDialog() {
  editTarget.value = null;
  dialogVisible.value = true;
}

function openEditDialog(row: ConfigFileSummary) {
  editTarget.value = row;
  dialogVisible.value = true;
}

function handleFormSuccess(item: ConfigFileSummary) {
  // Refresh the list after create/edit
  loadConfigFiles();
  // Optimistic update: replace in list if editing
  const idx = configFiles.value.findIndex((c) => c.id === item.id);
  if (idx !== -1) {
    configFiles.value[idx] = item;
  }
}

onMounted(loadAll);

watch(
  () => route.params.projectId,
  () => loadAll(),
);
</script>

<style scoped>
.config-file-list-page {
  max-width: 1200px;
  margin: 0 auto;
  padding: var(--spacing-lg);
}

.config-file-list-page__section {
  margin-top: var(--spacing-md);
}

.config-file-list-page__toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--spacing-md);
  gap: var(--spacing-md);
}

.config-file-list-page__filters {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
}

.config-file-list-page__code {
  font-family: monospace;
  font-size: 0.9em;
}

.config-file-list-page__optional {
  color: var(--color-text-secondary);
}

@media (max-width: 768px) {
  .config-file-list-page {
    padding: var(--spacing-md);
  }

  .config-file-list-page__toolbar {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
