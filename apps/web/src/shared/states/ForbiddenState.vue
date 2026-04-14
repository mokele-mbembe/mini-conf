<template>
  <div class="forbidden-state">
    <el-result
      icon="warning"
      :title="t('state.forbidden.title')"
      :sub-title="subtitle || t('state.forbidden.subtitle')"
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

defineProps<{ subtitle?: string }>();
</script>

<style scoped>
.forbidden-state {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-xl) 0;
  min-height: 200px;
}
</style>
