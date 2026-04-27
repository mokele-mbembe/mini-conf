<template>
  <div class="config-workspace-layout">
    <main class="config-workspace-layout__main">
      <slot name="main" />
    </main>
    <aside v-if="$slots.aside" class="config-workspace-layout__aside">
      <slot name="aside" />
    </aside>
  </div>
</template>

<style scoped>
.config-workspace-layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 320px;
  gap: var(--spacing-md);
}

.config-workspace-layout__main {
  min-width: 0;
}

.config-workspace-layout__aside {
  min-width: 0;
}

@media (max-width: 1200px) {
  .config-workspace-layout {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
