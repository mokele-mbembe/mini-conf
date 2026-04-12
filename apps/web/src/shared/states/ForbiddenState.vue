<template>
  <div class="forbidden-state">
    <el-result icon="warning" title="权限不足" :sub-title="subtitle">
      <template #extra>
        <el-button type="primary" @click="$router.back()"> 返回 </el-button>
      </template>
    </el-result>
  </div>
</template>

<script setup lang="ts">
withDefaults(defineProps<{ subtitle?: string }>(), {
  subtitle: "你当前角色没有执行这个操作的权限",
});
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
