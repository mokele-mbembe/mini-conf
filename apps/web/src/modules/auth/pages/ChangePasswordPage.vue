<template>
  <div class="change-password-page">
    <div class="change-password-page__card">
      <h2 class="change-password-page__title">
        {{ t("changePassword.title") }}
      </h2>
      <p class="change-password-page__subtitle">
        {{ t("changePassword.subtitle") }}
      </p>

      <el-alert
        v-if="errorMsg"
        :title="errorMsg"
        type="error"
        show-icon
        :closable="false"
        class="change-password-page__alert"
      />

      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-position="top"
        @submit.prevent="handleSubmit"
      >
        <el-form-item
          :label="t('changePassword.currentPassword')"
          prop="currentPassword"
        >
          <el-input
            v-model="form.currentPassword"
            type="password"
            :placeholder="t('changePassword.currentPasswordPlaceholder')"
            show-password
            autocomplete="current-password"
          />
        </el-form-item>
        <el-form-item
          :label="t('changePassword.newPassword')"
          prop="newPassword"
        >
          <el-input
            v-model="form.newPassword"
            type="password"
            :placeholder="t('changePassword.newPasswordPlaceholder')"
            show-password
            autocomplete="new-password"
          />
        </el-form-item>
        <el-form-item
          :label="t('changePassword.confirmPassword')"
          prop="confirmPassword"
        >
          <el-input
            v-model="form.confirmPassword"
            type="password"
            :placeholder="t('changePassword.confirmPasswordPlaceholder')"
            show-password
            autocomplete="new-password"
            @keyup.enter="handleSubmit"
          />
        </el-form-item>
        <el-form-item>
          <el-button
            type="primary"
            native-type="submit"
            :loading="submitting"
            class="change-password-page__submit"
          >
            {{ t("changePassword.submit") }}
          </el-button>
        </el-form-item>
      </el-form>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref } from "vue";
import { useRouter } from "vue-router";
import type { FormInstance, FormRules } from "element-plus";
import { ElMessage } from "element-plus";
import { isApiError } from "@/api/error";
import { useAuthSession } from "@/modules/auth/composables/useAuthSession";
import { getErrorMessage } from "@/shared/constants/error-messages";
import { ROUTE_NAMES } from "@/shared/constants/routes";
import { useI18nText } from "@/shared/i18n";

interface FormState {
  currentPassword: string;
  newPassword: string;
  confirmPassword: string;
}

const router = useRouter();
const authSession = useAuthSession();
const { t } = useI18nText();
const formRef = ref<FormInstance>();
const submitting = ref(false);
const errorMsg = ref("");

const form = reactive<FormState>({
  currentPassword: "",
  newPassword: "",
  confirmPassword: "",
});

const rules: FormRules<FormState> = {
  currentPassword: [
    {
      required: true,
      message: t("validation.login.passwordRequired"),
      trigger: "blur",
    },
  ],
  newPassword: [
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
  confirmPassword: [
    {
      required: true,
      message: t("changePassword.validation.confirmRequired"),
      trigger: "blur",
    },
    {
      validator: (_rule, value: string, callback) => {
        if (value !== form.newPassword) {
          callback(new Error(t("changePassword.validation.mismatch")));
          return;
        }
        callback();
      },
      trigger: "blur",
    },
  ],
};

async function handleSubmit() {
  const valid = await formRef.value?.validate().catch(() => false);
  if (!valid) return;

  submitting.value = true;
  errorMsg.value = "";
  try {
    await authSession.changePassword(form.currentPassword, form.newPassword);
    ElMessage.success(t("changePassword.success"));
    router.replace(
      authSession.isPlatformAdmin
        ? { name: ROUTE_NAMES.ADMIN_DASHBOARD }
        : { name: ROUTE_NAMES.PROJECTS },
    );
  } catch (err) {
    if (isApiError(err)) {
      errorMsg.value = getErrorMessage(err.code, err.message);
    } else {
      errorMsg.value = getErrorMessage("unknown_error");
    }
  } finally {
    submitting.value = false;
  }
}
</script>

<style scoped>
.change-password-page {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  padding: var(--spacing-md);
  background-color: var(--color-bg-page);
}

.change-password-page__card {
  width: 100%;
  max-width: 440px;
  padding: var(--spacing-xl);
  background: var(--color-bg-card);
  border-radius: var(--border-radius-lg);
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.08);
}

.change-password-page__title {
  margin: 0 0 var(--spacing-xs);
  color: var(--color-text-primary);
  font-size: var(--font-size-xl);
  font-weight: 600;
  text-align: center;
}

.change-password-page__subtitle {
  margin: 0 0 var(--spacing-lg);
  color: var(--color-text-secondary);
  line-height: 1.6;
  text-align: center;
}

.change-password-page__alert {
  margin-bottom: var(--spacing-lg);
}

.change-password-page__submit {
  width: 100%;
}
</style>
