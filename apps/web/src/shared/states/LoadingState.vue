<template>
  <div class="loading-state">
    <el-icon class="loading-state__icon" :size="40"><Loading /></el-icon>
    <p v-if="resolvedText" class="loading-state__text">{{ resolvedText }}</p>
  </div>
</template>

<script setup lang="ts">
import { Loading } from "@element-plus/icons-vue";
import { computed } from "vue";
import { useI18nText } from "@/shared/i18n";

const props = defineProps<{ text?: string }>();
const { t } = useI18nText();
const resolvedText = computed(() => props.text ?? t("state.loading"));
</script>

<style scoped>
.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--spacing-xl) 0;
  min-height: 200px;
}
.loading-state__icon {
  color: var(--color-primary);
  animation: spin 1s linear infinite;
}
.loading-state__text {
  margin-top: var(--spacing-md);
  color: var(--color-text-secondary);
}
@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}
</style>
