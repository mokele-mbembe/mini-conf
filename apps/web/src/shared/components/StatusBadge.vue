<template>
  <el-tag :type="tagType" size="small" effect="light">{{ label }}</el-tag>
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  status: string;
}>();

const statusMap: Record<
  string,
  { type: "" | "success" | "warning" | "danger" | "info"; label: string }
> = {
  active: { type: "success", label: "Active" },
  inactive: { type: "info", label: "Inactive" },
  archived: { type: "info", label: "Archived" },
  deprecated: { type: "warning", label: "Deprecated" },
};

const tagType = computed(() => statusMap[props.status]?.type ?? "info");
const label = computed(() => statusMap[props.status]?.label ?? props.status);
</script>
