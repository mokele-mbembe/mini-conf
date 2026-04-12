<template>
  <div class="error-state">
    <el-result icon="error" :title="title" :sub-title="subtitle">
      <template #extra>
        <el-button v-if="retryable" type="primary" @click="$emit('retry')">
          重试
        </el-button>
      </template>
    </el-result>
  </div>
</template>

<script setup lang="ts">
withDefaults(
  defineProps<{
    title?: string;
    subtitle?: string;
    retryable?: boolean;
  }>(),
  {
    title: "加载失败",
    subtitle: "请稍后重试",
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
