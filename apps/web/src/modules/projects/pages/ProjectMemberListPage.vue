<template>
  <div class="project-member-list-page page-container">
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

      <ForbiddenState
        v-if="!isAdmin"
        :subtitle="t('projectMembers.forbidden')"
      />

      <div v-else class="project-member-list-page__section">
        <div class="project-member-list-page__toolbar">
          <div class="project-member-list-page__hint">
            {{ t("projectMembers.page.hint") }}
          </div>

          <el-button type="primary" @click="dialogVisible = true">
            {{ t("projectMembers.action.add") }}
          </el-button>
        </div>

        <LoadingState v-if="listLoading" />

        <ErrorState
          v-else-if="listError"
          :title="t('projectMembers.page.loadError')"
          :subtitle="getErrorMessage(listError.code, listError.message)"
          @retry="loadMembers"
        />

        <EmptyState
          v-else-if="members.length === 0"
          :description="t('projectMembers.empty')"
        >
          <el-button type="primary" @click="dialogVisible = true">
            {{ t("projectMembers.action.add") }}
          </el-button>
        </EmptyState>

        <div v-else class="page-table-shell">
          <el-table :data="members" stripe style="width: 100%">
            <el-table-column
              prop="username"
              :label="t('projectMembers.column.username')"
              min-width="180"
            />
            <el-table-column
              prop="user_id"
              :label="t('projectMembers.column.userId')"
              width="110"
              align="center"
            />
            <el-table-column
              :label="t('projectMembers.column.role')"
              min-width="180"
            >
              <template #default="{ row }">
                <el-select
                  :model-value="row.role"
                  size="small"
                  class="project-member-list-page__role-select"
                  @change="handleRoleChange(row, $event)"
                >
                  <el-option
                    v-for="role in roles"
                    :key="role"
                    :label="roleLabel(role)"
                    :value="role"
                    :disabled="isLastAdmin(row) && role !== 'admin'"
                  />
                </el-select>
              </template>
            </el-table-column>
            <el-table-column
              :label="t('projectMembers.column.createdAt')"
              width="180"
            >
              <template #default="{ row }">
                {{ formatDate(row.created_at) }}
              </template>
            </el-table-column>
            <el-table-column
              :label="t('projectMembers.column.actions')"
              width="120"
              align="center"
              fixed="right"
            >
              <template #default="{ row }">
                <el-tooltip
                  :disabled="!isLastAdmin(row)"
                  :content="t('projectMembers.lastAdminHint')"
                  placement="top"
                >
                  <span>
                    <el-button
                      text
                      type="danger"
                      size="small"
                      :disabled="isLastAdmin(row)"
                      @click="handleDelete(row)"
                    >
                      {{ t("common.delete") }}
                    </el-button>
                  </span>
                </el-tooltip>
              </template>
            </el-table-column>
          </el-table>
        </div>
      </div>
    </template>

    <ProjectMemberFormDialog
      v-model:visible="dialogVisible"
      :project-id="projectId"
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
import ProjectMemberFormDialog from "@/modules/projects/components/ProjectMemberFormDialog.vue";
import PageHeader from "@/shared/components/PageHeader.vue";
import StatusBadge from "@/shared/components/StatusBadge.vue";
import LoadingState from "@/shared/states/LoadingState.vue";
import EmptyState from "@/shared/states/EmptyState.vue";
import ErrorState from "@/shared/states/ErrorState.vue";
import ForbiddenState from "@/shared/states/ForbiddenState.vue";
import NotFoundState from "@/shared/states/NotFoundState.vue";
import * as projectsApi from "@/api/projects";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type { ProjectMember, ProjectRole } from "@/api/types/project";
import { useI18nText } from "@/shared/i18n";

const route = useRoute();
const { t } = useI18nText();
const {
  project,
  loading: projectLoading,
  error: projectError,
  fetchProject,
} = useProjectContext();

const roles: ProjectRole[] = ["admin", "editor", "viewer"];
const projectId = computed(() => Number(route.params.projectId));
const isAdmin = computed(() => project.value?.current_user_role === "admin");
const adminCount = computed(
  () => members.value.filter((member) => member.role === "admin").length,
);

const members = ref<ProjectMember[]>([]);
const listLoading = ref(false);
const listError = ref<ApiRequestError | null>(null);
const dialogVisible = ref(false);

async function loadMembers() {
  if (!isAdmin.value) {
    members.value = [];
    return;
  }

  listLoading.value = true;
  listError.value = null;
  try {
    const res = await projectsApi.getProjectMembers(projectId.value);
    members.value = res.items;
  } catch (err) {
    if (err instanceof ApiRequestError) {
      listError.value = err;
    } else {
      listError.value = new ApiRequestError(0, {
        code: "unknown_error",
        message: t("projectMembers.page.loadError"),
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
  await loadMembers();
}

function roleLabel(role: ProjectRole): string {
  return t(`projectMembers.role.${role}`);
}

function isLastAdmin(member: ProjectMember): boolean {
  return member.role === "admin" && adminCount.value <= 1;
}

async function handleRoleChange(member: ProjectMember, value: unknown) {
  if (!isProjectRole(value) || value === member.role) {
    return;
  }

  try {
    await projectsApi.updateProjectMember(projectId.value, member.id, {
      role: value,
    });
    ElMessage.success(t("projectMembers.toast.updated"));
    await loadAll();
  } catch (err) {
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("projectMembers.toast.updateFailed"));
    }
    await loadMembers();
  }
}

async function handleDelete(member: ProjectMember) {
  try {
    await ElMessageBox.confirm(
      t("projectMembers.dialog.deleteConfirm", {
        username: member.username,
      }),
      t("projectMembers.dialog.deleteTitle"),
      {
        type: "warning",
        confirmButtonText: t("common.delete"),
        confirmButtonClass: "el-button--danger",
      },
    );
    await projectsApi.deleteProjectMember(projectId.value, member.id);
    ElMessage.success(t("projectMembers.toast.deleted"));
    await loadMembers();
  } catch (err) {
    if (err === "cancel" || err === "close") return;
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("projectMembers.toast.deleteFailed"));
    }
  }
}

function handleDialogSuccess() {
  loadMembers();
}

function isProjectRole(value: unknown): value is ProjectRole {
  return value === "admin" || value === "editor" || value === "viewer";
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString();
}

onMounted(loadAll);

watch(
  () => route.params.projectId,
  () => loadAll(),
);
</script>

<style scoped>
.project-member-list-page {
  width: 100%;
}

.project-member-list-page__section {
  margin-top: var(--spacing-md);
}

.project-member-list-page__toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-md);
  margin-bottom: var(--spacing-md);
}

.project-member-list-page__hint {
  color: var(--color-text-secondary);
}

.project-member-list-page__role-select {
  width: 150px;
}
</style>
