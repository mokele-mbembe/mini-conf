<template>
  <div class="empty-state">
    <el-empty :description="description">
      <slot />
    </el-empty>
  </div>
</template>

<script setup lang="ts">
withDefaults(defineProps<{ description?: string }>(), {
  description: "暂无数据",
});
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
