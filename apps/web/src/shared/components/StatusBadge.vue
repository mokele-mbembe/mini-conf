<template>
  <el-tag :type="tagType" size="small" effect="light">{{ label }}</el-tag>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18nText } from "@/shared/i18n";

const props = defineProps<{
  status: string;
}>();

const { t } = useI18nText();

const statusMap: Record<
  string,
  { type: "" | "success" | "warning" | "danger" | "info"; labelKey: string }
> = {
  active: { type: "success", labelKey: "status.active" },
  inactive: { type: "info", labelKey: "status.inactive" },
  archived: { type: "info", labelKey: "status.archived" },
  deprecated: { type: "warning", labelKey: "status.deprecated" },
};

const tagType = computed(() => statusMap[props.status]?.type ?? "info");
const label = computed(() => {
  const key = statusMap[props.status]?.labelKey;
  return key ? t(key) : props.status;
});
</script>
