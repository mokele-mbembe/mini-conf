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

        <div class="setup-page__actions">
          <el-button
            v-if="setupStatus.setupRequired && authSession.isPlatformAdmin"
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
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { ElMessage } from "element-plus";
import { isApiError } from "@/api/error";
import * as setupApi from "@/api/setup";
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

onMounted(async () => {
  if (!setupStatus.checked) {
    await setupStatus.checkStatus();
  }
});

function goToLogin() {
  router.push({ name: ROUTE_NAMES.LOGIN });
}

function goToProjects() {
  router.push({ name: ROUTE_NAMES.PROJECTS });
}

async function handleCompleteSetup() {
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
