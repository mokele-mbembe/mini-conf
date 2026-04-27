<template>
  <div v-if="configFiles.length > 1" class="config-file-switcher">
    <div class="config-file-switcher__label">
      {{ t("drafts.configSwitcher.title") }}
    </div>
    <el-radio-group
      :model-value="currentConfigFileId"
      @update:model-value="handleSwitch"
    >
      <el-radio-button v-for="cf in configFiles" :key="cf.id" :value="cf.id">
        <span>{{ cf.code }}</span>
        <el-tag
          v-if="previewStatusMap[cf.id]"
          size="small"
          :type="configStatusTagType(previewStatusMap[cf.id])"
          class="config-file-switcher__status-tag"
        >
          {{ configStatusLabel(previewStatusMap[cf.id]) }}
        </el-tag>
      </el-radio-button>
    </el-radio-group>
  </div>
</template>

<script setup lang="ts">
import { useI18nText } from "@/shared/i18n";
import type { ConfigFileSummary } from "@/api/types/config-file";

defineProps<{
  configFiles: ConfigFileSummary[];
  currentConfigFileId: number;
  previewStatusMap: Record<number, string>;
}>();

const emit = defineEmits<{
  switch: [configFileId: number];
}>();

const { t } = useI18nText();

function configStatusTagType(source: string) {
  switch (source) {
    case "draft":
      return "warning";
    case "latest_release":
      return "success";
    case "missing_required":
      return "danger";
    default:
      return "info";
  }
}

function configStatusLabel(source: string) {
  switch (source) {
    case "draft":
      return t("preview.source.draft");
    case "latest_release":
      return t("preview.source.latest_release");
    case "missing_required":
      return t("preview.status.missing_required");
    case "missing_optional":
      return t("preview.status.missing_optional");
    default:
      return t("preview.source.none");
  }
}

function handleSwitch(value: string | number | boolean | undefined) {
  if (typeof value !== "number") return;
  emit("switch", value);
}
</script>

<style scoped>
.config-file-switcher {
  margin-bottom: var(--spacing-md);
}

.config-file-switcher__label {
  font-size: 13px;
  color: var(--el-text-color-secondary);
  margin-bottom: var(--spacing-xs, 4px);
}

.config-file-switcher__status-tag {
  margin-left: 4px;
}
</style>
