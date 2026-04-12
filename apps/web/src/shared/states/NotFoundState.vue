<template>
  <div class="not-found-state">
    <el-result icon="warning" :title="title" :sub-title="subtitle">
      <template #extra>
        <el-button type="primary" @click="$router.back()"> 返回 </el-button>
      </template>
    </el-result>
  </div>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    title?: string;
    subtitle?: string;
  }>(),
  {
    title: "未找到",
    subtitle: "请求的资源不存在或你没有访问权限",
  },
);
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
