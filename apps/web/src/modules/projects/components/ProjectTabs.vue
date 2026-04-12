<template>
  <div class="project-tabs">
    <el-tabs v-model="activeTab" @tab-click="handleTabClick">
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
import { ref } from "vue";
import type { TabsPaneContext } from "element-plus";

const tabs = [
  { name: "overview", label: "Overview" },
  { name: "config-files", label: "Config Files" },
  { name: "deployments", label: "Deployments" },
  { name: "releases", label: "Releases" },
  { name: "members", label: "Members" },
  { name: "sync-records", label: "Sync Records" },
  { name: "heartbeats", label: "Heartbeats" },
  { name: "audit-logs", label: "Audit Logs" },
];

const activeTab = ref("overview");

const emit = defineEmits<{
  navigate: [tabName: string];
}>();

function handleTabClick(pane: TabsPaneContext) {
  if (pane.paneName) {
    emit("navigate", pane.paneName as string);
  }
}
</script>

<style scoped>
.project-tabs {
  margin-bottom: var(--spacing-lg);
}
</style>
