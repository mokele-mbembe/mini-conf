<template>
  <el-dialog
    v-model="visible"
    :title="t('projectMembers.dialog.createTitle')"
    width="520px"
    :close-on-click-modal="false"
    @closed="handleClosed"
  >
    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-width="110px"
      label-position="right"
    >
      <el-form-item :label="t('projectMembers.form.username')" prop="username">
        <el-input
          v-model="form.username"
          :placeholder="t('projectMembers.form.usernamePlaceholder')"
        />
        <div class="project-member-form-dialog__hint">
          {{ t("projectMembers.form.usernameHint") }}
        </div>
      </el-form-item>

      <el-form-item :label="t('projectMembers.form.role')" prop="role">
        <el-select v-model="form.role" style="width: 100%">
          <el-option
            v-for="role in roles"
            :key="role"
            :label="roleLabel(role)"
            :value="role"
          />
        </el-select>
      </el-form-item>
    </el-form>

    <template #footer>
      <el-button @click="visible = false">{{ t("common.cancel") }}</el-button>
      <el-button type="primary" :loading="submitting" @click="handleSubmit">
        {{ t("common.add") }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { reactive, ref } from "vue";
import type { FormInstance, FormRules } from "element-plus";
import { ElMessage } from "element-plus";
import type { ProjectMember, ProjectRole } from "@/api/types/project";
import * as projectsApi from "@/api/projects";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import { useI18nText } from "@/shared/i18n";

const props = defineProps<{
  projectId: number;
}>();

const emit = defineEmits<{
  success: [member: ProjectMember];
}>();

interface FormState {
  username: string;
  role: ProjectRole;
}

const visible = defineModel<boolean>("visible", { default: false });
const { t } = useI18nText();
const formRef = ref<FormInstance>();
const submitting = ref(false);
const roles: ProjectRole[] = ["admin", "editor", "viewer"];

function makeEmptyForm(): FormState {
  return {
    username: "",
    role: "viewer",
  };
}

const form = reactive<FormState>(makeEmptyForm());

const rules: FormRules<FormState> = {
  username: [
    {
      required: true,
      message: t("validation.projectMembers.usernameRequired"),
      trigger: "blur",
    },
  ],
  role: [
    {
      required: true,
      message: t("validation.projectMembers.roleRequired"),
      trigger: "change",
    },
  ],
};

function roleLabel(role: ProjectRole): string {
  return t(`projectMembers.role.${role}`);
}

async function handleSubmit() {
  const valid = await formRef.value?.validate().catch(() => false);
  if (!valid) return;

  submitting.value = true;
  try {
    const member = await projectsApi.createProjectMember(props.projectId, {
      username: form.username.trim(),
      role: form.role,
    });
    ElMessage.success(t("projectMembers.toast.created"));
    emit("success", member);
    visible.value = false;
  } catch (err) {
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("projectMembers.toast.createFailed"));
    }
  } finally {
    submitting.value = false;
  }
}

function handleClosed() {
  formRef.value?.resetFields();
  Object.assign(form, makeEmptyForm());
}
</script>

<style scoped>
.project-member-form-dialog__hint {
  width: 100%;
  margin-top: 4px;
  color: var(--color-text-secondary);
  font-size: var(--font-size-sm);
  line-height: 1.5;
}
</style>
