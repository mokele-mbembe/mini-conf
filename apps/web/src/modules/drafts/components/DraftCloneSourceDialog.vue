<template>
  <el-dialog
    v-model="visible"
    :title="t('drafts.cloneDialog.title')"
    width="520px"
    destroy-on-close
  >
    <el-alert
      v-if="loadError"
      type="error"
      :closable="false"
      class="draft-clone-source-dialog__alert"
    >
      {{ t("drafts.cloneDialog.loadError") }}
    </el-alert>
    <el-form label-position="top">
      <el-form-item :label="t('drafts.cloneDialog.sourceInstance')">
        <el-select
          v-model="sourceInstanceModel"
          :placeholder="t('drafts.cloneDialog.selectInstance')"
          filterable
          remote
          :remote-method="handleSearch"
          class="draft-clone-source-dialog__select"
          :loading="instancesLoading"
        >
          <el-option
            v-for="src in cloneSources"
            :key="src.deployment_instance_id"
            :label="sourceLabel(src)"
            :value="src.deployment_instance_id"
          >
            <div class="draft-clone-source-dialog__option">
              <span>
                {{ src.name }}
                <span class="draft-clone-source-dialog__muted">
                  ({{ src.deployment_key }})
                </span>
                <el-tag
                  v-if="src.is_template"
                  size="small"
                  type="info"
                  class="draft-clone-source-dialog__template-tag"
                >
                  {{ t("drafts.cloneDialog.templateTag") }}
                </el-tag>
              </span>
              <span class="draft-clone-source-dialog__availability">
                <template v-if="cloneSourceHasNoAvailableSources(src)">
                  {{ t("drafts.cloneDialog.noSources") }}
                </template>
                <template v-else>
                  <span
                    v-if="src.available_sources.draft"
                    class="draft-clone-source-dialog__availability-item"
                  >
                    Draft ✓
                  </span>
                  <span v-if="src.available_sources.latest_release">
                    Release ✓
                  </span>
                </template>
              </span>
            </div>
          </el-option>
          <template v-if="nextCursor" #footer>
            <el-button
              text
              :loading="loadingMore"
              class="draft-clone-source-dialog__load-more"
              @mousedown.prevent
              @click="$emit('load-more')"
            >
              {{ t("drafts.cloneDialog.loadMore") }}
            </el-button>
          </template>
        </el-select>
      </el-form-item>
      <el-form-item :label="t('drafts.cloneDialog.sourceKind')">
        <el-radio-group v-model="sourceKindModel">
          <el-radio value="draft" :disabled="draftOptionDisabled">
            {{ t("drafts.cloneDialog.kindDraft") }}
            <span
              v-if="selectedSourceDraftUnavailable"
              class="draft-clone-source-dialog__muted"
            >
              ({{ t("drafts.cloneDialog.sourceUnavailable") }})
            </span>
          </el-radio>
          <el-radio value="latest_release" :disabled="releaseOptionDisabled">
            {{ t("drafts.cloneDialog.kindRelease") }}
            <span
              v-if="selectedSource?.is_template"
              class="draft-clone-source-dialog__muted"
            >
              ({{ t("drafts.cloneDialog.templateNoRelease") }})
            </span>
            <span
              v-else-if="selectedSourceReleaseUnavailable"
              class="draft-clone-source-dialog__muted"
            >
              ({{ t("drafts.cloneDialog.sourceUnavailable") }})
            </span>
          </el-radio>
        </el-radio-group>
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">
        {{ t("common.cancel") }}
      </el-button>
      <el-button
        type="primary"
        :loading="cloning"
        :disabled="submitDisabled"
        @click="$emit('submit')"
      >
        {{ t("drafts.cloneDialog.submit") }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed } from "vue";
import type { CloneSourceSummary } from "@/api/types/clone-source";
import { useI18nText } from "@/shared/i18n";

export type DraftCloneSourceKind = "draft" | "latest_release";

const props = defineProps<{
  modelValue: boolean;
  cloneSources: CloneSourceSummary[];
  sourceInstanceId: number | null;
  sourceKind: DraftCloneSourceKind;
  loadError: boolean;
  instancesLoading: boolean;
  loadingMore: boolean;
  nextCursor: number | null;
  selectedSource: CloneSourceSummary | undefined;
  draftOptionDisabled: boolean;
  releaseOptionDisabled: boolean;
  selectedSourceDraftUnavailable: boolean;
  selectedSourceReleaseUnavailable: boolean;
  submitDisabled: boolean;
  cloning: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [visible: boolean];
  "update:sourceInstanceId": [sourceInstanceId: number | null];
  "update:sourceKind": [sourceKind: DraftCloneSourceKind];
  search: [keyword: string];
  "load-more": [];
  submit: [];
}>();

const { t } = useI18nText();

const visible = computed({
  get: () => props.modelValue,
  set: (value: boolean) => emit("update:modelValue", value),
});

const sourceInstanceModel = computed({
  get: () => props.sourceInstanceId,
  set: (value: string | number | boolean | null | undefined) => {
    emit("update:sourceInstanceId", typeof value === "number" ? value : null);
  },
});

const sourceKindModel = computed({
  get: () => props.sourceKind,
  set: (value: string | number | boolean | undefined) => {
    if (value === "draft" || value === "latest_release") {
      emit("update:sourceKind", value);
    }
  },
});

function sourceLabel(src: CloneSourceSummary): string {
  const label = `${src.name} (${src.deployment_key})`;
  return src.is_template
    ? `${label} [${t("drafts.cloneDialog.templateTag")}]`
    : label;
}

function cloneSourceHasNoAvailableSources(src: CloneSourceSummary): boolean {
  return !src.available_sources.draft && !src.available_sources.latest_release;
}

function handleSearch(keyword: string) {
  emit("search", keyword);
}
</script>

<style scoped>
.draft-clone-source-dialog__alert {
  margin-bottom: 16px;
}

.draft-clone-source-dialog__select,
.draft-clone-source-dialog__load-more {
  width: 100%;
}

.draft-clone-source-dialog__option {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.draft-clone-source-dialog__muted {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}

.draft-clone-source-dialog__template-tag {
  margin-left: 4px;
}

.draft-clone-source-dialog__availability {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}

.draft-clone-source-dialog__availability-item {
  margin-right: 6px;
}
</style>
