<template>
  <div class="admin-users-page">
    <div class="admin-users-page__header">
      <div>
        <h1>{{ t("admin.users") }}</h1>
      </div>
      <el-button type="primary" @click="showCreateDialog = true">
        {{ t("admin.users.create") }}
      </el-button>
    </div>

    <el-card class="admin-users-page__filters">
      <template #header>
        <span>{{ t("common.filter") }}</span>
      </template>
      <el-form :model="filters" layout="inline">
        <el-form-item>
          <el-input
            v-model="filters.search"
            :placeholder="t('admin.users.search')"
            clearable
            @input="handleSearch"
          />
        </el-form-item>
        <el-form-item>
          <el-select
            v-model="filters.status"
            :placeholder="t('admin.users.filterByStatus')"
            clearable
            @change="handleFilterChange"
          >
            <el-option label="Active" value="active" />
            <el-option label="Disabled" value="disabled" />
          </el-select>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card v-loading="loading">
      <el-table :data="users" stripe>
        <el-table-column
          prop="username"
          :label="t('admin.users.columns.username')"
        />
        <el-table-column prop="status" :label="t('admin.users.columns.status')">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'danger'">
              {{ row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('admin.users.columns.platformRole')">
          <template #default="{ row }">
            <el-tag v-if="row.is_platform_admin" type="warning">
              {{ t("admin.users.platformAdmin") }}
            </el-tag>
            <span v-else>{{ t("admin.users.normalUser") }}</span>
          </template>
        </el-table-column>
        <el-table-column
          prop="must_change_password"
          :label="t('admin.users.columns.mustChangePassword')"
        >
          <template #default="{ row }">
            <el-checkbox v-model="row.must_change_password" disabled />
          </template>
        </el-table-column>
        <el-table-column
          prop="last_login_at"
          :label="t('admin.users.columns.lastLogin')"
        >
          <template #default="{ row }">
            {{ formatDate(row.last_login_at) }}
          </template>
        </el-table-column>
        <el-table-column
          prop="project_count"
          :label="t('admin.users.columns.projectCount')"
          width="120"
        />
        <el-table-column :label="t('admin.users.columns.actions')">
          <template #default="{ row }">
            <el-space>
              <el-dropdown @command="handleDropdownCommand($event, row)">
                <el-button link type="primary" size="small">
                  {{ t("common.actions") }}
                  <el-icon class="el-icon--right">
                    <ArrowDown />
                  </el-icon>
                </el-button>
                <template #dropdown>
                  <el-dropdown-menu>
                    <el-dropdown-item command="toggle-status">
                      {{
                        row.status === "active"
                          ? t("admin.users.actions.disable")
                          : t("admin.users.actions.enable")
                      }}
                    </el-dropdown-item>
                    <el-dropdown-item command="reset-password">
                      {{ t("admin.users.actions.resetPassword") }}
                    </el-dropdown-item>
                    <el-dropdown-item command="toggle-platform-admin">
                      {{
                        row.is_platform_admin
                          ? t("admin.users.actions.revokePlatformAdmin")
                          : t("admin.users.actions.grantPlatformAdmin")
                      }}
                    </el-dropdown-item>
                  </el-dropdown-menu>
                </template>
              </el-dropdown>
            </el-space>
          </template>
        </el-table-column>
      </el-table>

      <div class="admin-users-page__pagination">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50]"
          :total="total"
          layout="total, sizes, prev, pager, next"
          background
          @current-change="loadUsers"
          @size-change="handlePageSizeChange"
        />
      </div>
    </el-card>

    <!-- Create User Dialog -->
    <AdminUserFormDialog
      v-model="showCreateDialog"
      :loading="dialogLoading"
      @submit="handleCreateUser"
    />

    <!-- Reset Password Dialog -->
    <AdminUserResetPasswordDialog
      v-model="showResetPasswordDialog"
      :user="selectedUser"
      :loading="dialogLoading"
      @submit="handleResetPassword"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { ArrowDown } from "@element-plus/icons-vue";
import { isApiError } from "@/api/error";
import { useI18nText } from "@/shared/i18n";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type {
  AdminUser,
  AdminUserCreateRequest,
  AdminUserResetPasswordRequest,
  UserStatus,
} from "@/api/types/admin-user";
import * as adminUsersApi from "@/api/admin-users";
import AdminUserFormDialog from "../components/AdminUserFormDialog.vue";
import AdminUserResetPasswordDialog from "../components/AdminUserResetPasswordDialog.vue";

const { t } = useI18nText();

type AdminUserAction =
  | "toggle-status"
  | "reset-password"
  | "toggle-platform-admin";
type DropdownCommand = string | number | object;

const loading = ref(false);
const dialogLoading = ref(false);
const users = ref<AdminUser[]>([]);
const total = ref(0);
const page = ref(1);
const pageSize = ref(20);
const showCreateDialog = ref(false);
const showResetPasswordDialog = ref(false);
const selectedUser = ref<AdminUser | null>(null);
let requestSeq = 0;

const filters = ref({
  search: "",
  status: "" as UserStatus | "",
});

onMounted(async () => {
  await loadUsers();
});

async function loadUsers() {
  const seq = ++requestSeq;
  loading.value = true;
  try {
    const response = await adminUsersApi.listAdminUsers({
      keyword: filters.value.search || undefined,
      status: filters.value.status || undefined,
      page: page.value,
      page_size: pageSize.value,
    });

    if (seq !== requestSeq) return;

    users.value = response.items;
    total.value = response.total;
    page.value = response.page;
    pageSize.value = response.page_size;
  } catch (err) {
    if (seq !== requestSeq) return;
    if (isApiError(err)) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("admin.users.loadFailed"));
    }
  } finally {
    if (seq === requestSeq) {
      loading.value = false;
    }
  }
}

