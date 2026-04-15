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
import { ROUTE_NAMES } from "@/shared/constants/routes";
import { useI18nText } from "@/shared/i18n";

const router = useRouter();
const route = useRoute();
const { t } = useI18nText();

interface Tab {
  name: string;
  label: string;
  routeName?: string;
}

const tabs = computed<Tab[]>(() => [
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
    name: "deployments",
    label: t("tabs.deployments"),
    routeName: ROUTE_NAMES.DEPLOYMENT_LIST,
  },
  {
    name: "releases",
    label: t("tabs.releases"),
    routeName: ROUTE_NAMES.RELEASE_LIST,
  },
  {
    name: "members",
    label: t("tabs.members"),
    routeName: ROUTE_NAMES.PROJECT_MEMBERS,
  },
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
  {
    name: "audit-logs",
    label: t("tabs.auditLogs"),
    routeName: ROUTE_NAMES.AUDIT_LOG_LIST,
  },
]);

const activeTab = computed(() => {
  const currentRouteName = route.name as string;
  if (currentRouteName === ROUTE_NAMES.DEPLOYMENT_DETAIL) {
    return "deployments";
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
