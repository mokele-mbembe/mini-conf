<template>
  <div class="draft-workspace-summary">
    <div class="draft-workspace-summary__meta">
      <strong class="draft-workspace-summary__name">
        {{ deployment.name }}
      </strong>
      <span class="draft-workspace-summary__code">
        {{ deployment.deployment_key }}
      </span>
      <el-tag size="small" type="info">
        {{
          deployment.is_template
            ? t("deployments.type.template")
            : t("deployments.type.instance")
        }}
      </el-tag>
      <span class="draft-workspace-summary__divider" />
      <span class="draft-workspace-summary__code">
        {{ configFile.code }}
      </span>
      <el-tag size="small" type="info">
        {{ configFile.format }}
      </el-tag>
      <span class="draft-workspace-summary__version">
        {{ t("drafts.field.version") }} {{ versionLabel }}
      </span>
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
.draft-workspace-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--spacing-md);
  margin-bottom: var(--spacing-md);
  padding: 10px 0;
  border-top: 1px solid var(--color-border-light);
  border-bottom: 1px solid var(--color-border-light);
}

.draft-workspace-summary__meta {
  display: flex;
  min-width: 0;
  flex: 1;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px 10px;
  color: var(--el-text-color-regular);
  font-size: 13px;
}

.draft-workspace-summary__name {
  max-width: 220px;
  overflow: hidden;
  color: var(--el-text-color-primary);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.draft-workspace-summary__code {
  font-family: monospace;
}

.draft-workspace-summary__divider {
  width: 1px;
  height: 16px;
  background: var(--color-border-light);
}

.draft-workspace-summary__version {
  color: var(--el-text-color-secondary);
}

.draft-editor-page__draft-actions {
  display: flex;
  flex-shrink: 0;
  flex-wrap: wrap;
  gap: var(--spacing-sm);
  justify-content: flex-end;
}

@media (max-width: 768px) {
  .draft-workspace-summary {
    align-items: flex-start;
    flex-direction: column;
  }

  .draft-editor-page__draft-actions {
    justify-content: flex-start;
  }
}
</style>
