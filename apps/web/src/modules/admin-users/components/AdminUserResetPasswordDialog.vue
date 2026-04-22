<template>
  <el-dialog
    v-model="localOpen"
    :title="t('admin.users.resetPasswordDialog.title')"
    @closed="handleClosed"
  >
    <div v-if="user">
      <p>
        {{
          t("admin.users.resetPasswordDialog.confirm", {
            username: user.username,
          })
        }}
      </p>
      <el-form ref="formRef" :model="form" :rules="rules" label-position="top">
        <el-form-item
          :label="t('admin.users.form.newPassword')"
          prop="new_password"
        >
          <el-input
            v-model="form.new_password"
            type="password"
            :placeholder="t('admin.users.form.newPasswordPlaceholder')"
            show-password
          />
        </el-form-item>
        <el-form-item>
          <el-checkbox v-model="form.must_change_password">
            {{ t("admin.users.form.mustChangePassword") }}
          </el-checkbox>
        </el-form-item>
      </el-form>
    </div>
    <template #footer>
      <el-button @click="localOpen = false">{{ t("common.cancel") }}</el-button>
      <el-button type="primary" :loading="loading" @click="handleSubmit">
        {{ t("common.confirm") }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import type { FormInstance, FormRules } from "element-plus";
import { useI18nText } from "@/shared/i18n";
import type { AdminUser } from "@/api/types/admin-user";
import type { AdminUserResetPasswordRequest } from "@/api/types/admin-user";

const { t } = useI18nText();

interface Props {
  modelValue: boolean;
  user: AdminUser | null;
  loading?: boolean;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  submit: [payload: AdminUserResetPasswordRequest];
  "update:modelValue": [value: boolean];
}>();

const formRef = ref<FormInstance>();

function createInitialForm(): AdminUserResetPasswordRequest {
  return {
    new_password: "",
    must_change_password: true,
  };
}

const form = ref<AdminUserResetPasswordRequest>(createInitialForm());

const rules: FormRules = {
  new_password: [
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
    new_password: form.value.new_password,
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
