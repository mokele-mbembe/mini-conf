<template>
  <el-container class="admin-layout">
    <el-header class="admin-layout__header">
      <div class="admin-layout__brand" @click="goAdminHome">mini-conf</div>
      <span class="admin-layout__title">{{ t("admin.title") }}</span>
      <div class="admin-layout__user">
        <LocaleSelect class-name="admin-layout__locale" />
        <span v-if="authSession.user">{{ authSession.user.username }}</span>
        <el-button text @click="handleLogout">{{ t("app.logout") }}</el-button>
      </div>
    </el-header>
    <el-container class="admin-layout__content">
      <el-aside class="admin-layout__sidebar" width="240px">
        <nav class="admin-layout__nav">
          <router-link
            :to="{ name: ROUTE_NAMES.ADMIN_USERS }"
            :class="[
              'admin-layout__nav-item',
              {
                'admin-layout__nav-item--active': isActive(
                  ROUTE_NAMES.ADMIN_USERS,
                ),
              },
            ]"
          >
            {{ t("admin.users") }}
          </router-link>
          <router-link
            :to="{ name: ROUTE_NAMES.ADMIN_PROJECTS }"
            :class="[
              'admin-layout__nav-item',
              {
                'admin-layout__nav-item--active':
                  isActive(ROUTE_NAMES.ADMIN_PROJECTS) ||
                  isActive(ROUTE_NAMES.ADMIN_CREATE_PROJECT),
              },
            ]"
          >
            {{ t("admin.projects") }}
          </router-link>
          <router-link
            :to="{ name: ROUTE_NAMES.PROJECTS }"
            class="admin-layout__nav-item admin-layout__nav-item--back"
          >
            {{ t("admin.backToProjects") }}
          </router-link>
        </nav>
      </el-aside>
      <el-main class="admin-layout__main">
        <router-view />
      </el-main>
    </el-container>
  </el-container>
</template>

<script setup lang="ts">
import { useRouter, useRoute } from "vue-router";
import { useAuthSession } from "@/modules/auth/composables/useAuthSession";
import { ROUTE_NAMES } from "@/shared/constants/routes";
import { useI18nText } from "@/shared/i18n";
import LocaleSelect from "@/shared/components/LocaleSelect.vue";

const router = useRouter();
const route = useRoute();
const authSession = useAuthSession();
const { t } = useI18nText();

function goAdminHome() {
  router.push({ name: ROUTE_NAMES.ADMIN_USERS });
}

function isActive(routeName: string): boolean {
  return route.name === routeName;
}

async function handleLogout() {
  await authSession.logout();
  router.replace({ name: ROUTE_NAMES.LOGIN });
}
</script>

<style scoped>
.admin-layout {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

.admin-layout__header {
  display: flex;
  align-items: center;
  gap: var(--spacing-lg);
  height: var(--header-height);
  padding: 0 var(--spacing-lg);
  background: var(--color-bg-card);
  border-bottom: 1px solid var(--color-border-light);
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.04);
}

.admin-layout__brand {
  font-size: var(--font-size-lg);
  font-weight: 700;
  color: var(--color-primary);
  cursor: pointer;
  user-select: none;
  flex-shrink: 0;
}

.admin-layout__title {
  font-size: var(--font-size-base);
  color: var(--color-text-secondary);
  flex: 1;
}

.admin-layout__user {
  display: flex;
  align-items: center;
  gap: var(--spacing-sm);
  font-size: var(--font-size-base);
  color: var(--color-text-regular);
}

.admin-layout__locale {
  width: 116px;
}

.admin-layout__content {
  flex: 1;
  overflow: hidden;
}

.admin-layout__sidebar {
  background: var(--color-bg-page);
  border-right: 1px solid var(--color-border-light);
  overflow-y: auto;
  padding: var(--spacing-md);
}

.admin-layout__nav {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs);
}

.admin-layout__nav-item {
  display: block;
  padding: var(--spacing-sm) var(--spacing-md);
  border-radius: var(--border-radius-md);
  color: var(--color-text-regular);
  text-decoration: none;
  transition: all 0.2s ease;

  &:hover {
    background: var(--color-bg-hover);
    color: var(--color-text-primary);
  }
}

.admin-layout__nav-item--active {
  background: var(--color-primary-light);
  color: var(--color-primary);
  font-weight: 500;
}

.admin-layout__nav-item--back {
  margin-top: var(--spacing-md);
  padding-top: var(--spacing-md);
  border-top: 1px solid var(--color-border-light);
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
}

.admin-layout__main {
  padding: var(--spacing-lg);
  overflow-y: auto;
}
</style>
