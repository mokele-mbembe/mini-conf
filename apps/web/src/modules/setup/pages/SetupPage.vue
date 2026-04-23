<template>
  <div class="setup-page">
    <div class="setup-page__card">
      <PageHeader :title="t('setup.title')" :subtitle="t('setup.subtitle')" />

      <el-alert
        v-if="setupStatus.loadError"
        :title="setupStatus.loadError"
        type="error"
        :closable="false"
      />

      <template v-else-if="setupStatus.status">
        <el-alert
          :title="
            setupStatus.setupRequired
              ? t('setup.requiredTitle')
              : t('setup.completedTitle')
          "
          :type="setupStatus.setupRequired ? 'warning' : 'success'"
          :closable="false"
          class="setup-page__alert"
        >
          {{
            setupStatus.setupRequired
              ? t("setup.requiredDescription")
              : t("setup.completedDescription")
          }}
        </el-alert>

        <el-descriptions :column="1" border>
          <el-descriptions-item :label="t('setup.fields.platformAdmins')">
            {{ setupStatus.status.active_platform_admin_count }}
          </el-descriptions-item>
          <el-descriptions-item :label="t('setup.fields.projectCount')">
            {{ setupStatus.status.project_count }}
          </el-descriptions-item>
          <el-descriptions-item :label="t('setup.fields.completedAt')">
            {{ formatOptionalDate(setupStatus.status.setup_completed_at) }}
          </el-descriptions-item>
          <el-descriptions-item :label="t('setup.fields.completedBy')">
            {{ setupStatus.status.setup_completed_by_user_id ?? "-" }}
          </el-descriptions-item>
        </el-descriptions>

        <template
          v-if="setupStatus.setupRequired && authSession.isPlatformAdmin"
        >
          <el-steps
            :active="wizardStep"
            finish-status="success"
            class="setup-page__steps"
          >
            <el-step :title="t('setup.wizard.steps.admin')" />
            <el-step :title="t('setup.wizard.steps.project')" />
            <el-step :title="t('setup.wizard.steps.complete')" />
          </el-steps>

          <el-card class="setup-page__section" shadow="never">
            <template #header>
              <div class="setup-page__section-header">
                <span>{{ t("setup.wizard.admin.title") }}</span>
                <el-tag v-if="createdProjectAdmin" type="success">
                  {{ t("setup.wizard.created") }}
                </el-tag>
              </div>
            </template>

            <p class="setup-page__hint">
              {{ t("setup.wizard.admin.description") }}
            </p>

            <el-form
              ref="adminFormRef"
              :model="adminForm"
              :rules="adminRules"
              label-position="top"
            >
              <el-form-item
                :label="t('admin.users.form.username')"
                prop="username"
              >
                <el-input
                  v-model="adminForm.username"
                  :placeholder="t('admin.users.form.usernamePlaceholder')"
                />
              </el-form-item>
              <el-form-item
                :label="t('admin.users.form.password')"
                prop="password"
              >
                <el-input
                  v-model="adminForm.password"
                  type="password"
                  :placeholder="t('admin.users.form.passwordPlaceholder')"
                  show-password
                />
              </el-form-item>
              <el-form-item>
                <el-checkbox v-model="adminForm.must_change_password">
                  {{ t("admin.users.form.mustChangePassword") }}
                </el-checkbox>
              </el-form-item>
              <el-button
                type="primary"
                :loading="creatingAdmin"
                @click="handleCreateProjectAdmin"
              >
                {{ t("setup.wizard.admin.create") }}
              </el-button>
            </el-form>
          </el-card>

          <el-card class="setup-page__section" shadow="never">
            <template #header>
              <div class="setup-page__section-header">
                <span>{{ t("setup.wizard.project.title") }}</span>
                <el-tag v-if="createdProject" type="success">
                  {{ t("setup.wizard.created") }}
                </el-tag>
              </div>
            </template>

            <p class="setup-page__hint">
              {{ t("setup.wizard.project.description") }}
            </p>

            <el-form
              ref="projectFormRef"
              :model="projectForm"
              :rules="projectRules"
              label-position="top"
            >
              <el-form-item :label="t('admin.projects.form.code')" prop="code">
                <el-input
                  v-model="projectForm.code"
                  :placeholder="t('admin.projects.form.codePlaceholder')"
                />
              </el-form-item>
              <el-form-item :label="t('admin.projects.form.name')" prop="name">
                <el-input
                  v-model="projectForm.name"
                  :placeholder="t('admin.projects.form.namePlaceholder')"
                />
              </el-form-item>
              <el-form-item
                :label="t('admin.projects.form.description')"
                prop="description"
              >
                <el-input
                  v-model="projectForm.description"
                  type="textarea"
                  :rows="3"
                  :placeholder="t('admin.projects.form.descriptionPlaceholder')"
                />
              </el-form-item>
              <el-form-item
                :label="t('admin.projects.form.initialAdmin')"
                prop="initial_admin_user_id"
              >
                <el-select
                  v-model="projectForm.initial_admin_user_id"
                  filterable
                  remote
                  reserve-keyword
                  clearable
                  style="width: 100%"
                  :placeholder="
                    t('admin.projects.form.initialAdminPlaceholder')
                  "
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
                    <div class="setup-page__option">
                      <span>{{ option.username }}</span>
                      <el-tag
                        v-if="option.is_platform_admin"
                        size="small"
                        type="warning"
                      >
                        {{ t("admin.users.platformAdmin") }}
                      </el-tag>
                    </div>
                  </el-option>
                </el-select>
              </el-form-item>
              <el-button
                type="primary"
                :loading="creatingProject"
                @click="handleCreateProject"
              >
                {{ t("setup.wizard.project.create") }}
              </el-button>
            </el-form>
          </el-card>

          <el-card class="setup-page__section" shadow="never">
            <template #header>
              {{ t("setup.wizard.complete.title") }}
            </template>
            <p class="setup-page__hint">
              {{ t("setup.wizard.complete.description") }}
            </p>
            <el-button
              type="primary"
              :loading="completing"
              :disabled="!canCompleteSetup"
              @click="handleCompleteSetup"
            >
              {{ t("setup.actions.completeSetup") }}
            </el-button>
          </el-card>
        </template>

        <div class="setup-page__actions">
          <el-button
            v-if="showLegacyCompleteAction"
            type="primary"
            :loading="completing"
            @click="handleCompleteSetup"
          >
            {{ t("setup.actions.completeSetup") }}
          </el-button>
          <el-button type="primary" @click="goToLogin">
            {{ t("setup.actions.goToLogin") }}
          </el-button>
          <el-button v-if="!setupStatus.setupRequired" @click="goToProjects">
            {{ t("setup.actions.goToProjects") }}
          </el-button>
        </div>
      </template>

      <div v-else class="setup-page__loading">
        {{ t("state.loading") }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import type { FormInstance, FormRules } from "element-plus";
import { ElMessage } from "element-plus";
import { isApiError } from "@/api/error";
import * as adminProjectsApi from "@/api/admin-projects";
import * as adminUsersApi from "@/api/admin-users";
import * as setupApi from "@/api/setup";
import type {
  AdminProjectCreateRequest,
  AdminProjectCreateResponse,
} from "@/api/types/admin-project";
import type {
  AdminUserCreateRequest,
  AdminUserSummary,
} from "@/api/types/admin-user";
import { useAuthSession } from "@/modules/auth/composables/useAuthSession";
import PageHeader from "@/shared/components/PageHeader.vue";
import { getErrorMessage } from "@/shared/constants/error-messages";
import { ROUTE_NAMES } from "@/shared/constants/routes";
import { useI18nText } from "@/shared/i18n";
import { useSetupStatus } from "../composables/useSetupStatus";

const router = useRouter();
const { t } = useI18nText();
const setupStatus = useSetupStatus();
const authSession = useAuthSession();
const completing = ref(false);
const creatingAdmin = ref(false);
const creatingProject = ref(false);
const adminSearchLoading = ref(false);
const adminFormRef = ref<FormInstance>();
const projectFormRef = ref<FormInstance>();
const createdProjectAdmin = ref<AdminUserSummary | null>(null);
const createdProject = ref<AdminProjectCreateResponse | null>(null);
const initialAdminOptions = ref<AdminUserSummary[]>([]);
let adminSearchRequestSeq = 0;

const showWizard = computed(
  () => setupStatus.setupRequired && authSession.isPlatformAdmin,
);

const showLegacyCompleteAction = computed(
  () =>
    setupStatus.setupRequired &&
    authSession.isPlatformAdmin &&
    !showWizard.value,
);

const wizardStep = computed(() => {
  if (!createdProjectAdmin.value) return 0;
  if (!createdProject.value && setupStatus.status?.project_count === 0)
    return 1;
  return 2;
});

const canCompleteSetup = computed(
  () =>
    Boolean(createdProject.value) ||
    (setupStatus.status?.project_count ?? 0) > 0,
);

function createInitialAdminForm(): AdminUserCreateRequest {
  return {
    username: "",
    password: "",
    status: "active",
    is_platform_admin: false,
    must_change_password: true,
  };
}

interface ProjectFormState {
  code: string;
  name: string;
  description: string;
  initial_admin_user_id: number | null;
}

function createInitialProjectForm(): ProjectFormState {
  return {
    code: "",
    name: "",
    description: "",
    initial_admin_user_id: null,
  };
}

const adminForm = ref<AdminUserCreateRequest>(createInitialAdminForm());
const projectForm = ref<ProjectFormState>(createInitialProjectForm());

const adminRules: FormRules<AdminUserCreateRequest> = {
  username: [
    {
      required: true,
      message: t("validation.login.usernameRequired"),
      trigger: "blur",
    },
    {
      min: 3,
      message: t("validation.admin.usernameTooShort"),
      trigger: "blur",
    },
  ],
  password: [
    {
      required: true,
      message: t("validation.login.passwordRequired"),
      trigger: "blur",
    },
    {
      min: 8,
      message: t("validation.admin.passwordTooShort"),
      trigger: "blur",
    },
  ],
};

const projectRules: FormRules<ProjectFormState> = {
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

onMounted(async () => {
  if (!setupStatus.checked) {
    await setupStatus.checkStatus();
  }

  if (showWizard.value) {
    void searchInitialAdminOptions("");
  }
});

function goToLogin() {
  router.push({ name: ROUTE_NAMES.LOGIN });
}

function goToProjects() {
  router.push({ name: ROUTE_NAMES.PROJECTS });
}

async function handleCompleteSetup() {
  if (!canCompleteSetup.value) {
    ElMessage.warning(t("setup.wizard.complete.projectRequired"));
    return;
  }

  try {
    completing.value = true;
    const response = await setupApi.completeSetup();
    setupStatus.status = response;
    setupStatus.checked = true;
    ElMessage.success(t("setup.completedToast"));
  } catch (err) {
    if (isApiError(err)) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("setup.completeFailed"));
    }
  } finally {
    completing.value = false;
  }
}

