<template>
  <div class="page-header">
    <div class="page-header__main">
      <h1 class="page-header__title">{{ title }}</h1>
      <p v-if="subtitle" class="page-header__subtitle">{{ subtitle }}</p>
    </div>
    <div v-if="$slots.actions" class="page-header__actions">
      <slot name="actions" />
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  title: string;
  subtitle?: string;
}>();
</script>

<style scoped>
.page-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--spacing-md);
  margin-bottom: var(--spacing-lg);
}
.page-header__title {
  font-size: var(--font-size-xl);
  font-weight: 600;
  line-height: 1.4;
}
.page-header__subtitle {
  margin-top: var(--spacing-xs);
  font-size: var(--font-size-base);
  color: var(--color-text-secondary);
}
.page-header__actions {
  flex-shrink: 0;
}

@media (max-width: 768px) {
  .page-header {
    flex-direction: column;
  }
}
</style>
