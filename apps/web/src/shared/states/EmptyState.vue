<template>
  <div class="empty-state">
    <el-empty :description="description || t('state.empty.description')">
      <slot />
    </el-empty>
  </div>
</template>

<script setup lang="ts">
import { useI18nText } from "@/shared/i18n";

const { t } = useI18nText();

defineProps<{ description?: string }>();
</script>

<style scoped>
.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-xl) 0;
  min-height: 200px;
}
</style>
