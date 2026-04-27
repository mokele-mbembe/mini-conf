<template>
  <div class="admin-project-list-page">
    <PageHeader
      :title="t('admin.projects')"
      :subtitle="t('admin.projects.listSubtitle')"
    >
      <template #actions>
        <el-button type="primary" @click="goToCreateProject">
          {{ t("admin.projects.create") }}
        </el-button>
      </template>
    </PageHeader>

    <el-card class="admin-project-list-page__filters">
      <template #header>
        <span>{{ t("common.filter") }}</span>
      </template>
      <el-form :model="filters" inline>
        <el-form-item>
          <el-input
            v-model="filters.search"
            :placeholder="t('admin.projects.search')"
            clearable
            @input="handleSearch"
          />
        </el-form-item>
        <el-form-item>
          <el-select
            v-model="filters.status"
            :placeholder="t('admin.projects.filterByStatus')"
            clearable
            @change="handleFilterChange"
          >
            <el-option :label="t('status.active')" value="active" />
            <el-option :label="t('status.archived')" value="archived" />
          </el-select>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card v-loading="loading">
      <el-table :data="projects" stripe>
        <el-table-column
          prop="code"
          :label="t('admin.projects.columns.code')"
          min-width="180"
        />
        <el-table-column
          prop="name"
          :label="t('admin.projects.columns.name')"
          min-width="220"
        />
        <el-table-column
          prop="status"
          :label="t('admin.projects.columns.status')"
          width="120"
        >
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'">
              {{
                row.status === "active"
                  ? t("status.active")
                  : t("status.archived")
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="member_count"
          :label="t('admin.projects.columns.memberCount')"
          width="120"
        />
        <el-table-column
          prop="deployment_count"
          :label="t('admin.projects.columns.deploymentCount')"
          width="120"
        />
        <el-table-column
          prop="created_at"
          :label="t('admin.projects.columns.createdAt')"
          min-width="180"
        >
          <template #default="{ row }">
            {{ formatDate(row.created_at) }}
          </template>
        </el-table-column>
        <el-table-column
          :label="t('admin.projects.columns.actions')"
          width="100"
          align="center"
          fixed="right"
        >
          <template #default="{ row }">
            <el-button
              text
              type="danger"
              size="small"
              :loading="deletingProjectId === row.id"
              @click="handleDeleteProject(row)"
            >
              {{ t("common.delete") }}
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="admin-project-list-page__pagination">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50]"
          :total="total"
          layout="total, sizes, prev, pager, next"
          background
          @current-change="loadProjects"
          @size-change="handlePageSizeChange"
        />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { ElMessage, ElMessageBox } from "element-plus";
import * as adminProjectsApi from "@/api/admin-projects";
import { isApiError } from "@/api/error";
import type { AdminProjectSummary } from "@/api/types/admin-project";
import PageHeader from "@/shared/components/PageHeader.vue";
import { getErrorMessage } from "@/shared/constants/error-messages";
import { ROUTE_NAMES } from "@/shared/constants/routes";
import { useI18nText } from "@/shared/i18n";

const router = useRouter();
const { t } = useI18nText();

const loading = ref(false);
const projects = ref<AdminProjectSummary[]>([]);
const total = ref(0);
const page = ref(1);
const pageSize = ref(20);
const deletingProjectId = ref<number | null>(null);
let requestSeq = 0;

const filters = ref({
  search: "",
  status: "",
});

onMounted(async () => {
  await loadProjects();
});

async function loadProjects() {
  const seq = ++requestSeq;
  loading.value = true;

  try {
    const response = await adminProjectsApi.listAdminProjects({
      keyword: filters.value.search || undefined,
      status: filters.value.status || undefined,
      page: page.value,
      page_size: pageSize.value,
    });

    if (seq !== requestSeq) return;

    projects.value = response.items;
    total.value = response.total;
    page.value = response.page;
    pageSize.value = response.page_size;
  } catch (err) {
    if (seq !== requestSeq) return;
    if (isApiError(err)) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("admin.projects.loadFailed"));
    }
  } finally {
    if (seq === requestSeq) {
      loading.value = false;
    }
  }
}

function handleSearch() {
  page.value = 1;
  void loadProjects();
}

function handleFilterChange() {
  page.value = 1;
  void loadProjects();
}

function handlePageSizeChange() {
  page.value = 1;
  void loadProjects();
}

function goToCreateProject() {
  router.push({ name: ROUTE_NAMES.ADMIN_CREATE_PROJECT });
}

async function handleDeleteProject(project: AdminProjectSummary) {
  try {
    await ElMessageBox.confirm(
      t("admin.projects.delete.confirm", { code: project.code }),
      t("admin.projects.delete.title"),
      {
        confirmButtonText: t("common.delete"),
        cancelButtonText: t("common.cancel"),
        type: "warning",
        confirmButtonClass: "el-button--danger",
      },
    );
  } catch {
    return;
  }

  deletingProjectId.value = project.id;
  try {
    await adminProjectsApi.deleteAdminProject(project.id);
    ElMessage.success(t("admin.projects.delete.success"));
    await loadProjects();
  } catch (err) {
    if (isApiError(err)) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("admin.projects.delete.failed"));
    }
  } finally {
    deletingProjectId.value = null;
  }
}

function formatDate(dateStr: string) {
  return new Date(dateStr).toLocaleString();
}
</script>

<style scoped>
.admin-project-list-page {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-lg);
}

.admin-project-list-page__filters {
  margin-top: calc(var(--spacing-md) * -1);
}

.admin-project-list-page__pagination {
  display: flex;
  justify-content: flex-end;
  margin-top: var(--spacing-lg);
}
</style>
