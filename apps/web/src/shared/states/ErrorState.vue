<template>
  <div class="error-state">
    <el-result
      icon="error"
      :title="title || t('state.error.title')"
      :sub-title="subtitle || t('state.error.subtitle')"
    >
      <template #extra>
        <el-button v-if="retryable" type="primary" @click="$emit('retry')">
          {{ t("state.error.retry") }}
        </el-button>
      </template>
    </el-result>
  </div>
</template>

<script setup lang="ts">
import { useI18nText } from "@/shared/i18n";

const { t } = useI18nText();

withDefaults(
  defineProps<{
    title?: string;
    subtitle?: string;
    retryable?: boolean;
  }>(),
  {
    title: "",
    subtitle: "",
    retryable: true,
  },
);

defineEmits<{ retry: [] }>();
</script>

<style scoped>
.error-state {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-xl) 0;
  min-height: 200px;
}
</style>
