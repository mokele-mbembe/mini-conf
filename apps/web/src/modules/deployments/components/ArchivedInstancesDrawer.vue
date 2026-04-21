<template>
  <el-drawer
    :model-value="visible"
    :title="t('deployments.section.archived')"
    direction="rtl"
    size="700px"
    @open="onOpen"
    @update:model-value="$emit('update:visible', $event)"
  >
    <template #default>
      <p class="archived-instances-drawer__desc">
        {{ t("deployments.section.archivedDesc") }}
      </p>

      <div class="archived-instances-drawer__toolbar">
        <div class="archived-instances-drawer__filters">
          <el-input
            v-model="keyword"
            :placeholder="t('deployments.filter.keywordPlaceholder')"
            clearable
            style="width: 200px"
            @keyup.enter="search"
            @clear="search"
          />
          <el-button @click="search">
            {{ t("deployments.filter.search") }}
          </el-button>
        </div>
      </div>

      <ErrorState
        v-if="error"
        :title="t('deployments.page.loadError')"
        :subtitle="getErrorMessage(error.code, error.message)"
        @retry="load"
      />

      <EmptyState
        v-else-if="!loading && items.length === 0"
        :description="t('deployments.section.archivedEmpty')"
      />

      <template v-else>
        <div class="page-table-shell">
          <el-table
            v-loading="loading"
            :data="items"
            stripe
            style="width: 100%"
          >
            <el-table-column
              prop="environment_code"
              :label="t('deployments.column.environment')"
              min-width="150"
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
              min-width="160"
            >
              <template #default="{ row }">
                <span class="archived-instances-drawer__code">
                  {{ row.deployment_key }}
                </span>
              </template>
            </el-table-column>

            <el-table-column
              prop="name"
              :label="t('deployments.column.name')"
              min-width="150"
            />

            <el-table-column
              :label="t('deployments.column.archivedAt')"
              min-width="160"
            >
              <template #default="{ row }">
                {{ row.archived_at ?? "—" }}
              </template>
            </el-table-column>

            <el-table-column
              :label="t('deployments.column.archiveReason')"
              min-width="140"
              show-overflow-tooltip
            >
              <template #default="{ row }">
                {{ row.archive_reason || "—" }}
              </template>
            </el-table-column>

            <el-table-column
              :label="t('deployments.column.actions')"
              width="200"
              align="center"
              fixed="right"
            >
              <template #default="{ row }">
                <div class="archived-instances-drawer__actions">
                  <el-button
                    text
                    type="primary"
                    size="small"
                    :loading="isActionLoading(row.id, 'restore')"
                    @click="handleRestore(row)"
                  >
                    {{ t("deployments.action.restore") }}
                  </el-button>
                  <el-button
                    text
                    type="danger"
                    size="small"
                    :loading="isActionLoading(row.id, 'delete')"
                    @click="handleDelete(row)"
                  >
                    {{ t("deployments.action.permanentDelete") }}
                  </el-button>
                </div>
              </template>
            </el-table-column>
          </el-table>
        </div>

        <div class="archived-instances-drawer__pagination">
          <el-pagination
            v-model:current-page="page"
            v-model:page-size="pageSize"
            :page-sizes="[10, 20, 50]"
            :total="total"
            layout="total, sizes, prev, pager, next"
            background
            @current-change="load"
            @size-change="onPageSizeChange"
          />
        </div>
      </template>
    </template>
  </el-drawer>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import * as deploymentInstancesApi from "@/api/deployment-instances";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type { DeploymentInstanceSummary } from "@/api/types/deployment-instance";
import EmptyState from "@/shared/states/EmptyState.vue";
import ErrorState from "@/shared/states/ErrorState.vue";
import { useI18nText } from "@/shared/i18n";

const props = defineProps<{
  visible: boolean;
  projectId: number;
}>();

const emit = defineEmits<{
  (e: "update:visible", value: boolean): void;
  (e: "restored", item: DeploymentInstanceSummary): void;
  (e: "deleted"): void;
}>();

const { t } = useI18nText();

const items = ref<DeploymentInstanceSummary[]>([]);
const loading = ref(false);
const error = ref<ApiRequestError | null>(null);
const keyword = ref("");
const page = ref(1);
const pageSize = ref(20);
const total = ref(0);
const actionTarget = ref<{ id: number; action: string } | null>(null);
let requestSeq = 0;

function isActionLoading(id: number, action: string) {
  return actionTarget.value?.id === id && actionTarget.value.action === action;
}

async function load() {
  const seq = ++requestSeq;
  loading.value = true;
  error.value = null;
  try {
    const res = await deploymentInstancesApi.listDeploymentInstances({
      project_id: props.projectId,
      is_template: false,
      visibility_filter: "archived",
      keyword: keyword.value.trim() || undefined,
      page: page.value,
      page_size: pageSize.value,
    });
    if (seq !== requestSeq) return;
    items.value = res.items;
    total.value = res.total;
    page.value = res.page;
    pageSize.value = res.page_size;
  } catch (err) {
    if (seq !== requestSeq) return;
    if (err instanceof ApiRequestError) {
      error.value = err;
    } else {
      error.value = new ApiRequestError(0, {
        code: "unknown_error",
        message: "Failed to load archived deployments",
      });
    }
  } finally {
    if (seq === requestSeq) {
      loading.value = false;
    }
  }
}

function search() {
  page.value = 1;
  load();
}

function onPageSizeChange() {
  page.value = 1;
  load();
}

function onOpen() {
  keyword.value = "";
  page.value = 1;
  load();
}

async function handleRestore(row: DeploymentInstanceSummary) {
  try {
    await ElMessageBox.confirm(
      t("deployments.dialog.restoreConfirm", { name: row.name }),
      t("deployments.dialog.restoreTitle"),
      { type: "warning" },
    );
    actionTarget.value = { id: row.id, action: "restore" };
    const restored = await deploymentInstancesApi.restoreDeploymentInstance(
      row.id,
    );
    ElMessage.success(t("toast.deployments.restored"));
    emit("restored", restored);
    await load();
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

async function handleDelete(row: DeploymentInstanceSummary) {
  try {
    await ElMessageBox.prompt(
      t("deployments.dialog.deletePrompt", { key: row.deployment_key }),
      t("deployments.dialog.deleteTitle"),
      {
        type: "error",
        inputPlaceholder: t("deployments.dialog.deleteInputPlaceholder"),
        inputPattern: new RegExp(
          `^${row.deployment_key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`,
        ),
        inputErrorMessage: t("deployments.dialog.deleteKeyMismatch"),
        confirmButtonText: t("deployments.action.permanentDelete"),
        cancelButtonText: t("common.cancel"),
      },
    );
    actionTarget.value = { id: row.id, action: "delete" };
    await deploymentInstancesApi.deleteDeploymentInstance(row.id);
    ElMessage.success(t("toast.deployments.deleted"));
    emit("deleted");
    await load();
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
</script>

<style scoped>
.archived-instances-drawer__desc {
  margin: 0 0 var(--spacing-md) 0;
  font-size: 0.9em;
  color: var(--el-text-color-secondary);
}

.archived-instances-drawer__toolbar {
  margin-bottom: var(--spacing-md);
}

.archived-instances-drawer__filters {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--spacing-sm);
}

.archived-instances-drawer__code {
  font-family: monospace;
  font-size: 0.9em;
}

.archived-instances-drawer__pagination {
  display: flex;
  justify-content: flex-end;
  margin-top: var(--spacing-md);
}

.archived-instances-drawer__actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 4px 4px;
}
</style>
