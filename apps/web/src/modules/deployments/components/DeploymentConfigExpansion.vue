<template>
  <div class="deployment-config-expansion">
    <div class="deployment-config-expansion__meta">
      <span class="deployment-config-expansion__label">
        {{ t("deployments.expanded.meta") }}
      </span>
      <span class="deployment-config-expansion__code">
        {{ row.deployment_key }}
      </span>
      <span>{{ row.name }}</span>
      <el-tag size="small" type="info">
        {{ row.environment_name }} ({{ row.environment_code }})
      </el-tag>
      <StatusBadge :status="row.status" />
      <span class="deployment-config-expansion__pair">
        {{ t("deployments.expanded.token") }}
        <el-tag
          size="small"
          :type="row.status === 'active' ? 'success' : 'info'"
        >
          {{ tokenStatusLabel }}
        </el-tag>
      </span>
      <span class="deployment-config-expansion__pair">
        {{ t("deployments.expanded.updatedAt") }}
        <span>{{ deploymentUpdatedAt }}</span>
      </span>
    </div>

    <LoadingState
      v-if="detail?.loading"
      class="deployment-config-expansion__state"
    />

    <ErrorState
      v-else-if="detail?.error"
      :title="t('deployments.expanded.configLoadError')"
      :subtitle="errorMessage"
      @retry="$emit('retry')"
    />

    <EmptyState
      v-else-if="detail && detail.configFiles.length === 0"
      :description="t('deployments.configs.empty')"
    />

    <div v-else-if="detail" class="deployment-config-expansion__configs">
      <div class="deployment-config-expansion__label">
        {{ t("deployments.expanded.configs") }}
      </div>
      <div
        v-for="configFile in detail.configFiles"
        :key="configFile.id"
        class="deployment-config-expansion__config-row"
      >
        <div class="deployment-config-expansion__config-main">
          <span class="deployment-config-expansion__code">
            {{ configFile.code }}
          </span>
          <span>{{ configFile.name }}</span>
          <el-tag size="small" type="info">
            {{ configFile.format }}
          </el-tag>
          <el-tag v-if="configFile.is_required" size="small" type="danger">
            {{ t("configFiles.required") }}
          </el-tag>
        </div>

        <div class="deployment-config-expansion__config-hints">
          <el-tag size="small" :type="configStateTagType(configFile.id)">
            {{ configStateLabel(configFile.id) }}
          </el-tag>
          <el-tag size="small" :type="configReadinessTagType(configFile.id)">
            {{ configReadinessLabel(configFile.id) }}
          </el-tag>
          <span class="deployment-config-expansion__muted">
            {{ configSavedVersionsLabel(configFile.id) }}
          </span>
          <span class="deployment-config-expansion__muted">
            {{ configLatestReleaseLabel(configFile.id) }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import StatusBadge from "@/shared/components/StatusBadge.vue";
import LoadingState from "@/shared/states/LoadingState.vue";
import ErrorState from "@/shared/states/ErrorState.vue";
import EmptyState from "@/shared/states/EmptyState.vue";
import { getErrorMessage } from "@/shared/constants/error-messages";
import { useI18nText } from "@/shared/i18n";
import type { ApiRequestError } from "@/api/error";
import type { ConfigFileSummary } from "@/api/types/config-file";
import type {
  DeploymentInstanceSummary,
  DeploymentPreviewItem,
} from "@/api/types/deployment-instance";

interface ConfigHistoryHint {
  savedVersionsCount: number;
  latestReleaseRevision: string | null;
}

interface ExpandedDeploymentDetail {
  loading: boolean;
  error: ApiRequestError | null;
  configFiles: ConfigFileSummary[];
  previewStatusMap: Record<number, DeploymentPreviewItem>;
  configHistoryMap: Record<number, ConfigHistoryHint>;
}

const props = defineProps<{
  row: DeploymentInstanceSummary;
  detail?: ExpandedDeploymentDetail;
}>();

defineEmits<{
  retry: [];
}>();

const { t } = useI18nText();

const errorMessage = computed(() => {
  if (!props.detail?.error) return undefined;
  return getErrorMessage(props.detail.error.code, props.detail.error.message);
});

const tokenStatusLabel = computed(() =>
  props.row.status === "active"
    ? t("deployments.expanded.tokenActive")
    : t("deployments.expanded.tokenInactive"),
);

const deploymentUpdatedAt = computed(() => {
  const value = (
    props.row as DeploymentInstanceSummary & { updated_at?: string }
  ).updated_at;
  if (!value) return t("deployments.expanded.noUpdatedAt");

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString();
});

function previewItem(configFileId: number) {
  return props.detail?.previewStatusMap[configFileId] ?? null;
}

function historyHint(configFileId: number) {
  return props.detail?.configHistoryMap[configFileId] ?? null;
}

function configStateTagType(configFileId: number) {
  const item = previewItem(configFileId);
  if (!item) return "info";
  if (item.source === "draft") return "warning";
  if (item.source === "latest_release") return "success";
  return "info";
}

function configStateLabel(configFileId: number) {
  const item = previewItem(configFileId);
  if (!item) return t("preview.source.none");
  if (item.source === "draft" || item.source === "latest_release") {
    return t(`preview.source.${item.source}`);
  }
  return t("preview.source.none");
}

function configReadinessTagType(configFileId: number) {
  const item = previewItem(configFileId);
  if (!item) return "info";
  if (item.status === "missing_required") return "danger";
  if (item.status === "missing_optional") return "info";
  return "success";
}

function configReadinessLabel(configFileId: number) {
  const item = previewItem(configFileId);
  if (!item) return t("preview.source.none");
  return t(`preview.status.${item.status}`);
}

function configSavedVersionsLabel(configFileId: number) {
  const hint = historyHint(configFileId);
  return t("deployments.configs.savedVersionsCount", {
    count: hint?.savedVersionsCount ?? 0,
  });
}

function configLatestReleaseLabel(configFileId: number) {
  const revision = historyHint(configFileId)?.latestReleaseRevision;
  if (!revision) return t("deployments.configs.noRelease");
  return t("deployments.configs.latestRelease", { revision });
}
</script>

<style scoped>
.deployment-config-expansion {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
  padding: 10px 12px 12px 44px;
  background: var(--el-fill-color-lighter);
}

.deployment-config-expansion__meta,
.deployment-config-expansion__config-row,
.deployment-config-expansion__config-main,
.deployment-config-expansion__config-hints {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
}

.deployment-config-expansion__meta {
  gap: 6px 10px;
  color: var(--el-text-color-regular);
  font-size: 13px;
}

.deployment-config-expansion__label {
  color: var(--el-text-color-secondary);
  font-size: 12px;
  font-weight: 600;
}

.deployment-config-expansion__code {
  font-family: monospace;
  font-size: 0.9em;
}

.deployment-config-expansion__pair {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.deployment-config-expansion__state {
  padding: var(--spacing-sm) 0;
}

.deployment-config-expansion__configs {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.deployment-config-expansion__config-row {
  justify-content: space-between;
  gap: 8px 12px;
  padding: 8px 0;
  border-top: 1px solid var(--color-border-light);
}

.deployment-config-expansion__config-main {
  min-width: 220px;
  flex: 1;
  gap: 6px;
}

.deployment-config-expansion__config-hints {
  min-width: 320px;
  gap: 6px;
  font-size: 12px;
}

.deployment-config-expansion__muted {
  color: var(--el-text-color-secondary);
}

@media (max-width: 768px) {
  .deployment-config-expansion {
    padding-left: 12px;
  }

  .deployment-config-expansion__config-row {
    align-items: flex-start;
    flex-direction: column;
  }

  .deployment-config-expansion__config-hints {
    min-width: 0;
  }
}
</style>
