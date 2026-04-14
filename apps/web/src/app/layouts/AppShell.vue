<template>
  <el-container class="app-shell">
    <el-header class="app-shell__header">
      <div class="app-shell__brand" @click="goHome">mini-conf</div>
      <div class="app-shell__user">
        <LocaleSelect class-name="app-shell__locale" />
        <span v-if="authSession.user">{{ authSession.user.username }}</span>
        <el-button text @click="handleLogout">{{ t("app.logout") }}</el-button>
      </div>
    </el-header>
    <el-main class="app-shell__main">
      <router-view />
    </el-main>
  </el-container>
</template>

<script setup lang="ts">
import { useRouter } from "vue-router";
import { useAuthSession } from "@/modules/auth/composables/useAuthSession";
import { ROUTE_NAMES } from "@/shared/constants/routes";
import { useI18nText } from "@/shared/i18n";
import LocaleSelect from "@/shared/components/LocaleSelect.vue";

const router = useRouter();
const authSession = useAuthSession();
const { t } = useI18nText();

function goHome() {
  router.push({ name: ROUTE_NAMES.PROJECTS });
}

async function handleLogout() {
  await authSession.logout();
  router.replace({ name: ROUTE_NAMES.LOGIN });
}
</script>

<style scoped>
.app-shell {
  min-height: 100vh;
}
.app-shell__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: var(--header-height);
  padding: 0 var(--spacing-lg);
  background: var(--color-bg-card);
  border-bottom: 1px solid var(--color-border-light);
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.04);
}
.app-shell__brand {
  font-size: var(--font-size-lg);
  font-weight: 700;
  color: var(--color-primary);
  cursor: pointer;
  user-select: none;
}
.app-shell__user {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  font-size: var(--font-size-base);
  color: var(--color-text-regular);
}
.app-shell__locale {
  width: 116px;
}
.app-shell__main {
  padding: 0;
}
</style>
