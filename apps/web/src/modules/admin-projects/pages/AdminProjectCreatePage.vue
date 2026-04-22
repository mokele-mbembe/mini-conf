<template>
  <div class="admin-project-create-page">
    <PageHeader
      :title="t('admin.projects.create')"
      :subtitle="t('admin.projects.subtitle')"
    />

    <el-alert
      v-if="!createdProject"
      type="info"
      :closable="false"
      class="admin-project-create-page__notice"
    >
      {{ t("admin.projects.notice") }}
    </el-alert>

    <el-card v-if="!createdProject" class="admin-project-create-page__content">
      <el-form ref="formRef" :model="form" :rules="rules" label-position="top">
        <el-form-item :label="t('admin.projects.form.code')" prop="code">
          <el-input
            v-model="form.code"
            :placeholder="t('admin.projects.form.codePlaceholder')"
          />
        </el-form-item>

        <el-form-item :label="t('admin.projects.form.name')" prop="name">
          <el-input
            v-model="form.name"
            :placeholder="t('admin.projects.form.namePlaceholder')"
          />
        </el-form-item>

        <el-form-item
          :label="t('admin.projects.form.description')"
          prop="description"
        >
          <el-input
            v-model="form.description"
            type="textarea"
            :rows="4"
            :placeholder="t('admin.projects.form.descriptionPlaceholder')"
          />
        </el-form-item>

        <el-form-item
          :label="t('admin.projects.form.initialAdmin')"
          prop="initial_admin_user_id"
        >
          <el-select
            v-model="form.initial_admin_user_id"
            filterable
            remote
            reserve-keyword
            clearable
            style="width: 100%"
            :placeholder="t('admin.projects.form.initialAdminPlaceholder')"
            :remote-method="handleInitialAdminSearch"
            :loading="adminSearchLoading"
            @focus="handleInitialAdminFocus"
          >
            <el-option
              v-for="option in initialAdminOptions"
              :key="option.id"
              :label="option.username"
              :value="option.id"
            >
              <div class="admin-project-create-page__option">
                <span>{{ option.username }}</span>
                <div class="admin-project-create-page__option-meta">
                  <el-tag
                    v-if="option.is_platform_admin"
                    size="small"
                    type="warning"
                  >
                    {{ t("admin.users.platformAdmin") }}
                  </el-tag>
                  <span class="admin-project-create-page__option-count">
                    {{
                      t("admin.projects.form.optionProjectCount", {
                        count: option.project_count,
                      })
                    }}
                  </span>
                </div>
              </div>
            </el-option>
          </el-select>
          <div class="admin-project-create-page__field-hint">
            {{ t("admin.projects.form.initialAdminHint") }}
          </div>
        </el-form-item>

        <div class="admin-project-create-page__actions">
          <el-button type="primary" :loading="submitting" @click="handleSubmit">
            {{ t("common.create") }}
          </el-button>
        </div>
      </el-form>
    </el-card>

    <el-card v-else class="admin-project-create-page__success-card">
      <el-result icon="success" :title="t('admin.projects.success.title')">
        <template #sub-title>
          <div class="admin-project-create-page__success-subtitle">
            <p>
              {{
                t("admin.projects.success.subtitle", {
                  name: createdProject.project.name,
                  code: createdProject.project.code,
                })
              }}
            </p>
            <p>
              {{
                t("admin.projects.success.initialAdmin", {
                  username: createdProject.initial_admin.username,
                })
              }}
            </p>
          </div>
        </template>
        <template #extra>
          <el-space wrap>
            <el-button type="primary" @click="handleCreateAnother">
              {{ t("admin.projects.success.createAnother") }}
            </el-button>
            <el-button v-if="canGoToProjectList" @click="goToProjectList">
              {{ t("admin.projects.success.goToProjects") }}
            </el-button>
          </el-space>
        </template>
      </el-result>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import type { FormInstance, FormRules } from "element-plus";
