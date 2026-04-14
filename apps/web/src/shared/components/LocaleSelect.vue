<template>
  <el-select
    :model-value="locale"
    size="small"
    :class="selectClass"
    @change="handleChange"
  >
    <el-option :label="t('app.locale.zh-CN')" value="zh-CN" />
    <el-option :label="t('app.locale.en-US')" value="en-US" />
  </el-select>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { setLocale, type AppLocale, useI18nText } from "@/shared/i18n";

const props = withDefaults(
  defineProps<{
    className?: string;
  }>(),
  {
    className: "",
  },
);

const { locale, t } = useI18nText();
const selectClass = computed(() => props.className);

function handleChange(value: AppLocale) {
  setLocale(value);
}
</script>
