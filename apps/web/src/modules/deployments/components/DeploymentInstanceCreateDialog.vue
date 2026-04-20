<template>
  <el-dialog
    v-model="visible"
    :title="t('deployments.dialog.createTitle')"
    width="620px"
    :close-on-click-modal="false"
    @closed="handleClosed"
  >
    <el-alert
      :title="t('deployments.dialog.inactiveNotice')"
      type="info"
      show-icon
      :closable="false"
      style="margin-bottom: 16px"
    />

    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-width="120px"
      label-position="right"
    >
      <el-form-item :label="t('deployments.form.name')" prop="name">
        <el-input
          v-model="form.name"
          :placeholder="t('deployments.form.namePlaceholder')"
        />
      </el-form-item>

      <el-form-item
        :label="t('deployments.form.deploymentKey')"
        prop="deployment_key"
      >
        <el-input
          v-model="form.deployment_key"
          :placeholder="t('deployments.form.deploymentKeyPlaceholder')"
        />
        <div class="form-hint">
          {{ t("deployments.form.deploymentKeyHint") }}
        </div>
      </el-form-item>

      <el-form-item
        :label="t('deployments.form.environment')"
        prop="environment_id"
      >
        <el-select
          v-model="form.environment_id"
          :placeholder="t('deployments.form.environmentPlaceholder')"
          style="width: 100%"
        >
          <el-option
            v-for="item in activeEnvironments"
            :key="item.id"
            :label="`${item.name} (${item.code})`"
            :value="item.id"
          />
        </el-select>
      </el-form-item>

      <el-form-item :label="t('deployments.form.template')" prop="is_template">
        <el-switch v-model="form.is_template" />
        <span class="form-hint">
          {{ t("deployments.form.templateHint") }}
        </span>
      </el-form-item>

      <el-form-item
        :label="t('deployments.form.description')"
        prop="description"
      >
        <el-input
          v-model="form.description"
          type="textarea"
          :rows="3"
          :placeholder="t('deployments.form.descriptionPlaceholder')"
        />
      </el-form-item>
    </el-form>

    <template #footer>
      <el-button @click="visible = false">{{ t("common.cancel") }}</el-button>
      <el-button type="primary" :loading="submitting" @click="handleSubmit">
        {{ t("common.create") }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import type { FormInstance, FormRules } from "element-plus";
import { ElMessage } from "element-plus";
import * as deploymentInstancesApi from "@/api/deployment-instances";
import { useProjectEnvironments } from "@/modules/project-environments/composables/useProjectEnvironments";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type { DeploymentInstanceSummary } from "@/api/types/deployment-instance";
import { useI18nText } from "@/shared/i18n";

const props = defineProps<{
  projectId: number;
}>();

const emit = defineEmits<{
  success: [item: DeploymentInstanceSummary];
}>();

const visible = defineModel<boolean>("visible", { default: false });

const { t } = useI18nText();
const formRef = ref<FormInstance>();
const submitting = ref(false);

interface FormState {
  name: string;
  deployment_key: string;
  environment_id: number | undefined;
  is_template: boolean;
  description: string;
}

function makeEmptyForm(): FormState {
  return {
    name: "",
    deployment_key: "",
    environment_id: undefined,
    is_template: false,
    description: "",
  };
}

const form = reactive<FormState>(makeEmptyForm());
const { environments, load: loadEnvs } = useProjectEnvironments(
  () => props.projectId,
);
const activeEnvironments = computed(() =>
  environments.value.filter((item) => item.status === "active"),
);

const rules: FormRules = {
  name: [
    {
      required: true,
      message: t("validation.deployments.nameRequired"),
      trigger: "blur",
    },
  ],
  deployment_key: [
    {
      required: true,
      message: t("validation.deployments.deploymentKeyRequired"),
      trigger: "blur",
    },
  ],
  environment_id: [
    {
      required: true,
      message: t("validation.deployments.environmentRequired"),
      trigger: "change",
    },
  ],
};

watch(
  visible,
  async (value) => {
    if (!value) return;
    await loadEnvs();
    if (!form.environment_id) {
      form.environment_id = activeEnvironments.value[0]?.id;
    }
  },
  { immediate: true },
);

function resetFormState() {
  Object.assign(form, makeEmptyForm());
}

async function handleSubmit() {
  const valid = await formRef.value?.validate().catch(() => false);
  if (!valid) return;

  submitting.value = true;
  try {
    const result = await deploymentInstancesApi.createDeploymentInstance({
      project_id: props.projectId,
      name: form.name,
      deployment_key: form.deployment_key,
      environment_id: form.environment_id as number,
      description: form.description || null,
      is_template: form.is_template,
    });

    ElMessage.success(t("toast.deployments.created"));
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
  resetFormState();
  environments.value = [];
}
</script>

<style scoped>
.form-hint {
  font-size: var(--font-size-sm, 12px);
  color: var(--color-text-secondary);
  margin-top: 4px;
  line-height: 1.4;
}
</style>
