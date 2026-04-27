<template>
  <section class="draft-saved-versions-panel">
    <div class="draft-saved-versions-panel__header">
      <div class="draft-saved-versions-panel__title">
        {{ t("savedVersions.panel.title") }}
      </div>
      <el-tag size="small" type="info">
        {{ savedVersions.length }}
      </el-tag>
    </div>

    <el-alert
      v-if="error"
      :title="t('savedVersions.error.loadList')"
      type="error"
      :description="getErrorMessage(error.code, error.message)"
      show-icon
      :closable="false"
      class="draft-saved-versions-panel__alert"
    />

    <div v-else-if="loading" class="draft-saved-versions-panel__loading">
      <el-skeleton :rows="4" animated />
    </div>

    <el-empty
      v-else-if="savedVersions.length === 0"
      :description="t('savedVersions.empty')"
    />

    <template v-else>
      <div class="draft-saved-versions-panel__list">
        <button
          v-for="item in savedVersions"
          :key="item.id"
          type="button"
          class="draft-saved-versions-panel__item draft-editor-page__history-item"
          :class="{ 'is-active': selectedSavedVersionId === item.id }"
          @click="$emit('select', item.id)"
        >
          <div class="draft-saved-versions-panel__item-title">
            {{ item.title }}
          </div>
          <div class="draft-saved-versions-panel__item-meta">
            <span>
              {{ t("savedVersions.field.version") }}
              {{ item.source_draft_version }}
            </span>
            <span>{{ item.created_by_username }}</span>
          </div>
        </button>
      </div>

      <el-divider />

      <div
        class="draft-saved-versions-panel__detail draft-editor-page__history-detail"
      >
        <el-skeleton v-if="detailLoading" :rows="4" animated />

        <template v-else-if="savedVersionDetail">
          <el-descriptions :column="1" border size="small">
            <el-descriptions-item :label="t('savedVersions.field.title')">
              {{ savedVersionDetail.title }}
            </el-descriptions-item>
            <el-descriptions-item :label="t('savedVersions.field.createdAt')">
              {{ savedVersionDetail.created_at }}
            </el-descriptions-item>
            <el-descriptions-item :label="t('savedVersions.field.author')">
              {{ savedVersionDetail.created_by_username }}
            </el-descriptions-item>
          </el-descriptions>

          <div
            class="draft-saved-versions-panel__note draft-editor-page__history-note"
          >
            <div class="draft-saved-versions-panel__note-header">
              <span>{{ t("savedVersions.field.note") }}</span>
              <span>{{ note.length }}/{{ noteMaxLength }}</span>
            </div>
            <el-input
              :model-value="note"
              type="textarea"
              :rows="3"
              :maxlength="noteMaxLength"
              :placeholder="t('savedVersions.note.placeholder')"
              @update:model-value="$emit('update:note', $event)"
            />
          </div>

          <div class="draft-saved-versions-panel__actions">
            <el-button
              size="small"
              :loading="updatingNote"
              @click="$emit('save-note')"
            >
              {{ t("savedVersions.action.saveNote") }}
            </el-button>
            <el-button
              size="small"
              type="warning"
              :loading="restoring"
              @click="$emit('restore')"
            >
              {{ t("savedVersions.action.restore") }}
            </el-button>
            <el-button
              size="small"
              type="danger"
              :loading="deleting"
              @click="$emit('delete')"
            >
              {{ t("savedVersions.action.delete") }}
            </el-button>
          </div>
        </template>
      </div>
    </template>
  </section>
</template>

<script setup lang="ts">
import { useI18nText } from "@/shared/i18n";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type { ApiRequestError } from "@/api/error";
import type {
  SavedVersionDetail,
  SavedVersionSummary,
} from "@/api/types/saved-version";

defineProps<{
  savedVersions: SavedVersionSummary[];
  loading: boolean;
  error: ApiRequestError | null;
  selectedSavedVersionId: number | null;
  savedVersionDetail: SavedVersionDetail | null;
  detailLoading: boolean;
  note: string;
  noteMaxLength: number;
  updatingNote: boolean;
  restoring: boolean;
  deleting: boolean;
}>();

defineEmits<{
  select: [savedVersionId: number];
  "update:note": [note: string];
  "save-note": [];
  restore: [];
  delete: [];
}>();

const { t } = useI18nText();
</script>

<style scoped>
.draft-saved-versions-panel {
  border: 1px solid var(--el-border-color-light);
  border-radius: var(--el-border-radius-base);
  padding: var(--spacing-sm);
  background: var(--el-bg-color-page);
  min-height: 560px;
}

.draft-saved-versions-panel__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--spacing-sm);
}

.draft-saved-versions-panel__title {
  font-weight: 600;
}

.draft-saved-versions-panel__alert {
  margin-bottom: var(--spacing-sm);
}

.draft-saved-versions-panel__list {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-xs, 6px);
  max-height: 240px;
  overflow: auto;
}

.draft-saved-versions-panel__item {
  border: 1px solid var(--el-border-color-light);
  background: var(--el-bg-color);
  border-radius: var(--el-border-radius-base);
  padding: 8px;
  text-align: left;
  cursor: pointer;
}

.draft-saved-versions-panel__item.is-active {
  border-color: var(--el-color-primary);
  box-shadow: 0 0 0 1px var(--el-color-primary-light-5) inset;
}

.draft-saved-versions-panel__item-title {
  font-size: 13px;
  font-weight: 600;
}

.draft-saved-versions-panel__item-meta {
  margin-top: 4px;
  color: var(--el-text-color-secondary);
  font-size: 12px;
  display: flex;
  gap: 8px;
}

.draft-saved-versions-panel__detail {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-sm);
}

.draft-saved-versions-panel__note {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.draft-saved-versions-panel__note-header {
  display: flex;
  justify-content: space-between;
  color: var(--el-text-color-secondary);
  font-size: 12px;
}

.draft-saved-versions-panel__actions {
  display: flex;
  gap: var(--spacing-xs, 6px);
  flex-wrap: wrap;
}
</style>
