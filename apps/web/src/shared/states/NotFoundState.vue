<template>
  <div class="not-found-state">
    <el-result
      icon="warning"
      :title="title || t('state.notFound.title')"
      :sub-title="subtitle || t('state.notFound.subtitle')"
    >
      <template #extra>
        <el-button type="primary" @click="$router.back()">
          {{ t("state.back") }}
        </el-button>
      </template>
    </el-result>
  </div>
</template>

<script setup lang="ts">
import { useI18nText } from "@/shared/i18n";

const { t } = useI18nText();

defineProps<{
  title?: string;
  subtitle?: string;
}>();
</script>

<style scoped>
.not-found-state {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-xl) 0;
  min-height: 200px;
}
</style>
