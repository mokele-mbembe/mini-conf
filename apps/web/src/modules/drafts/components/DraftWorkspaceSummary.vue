<template>
  <div class="draft-editor-page__meta">
    <el-descriptions :column="3" border>
      <el-descriptions-item :label="t('deployments.field.name')">
        {{ deployment.name }}
      </el-descriptions-item>
      <el-descriptions-item :label="t('deployments.field.deploymentKey')">
        <span class="draft-editor-page__code">
          {{ deployment.deployment_key }}
        </span>
      </el-descriptions-item>
      <el-descriptions-item :label="t('deployments.field.type')">
        {{
          deployment.is_template
            ? t("deployments.type.template")
            : t("deployments.type.instance")
        }}
      </el-descriptions-item>
      <el-descriptions-item :label="t('configFiles.column.code')">
        <span class="draft-editor-page__code">
          {{ configFile.code }}
        </span>
      </el-descriptions-item>
      <el-descriptions-item :label="t('configFiles.column.format')">
        <el-tag size="small" type="info">
          {{ configFile.format }}
        </el-tag>
      </el-descriptions-item>
      <el-descriptions-item :label="t('drafts.field.version')">
        {{ versionLabel }}
      </el-descriptions-item>
    </el-descriptions>
  </div>

  <div v-if="canEdit" class="draft-editor-page__draft-actions">
    <el-button
      v-if="draftReady"
      text
      type="danger"
      size="small"
      :loading="discarding"
      @click="$emit('discard')"
    >
      {{ t("drafts.action.discard") }}
    </el-button>
    <el-button
      text
      type="primary"
      size="small"
      :loading="restoring"
      @click="$emit('restore-from-release')"
    >
      {{ t("drafts.action.restoreFromRelease") }}
    </el-button>
    <el-button
      text
      type="primary"
      size="small"
      @click="$emit('clone-from-instance')"
    >
      {{ t("drafts.action.cloneFromInstance") }}
    </el-button>
  </div>
</template>

<script setup lang="ts">
import type { ConfigFileSummary } from "@/api/types/config-file";
import type { DeploymentInstanceSummary } from "@/api/types/deployment-instance";
import { useI18nText } from "@/shared/i18n";

defineProps<{
  deployment: DeploymentInstanceSummary;
  configFile: ConfigFileSummary;
  versionLabel: string;
  canEdit: boolean;
  draftReady: boolean;
  discarding: boolean;
  restoring: boolean;
}>();

defineEmits<{
  discard: [];
  "restore-from-release": [];
  "clone-from-instance": [];
}>();

const { t } = useI18nText();
</script>

<style scoped>
.draft-editor-page__draft-actions {
  display: flex;
  gap: var(--spacing-sm);
  margin-bottom: var(--spacing-md);
}

.draft-editor-page__meta {
  margin-bottom: var(--spacing-md);
}

.draft-editor-page__code {
  font-family: monospace;
}
</style>
