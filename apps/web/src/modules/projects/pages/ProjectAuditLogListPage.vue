<template>
  <div class="project-audit-log-list-page page-container">
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

      <ForbiddenState v-if="!isAdmin" :subtitle="t('auditLogs.forbidden')" />

      <div v-else class="project-audit-log-list-page__section">
        <el-card class="project-audit-log-list-page__filters" shadow="never">
          <el-form :model="filters" layout="inline">
            <el-form-item>
              <el-input
                v-model="filters.user_id"
                :placeholder="t('auditLogs.filter.userId')"
                clearable
                class="project-audit-log-list-page__filter"
              />
            </el-form-item>

            <el-form-item>
              <el-input
                v-model="filters.action"
                :placeholder="t('auditLogs.filter.action')"
                clearable
                class="project-audit-log-list-page__filter"
              />
            </el-form-item>

            <el-form-item>
              <el-input
                v-model="filters.resource_type"
                :placeholder="t('auditLogs.filter.resourceType')"
                clearable
                class="project-audit-log-list-page__filter"
              />
            </el-form-item>

            <el-form-item>
              <el-button type="primary" @click="loadAuditLogs">
                {{ t("auditLogs.filter.search") }}
              </el-button>
              <el-button @click="resetFilters">
                {{ t("auditLogs.filter.reset") }}
              </el-button>
            </el-form-item>
          </el-form>
        </el-card>

        <div class="project-audit-log-list-page__hint">
          {{ t("auditLogs.page.hint") }}
        </div>

        <LoadingState v-if="listLoading" />

        <ErrorState
          v-else-if="listError"
          :title="t('auditLogs.page.loadError')"
          :subtitle="getErrorMessage(listError.code, listError.message)"
          @retry="loadAuditLogs"
        />

        <EmptyState
          v-else-if="logs.length === 0"
          :description="t('auditLogs.empty')"
        />

        <div v-else class="page-table-shell">
          <el-table :data="logs" stripe style="width: 100%">
            <el-table-column type="expand">
              <template #default="{ row }">
                <div class="project-audit-log-list-page__detail">
                  <div>
                    <span class="project-audit-log-list-page__detail-label">
                      {{ t("auditLogs.field.projectId") }}
                    </span>
                    {{ row.project_id ?? t("auditLogs.emptyValue") }}
                  </div>
                  <div>
                    <span class="project-audit-log-list-page__detail-label">
                      {{ t("auditLogs.field.userId") }}
                    </span>
                    {{ row.user_id ?? t("auditLogs.emptyValue") }}
                  </div>
                  <pre
                    v-if="row.detail"
                    class="project-audit-log-list-page__json"
                  ><code>{{ formatJson(row.detail) }}</code></pre>
                  <div v-else class="project-audit-log-list-page__empty">
                    {{ t("auditLogs.emptyDetail") }}
                  </div>
                </div>
              </template>
            </el-table-column>

            <el-table-column
              prop="action"
              :label="t('auditLogs.column.action')"
              min-width="230"
              show-overflow-tooltip
            />
            <el-table-column
              prop="resource_type"
              :label="t('auditLogs.column.resourceType')"
              width="150"
              show-overflow-tooltip
            />
            <el-table-column
              prop="resource_id"
              :label="t('auditLogs.column.resourceId')"
              width="150"
              show-overflow-tooltip
            />
            <el-table-column
              :label="t('auditLogs.column.userId')"
              width="110"
              align="center"
            >
              <template #default="{ row }">
                {{ row.user_id ?? t("auditLogs.emptyValue") }}
              </template>
            </el-table-column>
            <el-table-column
              :label="t('auditLogs.column.createdAt')"
              width="190"
            >
              <template #default="{ row }">
                {{ formatDate(row.created_at) }}
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
import * as auditLogsApi from "@/api/audit-logs";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type { AuditLogSummary } from "@/api/types/audit-log";
import { useI18nText } from "@/shared/i18n";

interface FilterState {
  user_id: string;
  action: string;
  resource_type: string;
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
const isAdmin = computed(() => project.value?.current_user_role === "admin");
const filters = reactive<FilterState>({
  user_id: "",
  action: "",
  resource_type: "",
});
const logs = ref<AuditLogSummary[]>([]);
const listLoading = ref(false);
const listError = ref<ApiRequestError | null>(null);

async function loadAuditLogs() {
  if (!isAdmin.value) {
    logs.value = [];
    return;
  }

  listLoading.value = true;
  listError.value = null;
  try {
    const res = await auditLogsApi.listAuditLogs({
      project_id: projectId.value,
      user_id: parseUserIdFilter(filters.user_id),
      action: normalizeFilter(filters.action),
      resource_type: normalizeFilter(filters.resource_type),
    });
    logs.value = res.items;
  } catch (err) {
    if (err instanceof ApiRequestError) {
      listError.value = err;
    } else {
      listError.value = new ApiRequestError(0, {
        code: "unknown_error",
        message: t("auditLogs.page.loadError"),
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
  await loadAuditLogs();
}

function resetFilters() {
  filters.user_id = "";
  filters.action = "";
  filters.resource_type = "";
  loadAuditLogs();
}

function normalizeFilter(value: string): string | undefined {
  const trimmed = value.trim();
  return trimmed ? trimmed : undefined;
}

function parseUserIdFilter(value: string): number | undefined {
  const trimmed = value.trim();
  if (!trimmed) return undefined;
  const parsed = Number(trimmed);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : undefined;
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
.project-audit-log-list-page {
  width: 100%;
}

.project-audit-log-list-page__section {
  margin-top: var(--spacing-md);
}

.project-audit-log-list-page__filters {
  margin-bottom: var(--spacing-md);
}

.project-audit-log-list-page__filter {
  width: 220px;
}

.project-audit-log-list-page__hint {
  margin-bottom: var(--spacing-md);
  color: var(--color-text-secondary);
}

.project-audit-log-list-page__detail {
  padding: var(--spacing-md) var(--spacing-xl);
  color: var(--color-text-primary);
  display: grid;
  gap: var(--spacing-sm);
}

.project-audit-log-list-page__detail-label {
  display: inline-block;
  min-width: 120px;
  color: var(--color-text-secondary);
}

.project-audit-log-list-page__json {
  margin: var(--spacing-sm) 0 0;
  padding: var(--spacing-md);
  border-radius: var(--radius-sm);
  background: var(--color-bg-subtle);
  color: var(--color-text-primary);
  overflow: auto;
  line-height: 1.5;
}

.project-audit-log-list-page__empty {
  color: var(--color-text-secondary);
}
</style>
