<template>
  <div class="login-page">
    <div class="login-page__card">
      <div class="login-page__toolbar">
        <LocaleSelect class-name="login-page__locale" />
      </div>
      <h2 class="login-page__title">{{ t("login.title") }}</h2>
      <el-alert
        v-if="errorMsg"
        :title="errorMsg"
        type="error"
        show-icon
        :closable="false"
        style="margin-bottom: 20px"
      />
      <LoginForm :loading="submitting" @submit="handleLogin" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import LoginForm from "../components/LoginForm.vue";
import { useAuthSession } from "../composables/useAuthSession";
import { isApiError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import { ROUTE_NAMES } from "@/shared/constants/routes";
import { useI18nText } from "@/shared/i18n";
import LocaleSelect from "@/shared/components/LocaleSelect.vue";

const router = useRouter();
const authSession = useAuthSession();
const { t } = useI18nText();

const submitting = ref(false);
const errorMsg = ref("");

onMounted(async () => {
  const result = await authSession.checkSession();
  if (result === "authenticated") {
    redirectAfterLogin();
  } else if (result === "error") {
    errorMsg.value = authSession.sessionError ?? t("login.sessionCheckFailed");
  }
});

function redirectAfterLogin() {
  const nextRoute = authSession.isPlatformAdmin
    ? { name: ROUTE_NAMES.ADMIN_DASHBOARD }
    : { name: ROUTE_NAMES.PROJECTS };
  router.replace(nextRoute);
}

async function handleLogin(username: string, password: string) {
  submitting.value = true;
  errorMsg.value = "";
  try {
    await authSession.login(username, password);
    redirectAfterLogin();
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
.login-page {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  padding: var(--spacing-md);
  background-color: var(--color-bg-page);
}
.login-page__card {
  width: 100%;
  max-width: 400px;
  padding: var(--spacing-xl);
  background: var(--color-bg-card);
  border-radius: var(--border-radius-lg);
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.08);
}
.login-page__toolbar {
  display: flex;
  justify-content: flex-end;
  margin-bottom: var(--spacing-sm);
}
.login-page__locale {
  width: 116px;
}
.login-page__title {
  font-size: var(--font-size-xl);
  font-weight: 600;
  text-align: center;
  margin-bottom: var(--spacing-lg);
  color: var(--color-text-primary);
}
</style>
