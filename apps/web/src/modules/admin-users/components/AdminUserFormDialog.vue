<template>
  <el-dialog
    v-model="localOpen"
    :title="t('admin.users.createDialog.title')"
    @closed="handleClosed"
  >
    <el-form ref="formRef" :model="form" :rules="rules" label-position="top">
      <el-form-item :label="t('admin.users.form.username')" prop="username">
        <el-input
          v-model="form.username"
          :placeholder="t('admin.users.form.usernamePlaceholder')"
        />
      </el-form-item>
      <el-form-item :label="t('admin.users.form.password')" prop="password">
        <el-input
          v-model="form.password"
          type="password"
          :placeholder="t('admin.users.form.passwordPlaceholder')"
          show-password
        />
      </el-form-item>
      <el-form-item :label="t('admin.users.form.status')">
        <el-radio-group v-model="form.status">
          <el-radio label="active">{{ t("status.active") }}</el-radio>
          <el-radio label="disabled">{{ t("status.disabled") }}</el-radio>
        </el-radio-group>
      </el-form-item>
      <el-form-item>
        <el-checkbox v-model="form.is_platform_admin">
          {{ t("admin.users.form.isPlatformAdmin") }}
        </el-checkbox>
      </el-form-item>
      <el-form-item>
        <el-checkbox v-model="form.must_change_password">
          {{ t("admin.users.form.mustChangePassword") }}
        </el-checkbox>
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="localOpen = false">{{ t("common.cancel") }}</el-button>
      <el-button type="primary" :loading="loading" @click="handleSubmit">
        {{ t("common.create") }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import type { FormInstance, FormRules } from "element-plus";
import { useI18nText } from "@/shared/i18n";
import type { AdminUserCreateRequest } from "@/api/types/admin-user";

const { t } = useI18nText();

interface Props {
  modelValue: boolean;
  loading?: boolean;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  submit: [data: AdminUserCreateRequest];
  "update:modelValue": [value: boolean];
}>();

const formRef = ref<FormInstance>();

function createInitialForm(): AdminUserCreateRequest {
  return {
    username: "",
    password: "",
    status: "active",
    is_platform_admin: false,
    must_change_password: false,
  };
}

const form = ref<AdminUserCreateRequest>(createInitialForm());

const rules: FormRules = {
  username: [
    {
      required: true,
      message: t("validation.login.usernameRequired"),
      trigger: "blur",
    },
    {
      min: 3,
      message: t("validation.admin.usernameTooShort"),
      trigger: "blur",
    },
  ],
  password: [
    {
      required: true,
      message: t("validation.login.passwordRequired"),
      trigger: "blur",
    },
    {
      min: 8,
      message: t("validation.admin.passwordTooShort"),
      trigger: "blur",
    },
  ],
};

const localOpen = computed({
  get: () => props.modelValue,
  set: (value) => emit("update:modelValue", value),
});

async function handleSubmit() {
  if (!formRef.value) return;
  const valid = await formRef.value.validate().catch(() => false);
  if (!valid) return;

  emit("submit", {
    username: form.value.username,
    password: form.value.password,
    status: form.value.status,
    is_platform_admin: form.value.is_platform_admin,
    must_change_password: form.value.must_change_password,
  });
}

function resetForm() {
  form.value = createInitialForm();
}

async function handleClosed() {
  resetForm();
  await nextTick();
  formRef.value?.clearValidate();
}
</script>
