<template>
  <el-dialog
    v-model="visible"
    :title="dialogTitle"
    width="620px"
    :close-on-click-modal="false"
    @closed="handleClosed"
  >
    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-width="120px"
      label-position="right"
    >
      <el-form-item :label="t('projectEnvironments.form.code')" prop="code">
        <el-input
          v-model="form.code"
          :disabled="isEdit"
          :placeholder="t('projectEnvironments.form.codePlaceholder')"
        />
      </el-form-item>

      <el-form-item :label="t('projectEnvironments.form.name')" prop="name">
        <el-input
          v-model="form.name"
          :placeholder="t('projectEnvironments.form.namePlaceholder')"
        />
      </el-form-item>

      <el-form-item :label="t('projectEnvironments.form.status')" prop="status">
        <el-select v-model="form.status" style="width: 100%">
          <el-option :label="t('status.active')" value="active" />
          <el-option :label="t('status.inactive')" value="inactive" />
        </el-select>
      </el-form-item>

      <el-form-item
        :label="t('projectEnvironments.form.sortOrder')"
        prop="sort_order"
      >
        <el-input-number v-model="form.sort_order" :min="0" :step="10" />
      </el-form-item>

      <el-form-item
        :label="t('projectEnvironments.form.description')"
        prop="description"
      >
        <el-input
          v-model="form.description"
          type="textarea"
          :rows="3"
          :placeholder="t('projectEnvironments.form.descriptionPlaceholder')"
        />
      </el-form-item>
    </el-form>

    <template #footer>
      <el-button @click="visible = false">{{ t("common.cancel") }}</el-button>
      <el-button type="primary" :loading="submitting" @click="handleSubmit">
        {{ isEdit ? t("common.save") : t("common.create") }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, reactive, watch, ref } from "vue";
import type { FormInstance, FormRules } from "element-plus";
import { ElMessage } from "element-plus";
import * as projectEnvironmentsApi from "@/api/project-environments";
import type {
  ProjectEnvironmentStatus,
  ProjectEnvironmentSummary,
} from "@/api/types/project-environment";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import { useI18nText } from "@/shared/i18n";

const props = defineProps<{
  projectId: number;
  editTarget: ProjectEnvironmentSummary | null;
}>();

const emit = defineEmits<{
  success: [item: ProjectEnvironmentSummary];
}>();

const visible = defineModel<boolean>("visible", { default: false });
const { t } = useI18nText();
const formRef = ref<FormInstance>();
const submitting = ref(false);

interface FormState {
  code: string;
  name: string;
  description: string;
  status: ProjectEnvironmentStatus;
  sort_order: number;
}

const isEdit = computed(() => props.editTarget !== null);
const dialogTitle = computed(() =>
  isEdit.value
    ? t("projectEnvironments.dialog.editTitle")
    : t("projectEnvironments.dialog.createTitle"),
);

function makeEmptyForm(): FormState {
  return {
    code: "",
    name: "",
    description: "",
    status: "active",
    sort_order: 10,
  };
}

const form = reactive<FormState>(makeEmptyForm());

const rules: FormRules<FormState> = {
  code: [
    {
      required: true,
      message: t("validation.projectEnvironments.codeRequired"),
      trigger: "blur",
    },
  ],
  name: [
    {
      required: true,
      message: t("validation.projectEnvironments.nameRequired"),
      trigger: "blur",
    },
  ],
};

watch(
  () => props.editTarget,
  (value) => {
    if (!value) {
      Object.assign(form, makeEmptyForm());
      return;
    }

    Object.assign(form, {
      code: value.code,
      name: value.name,
      description: value.description ?? "",
      status: value.status,
      sort_order: value.sort_order,
    });
  },
  { immediate: true },
);

async function handleSubmit() {
  const valid = await formRef.value?.validate().catch(() => false);
  if (!valid) return;

  submitting.value = true;
  try {
    const payload = {
      name: form.name,
      description: form.description || null,
      status: form.status,
      sort_order: form.sort_order,
    };
    const result = props.editTarget
      ? await projectEnvironmentsApi.updateProjectEnvironment(
          props.projectId,
          props.editTarget.id,
          payload,
        )
      : await projectEnvironmentsApi.createProjectEnvironment(props.projectId, {
          code: form.code,
          ...payload,
        });
    ElMessage.success(
      isEdit.value
        ? t("toast.projectEnvironments.updated")
        : t("toast.projectEnvironments.created"),
    );
    emit("success", result);
    visible.value = false;
  } catch (err) {
    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.operationFailed"));
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