async function handleCreateProjectAdmin() {
  const valid = await adminFormRef.value?.validate().catch(() => false);
  if (!valid) return;

  creatingAdmin.value = true;
  try {
    const response = await adminUsersApi.createAdminUser({
      username: adminForm.value.username.trim(),
      password: adminForm.value.password,
      status: "active",
      is_platform_admin: false,
      must_change_password: adminForm.value.must_change_password,
    });

    createdProjectAdmin.value = response;
    projectForm.value.initial_admin_user_id = response.id;
    initialAdminOptions.value = [
      response,
      ...initialAdminOptions.value.filter((item) => item.id !== response.id),
    ];
    adminForm.value = createInitialAdminForm();
    adminFormRef.value?.clearValidate();
    ElMessage.success(t("setup.wizard.admin.createdToast"));
  } catch (err) {
    if (isApiError(err)) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("admin.users.createFailed"));
    }
  } finally {
    creatingAdmin.value = false;
  }
}

async function handleCreateProject() {
  const valid = await projectFormRef.value?.validate().catch(() => false);
  if (!valid || projectForm.value.initial_admin_user_id == null) return;

  const payload: AdminProjectCreateRequest = {
    code: projectForm.value.code.trim(),
    name: projectForm.value.name.trim(),
    description: projectForm.value.description.trim() || undefined,
    initial_admin_user_id: projectForm.value.initial_admin_user_id,
  };

  creatingProject.value = true;
  try {
    const response = await adminProjectsApi.createAdminProject(payload);
    createdProject.value = response;
    if (setupStatus.status) {
      setupStatus.status = {
        ...setupStatus.status,
        project_count: setupStatus.status.project_count + 1,
      };
    }
    ElMessage.success(t("admin.projects.created"));
  } catch (err) {
    if (isApiError(err)) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("admin.projects.createFailed"));
    }
  } finally {
    creatingProject.value = false;
  }
}

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

function formatOptionalDate(value: string | null) {
  if (!value) {
    return "-";
  }

  return new Date(value).toLocaleString();
}
</script>

<style scoped>
.setup-page {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-lg);
  background: var(--color-bg-page);
}

.setup-page__card {
  width: 100%;
  max-width: 720px;
  padding: var(--spacing-xl);
  background: var(--color-bg-card);
  border-radius: var(--border-radius-lg);
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.08);
}

.setup-page__alert {
  margin-bottom: var(--spacing-lg);
}

.setup-page__steps {
  margin: var(--spacing-xl) 0;
}

.setup-page__section {
  margin-top: var(--spacing-lg);
}

.setup-page__section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-sm);
  font-weight: 600;
}

.setup-page__hint {
  margin-top: 0;
  color: var(--color-text-secondary);
  line-height: 1.7;
}

.setup-page__option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-sm);
}

.setup-page__actions {
  display: flex;
  gap: var(--spacing-sm);
  justify-content: flex-end;
  margin-top: var(--spacing-lg);
}

.setup-page__loading {
  color: var(--color-text-secondary);
}

@media (max-width: 768px) {
  .setup-page__actions {
    flex-direction: column;
  }
}
</style>