import { ElMessage } from "element-plus";
import * as adminProjectsApi from "@/api/admin-projects";
import * as adminUsersApi from "@/api/admin-users";
import type {
  AdminProjectCreateRequest,
  AdminProjectCreateResponse,
} from "@/api/types/admin-project";
import type { AdminUserSummary } from "@/api/types/admin-user";
import { isApiError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import { ROUTE_NAMES } from "@/shared/constants/routes";
import { useI18nText } from "@/shared/i18n";
import PageHeader from "@/shared/components/PageHeader.vue";
import { useAuthSession } from "@/modules/auth/composables/useAuthSession";

interface FormState {
  code: string;
  name: string;
  description: string;
  initial_admin_user_id: number | null;
}

const router = useRouter();
const { t } = useI18nText();
const authSession = useAuthSession();
const formRef = ref<FormInstance>();
const submitting = ref(false);
const adminSearchLoading = ref(false);
const initialAdminOptions = ref<AdminUserSummary[]>([]);
const createdProject = ref<AdminProjectCreateResponse | null>(null);
let adminSearchRequestSeq = 0;

function createInitialForm(): FormState {
  return {
    code: "",
    name: "",
    description: "",
    initial_admin_user_id: null,
  };
}

const form = ref<FormState>(createInitialForm());

const rules: FormRules<FormState> = {
  code: [
    {
      required: true,
      message: t("validation.adminProjects.codeRequired"),
      trigger: "blur",
    },
  ],
  name: [
    {
      required: true,
      message: t("validation.adminProjects.nameRequired"),
      trigger: "blur",
    },
  ],
  initial_admin_user_id: [
    {
      required: true,
      message: t("validation.adminProjects.initialAdminRequired"),
      trigger: "change",
    },
  ],
};

const canGoToProjectList = computed(
  () =>
    createdProject.value !== null &&
    authSession.user?.id === createdProject.value.initial_admin.user_id,
);

async function searchInitialAdminOptions(keyword: string) {
  const seq = ++adminSearchRequestSeq;
  adminSearchLoading.value = true;

  try {
    const response = await adminUsersApi.listAdminUsers({
      keyword: keyword.trim() || undefined,
      status: "active",
      page: 1,
      page_size: 20,
    });

    if (seq !== adminSearchRequestSeq) return;
    initialAdminOptions.value = response.items;
  } catch (err) {
    if (seq !== adminSearchRequestSeq) return;
    if (isApiError(err)) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("admin.projects.initialAdminLoadFailed"));
    }
  } finally {
    if (seq === adminSearchRequestSeq) {
      adminSearchLoading.value = false;
    }
  }
}

function handleInitialAdminSearch(keyword: string) {
  void searchInitialAdminOptions(keyword);
}

function handleInitialAdminFocus() {
  if (initialAdminOptions.value.length > 0) {
    return;
  }

  void searchInitialAdminOptions("");
}

async function handleSubmit() {
  const valid = await formRef.value?.validate().catch(() => false);
  if (!valid || form.value.initial_admin_user_id == null) {
    return;
  }

  const payload: AdminProjectCreateRequest = {
    code: form.value.code.trim(),
    name: form.value.name.trim(),
    description: form.value.description.trim() || undefined,
    initial_admin_user_id: form.value.initial_admin_user_id,
  };

  submitting.value = true;
  try {
    const response = await adminProjectsApi.createAdminProject(payload);
    createdProject.value = response;
    ElMessage.success(t("admin.projects.created"));
  } catch (err) {
    if (isApiError(err)) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("admin.projects.createFailed"));
    }
  } finally {
    submitting.value = false;
  }
}

function handleCreateAnother() {
  createdProject.value = null;
  form.value = createInitialForm();
  formRef.value?.clearValidate();
  void searchInitialAdminOptions("");
}

function goToProjectList() {
  router.push({ name: ROUTE_NAMES.PROJECTS });
}
</script>

<style scoped>
.admin-project-create-page {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-lg);
}

.admin-project-create-page__notice {
  margin-bottom: calc(var(--spacing-md) * -1);
}

.admin-project-create-page__content {
  padding: var(--spacing-lg);
}

.admin-project-create-page__field-hint {
  margin-top: var(--spacing-xs);
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.admin-project-create-page__option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-sm);
}

.admin-project-create-page__option-meta {
  display: flex;
  align-items: center;
  gap: var(--spacing-xs);
}

.admin-project-create-page__option-count {
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.admin-project-create-page__actions {
  display: flex;
  justify-content: flex-end;
}

.admin-project-create-page__success-card {
  padding: var(--spacing-md);
}

.admin-project-create-page__success-subtitle {
  color: var(--color-text-secondary);
  line-height: 1.7;
}

.admin-project-create-page__success-subtitle p {
  margin: 0;
}

@media (max-width: 768px) {
  .admin-project-create-page__option {
    flex-direction: column;
    align-items: flex-start;
  }

  .admin-project-create-page__actions {
    justify-content: stretch;
  }

  .admin-project-create-page__actions :deep(.el-button) {
    width: 100%;
  }
}
</style>