function handleSearch() {
  page.value = 1;
  void loadUsers();
}

function handleFilterChange() {
  page.value = 1;
  void loadUsers();
}

function handlePageSizeChange() {
  page.value = 1;
  void loadUsers();
}

function handleDropdownCommand(command: DropdownCommand, user: AdminUser) {
  if (typeof command !== "string") {
    return;
  }

  void handleAction(command as AdminUserAction, user);
}

async function handleAction(action: AdminUserAction, user: AdminUser) {
  switch (action) {
    case "toggle-status":
      await handleToggleStatus(user);
      break;
    case "reset-password":
      selectedUser.value = user;
      showResetPasswordDialog.value = true;
      break;
    case "toggle-platform-admin":
      await handleTogglePlatformAdmin(user);
      break;
  }
}

async function handleToggleStatus(user: AdminUser) {
  const newStatus: UserStatus =
    user.status === "active" ? "disabled" : "active";
  const action =
    newStatus === "active" ? t("common.enable") : t("common.disable");

  try {
    await ElMessageBox.confirm(
      t("admin.users.confirmToggleStatus", { action, username: user.username }),
      t("common.confirm"),
      {
        confirmButtonText: t("common.confirm"),
        cancelButtonText: t("common.cancel"),
        type: "warning",
      },
    );

    dialogLoading.value = true;
    await adminUsersApi.updateAdminUser(user.id, { status: newStatus });
    ElMessage.success(
      t("admin.users.statusUpdated", {
        username: user.username,
        status: newStatus,
      }),
    );
    await loadUsers();
  } catch (err) {
    if (err === "cancel" || err === "close") {
      return;
    }
    if (isApiError(err)) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("admin.users.updateFailed"));
    }
  } finally {
    dialogLoading.value = false;
  }
}

async function handleTogglePlatformAdmin(user: AdminUser) {
  const isPlatformAdmin = !user.is_platform_admin;
  const action = isPlatformAdmin
    ? t("admin.users.grantPlatformAdmin")
    : t("admin.users.revokePlatformAdmin");

  try {
    await ElMessageBox.confirm(
      t("admin.users.confirmTogglePlatformAdmin", {
        action,
        username: user.username,
      }),
      t("common.confirm"),
      {
        confirmButtonText: t("common.confirm"),
        cancelButtonText: t("common.cancel"),
        type: "warning",
      },
    );

    dialogLoading.value = true;
    await adminUsersApi.updateAdminUser(user.id, {
      is_platform_admin: isPlatformAdmin,
    });
    ElMessage.success(
      t("admin.users.platformAdminUpdated", { username: user.username }),
    );
    await loadUsers();
  } catch (err) {
    if (err === "cancel" || err === "close") {
      return;
    }
    if (isApiError(err)) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("admin.users.updateFailed"));
    }
  } finally {
    dialogLoading.value = false;
  }
}

async function handleCreateUser(formData: AdminUserCreateRequest) {
  try {
    dialogLoading.value = true;
    await adminUsersApi.createAdminUser(formData);
    ElMessage.success(t("admin.users.created"));
    showCreateDialog.value = false;
    await loadUsers();
  } catch (err) {
    if (isApiError(err)) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("admin.users.createFailed"));
    }
  } finally {
    dialogLoading.value = false;
  }
}

async function handleResetPassword(payload: AdminUserResetPasswordRequest) {
  if (!selectedUser.value) return;

  try {
    dialogLoading.value = true;
    await adminUsersApi.resetAdminUserPassword(selectedUser.value.id, payload);
    ElMessage.success(t("admin.users.passwordReset"));
    showResetPasswordDialog.value = false;
    await loadUsers();
  } catch (err) {
    if (isApiError(err)) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("admin.users.resetPasswordFailed"));
    }
  } finally {
    dialogLoading.value = false;
  }
}

function formatDate(dateStr: string | null) {
  if (!dateStr) return "-";
  return new Date(dateStr).toLocaleString();
}
</script>

<style scoped>
.admin-users-page {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-lg);
}

.admin-users-page__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.admin-users-page__header h1 {
  font-size: var(--font-size-xl);
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0;
}

.admin-users-page__filters {
  margin-top: var(--spacing-md);
}

.admin-users-page__pagination {
  display: flex;
  justify-content: flex-end;
  margin-top: var(--spacing-lg);
}
</style>
