<template>
  <el-form
    ref="formRef"
    :model="form"
    :rules="rules"
    label-position="top"
    @submit.prevent="handleSubmit"
  >
    <el-form-item :label="t('login.username')" prop="username">
      <el-input
        v-model="form.username"
        :placeholder="t('login.usernamePlaceholder')"
        :prefix-icon="User"
        autocomplete="username"
      />
    </el-form-item>
    <el-form-item :label="t('login.password')" prop="password">
      <el-input
        v-model="form.password"
        type="password"
        :placeholder="t('login.passwordPlaceholder')"
        :prefix-icon="Lock"
        show-password
        autocomplete="current-password"
        @keyup.enter="handleSubmit"
      />
    </el-form-item>
    <el-form-item>
      <el-button
        type="primary"
        :loading="loading"
        style="width: 100%"
        native-type="submit"
      >
        {{ t("login.submit") }}
      </el-button>
    </el-form-item>
  </el-form>
</template>

<script setup lang="ts">
import { reactive, ref } from "vue";
import { User, Lock } from "@element-plus/icons-vue";
import type { FormInstance, FormRules } from "element-plus";
import { useI18nText } from "@/shared/i18n";

const emit = defineEmits<{
  submit: [username: string, password: string];
}>();

defineProps<{
  loading: boolean;
}>();

const { t } = useI18nText();

const formRef = ref<FormInstance>();
const form = reactive({
  username: "",
  password: "",
});

const rules: FormRules = {
  username: [
    {
      required: true,
      message: t("validation.login.usernameRequired"),
      trigger: "blur",
    },
  ],
  password: [
    {
      required: true,
      message: t("validation.login.passwordRequired"),
      trigger: "blur",
    },
  ],
};

async function handleSubmit() {
  if (!formRef.value) return;
  const valid = await formRef.value.validate().catch(() => false);
  if (!valid) return;
  emit("submit", form.username, form.password);
}
</script>
