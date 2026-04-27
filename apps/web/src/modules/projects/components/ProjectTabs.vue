<template>
  <div class="project-tabs">
    <el-tabs :model-value="activeTab" @tab-click="handleTabClick">
      <el-tab-pane
        v-for="tab in tabs"
        :key="tab.name"
        :label="tab.label"
        :name="tab.name"
      />
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useRouter, useRoute } from "vue-router";
import type { TabsPaneContext } from "element-plus";
import { useProjectContext } from "@/modules/projects/composables/useProjectContext";
import { ROUTE_NAMES } from "@/shared/constants/routes";
import { useI18nText } from "@/shared/i18n";

const router = useRouter();
const route = useRoute();
const { t } = useI18nText();
const { project } = useProjectContext();

interface Tab {
  name: string;
  label: string;
  routeName?: string;
}

const isAdmin = computed(() => project.value?.current_user_role === "admin");

const tabs = computed<Tab[]>(() => {
  const visibleTabs: Tab[] = [
    {
      name: "overview",
      label: t("tabs.overview"),
      routeName: ROUTE_NAMES.PROJECT_OVERVIEW,
    },
    {
      name: "config-files",
      label: t("tabs.configFiles"),
      routeName: ROUTE_NAMES.CONFIG_FILE_LIST,
    },
    {
      name: "environments",
      label: t("tabs.environments"),
      routeName: ROUTE_NAMES.PROJECT_ENVIRONMENT_LIST,
    },
    {
      name: "deployments",
      label: t("tabs.deployments"),
      routeName: ROUTE_NAMES.DEPLOYMENT_LIST,
    },
    {
      name: "releases",
      label: t("tabs.releases"),
      routeName: ROUTE_NAMES.RELEASE_LIST,
    },
  ];

  if (isAdmin.value) {
    visibleTabs.push({
      name: "members",
      label: t("tabs.members"),
      routeName: ROUTE_NAMES.PROJECT_MEMBERS,
    });
  }

  visibleTabs.push(
    {
      name: "sync-records",
      label: t("tabs.syncRecords"),
      routeName: ROUTE_NAMES.SYNC_RECORD_LIST,
    },
    {
      name: "heartbeats",
      label: t("tabs.heartbeats"),
      routeName: ROUTE_NAMES.HEARTBEAT_LIST,
    },
  );

  if (isAdmin.value) {
    visibleTabs.push({
      name: "audit-logs",
      label: t("tabs.auditLogs"),
      routeName: ROUTE_NAMES.AUDIT_LOG_LIST,
    });
  }

  return visibleTabs;
});

const activeTab = computed(() => {
  const currentRouteName = route.name as string;
  if (
    currentRouteName === ROUTE_NAMES.DEPLOYMENT_DETAIL ||
    currentRouteName === ROUTE_NAMES.DEPLOYMENT_PREVIEW ||
    currentRouteName === ROUTE_NAMES.DRAFT_EDITOR
  ) {
    return "deployments";
  }
  if (
    currentRouteName === ROUTE_NAMES.RELEASE_DETAIL ||
    currentRouteName === ROUTE_NAMES.RELEASE_DIFF
  ) {
    return "releases";
  }

  const tab = tabs.value.find((t) => t.routeName === currentRouteName);
  return tab?.name ?? "overview";
});

function handleTabClick(pane: TabsPaneContext) {
  const tabName = pane.paneName as string;
  const tab = tabs.value.find((t) => t.name === tabName);
  if (tab?.routeName) {
    router.push({
      name: tab.routeName,
      params: { projectId: route.params.projectId },
    });
  }
}
</script>

<style scoped>
.project-tabs {
  margin-bottom: var(--spacing-lg);
}
</style>
