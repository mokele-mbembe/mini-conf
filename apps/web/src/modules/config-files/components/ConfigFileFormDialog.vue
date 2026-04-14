<template>
  <el-dialog
    v-model="visible"
    :title="
      isEdit
        ? t('configFiles.dialog.editTitle')
        : t('configFiles.dialog.createTitle')
    "
    width="600px"
    :close-on-click-modal="false"
    @closed="handleClosed"
  >
    <el-alert
      v-if="showUnsupportedFormatNotice"
      :title="
        t('configFiles.dialog.unsupportedFormat', {
          format: unsupportedCurrentFormat ?? '',
        })
      "
      type="warning"
      show-icon
      :closable="false"
      style="margin-bottom: 16px"
    />

    <el-form
      ref="formRef"
      v-loading="detailLoading"
      :model="form"
      :rules="rules"
      label-width="120px"
      label-position="right"
    >
      <el-form-item :label="t('configFiles.form.code')" prop="code">
        <el-input
          v-model="form.code"
          :placeholder="t('configFiles.form.codePlaceholder')"
          :disabled="isEdit"
        />
        <div class="form-hint">
          {{ t("configFiles.form.codeHint") }}
        </div>
      </el-form-item>

      <el-form-item :label="t('configFiles.form.name')" prop="name">
        <el-input
          v-model="form.name"
          :placeholder="t('configFiles.form.namePlaceholder')"
        />
      </el-form-item>

      <el-form-item :label="t('configFiles.form.format')" prop="format">
        <el-select
          v-model="form.format"
          :placeholder="t('configFiles.form.formatPlaceholder')"
          style="width: 100%"
        >
          <el-option
            v-for="option in formatOptions"
            :key="option.value"
            :label="option.label"
            :value="option.value"
          />
        </el-select>
        <div class="form-hint">{{ t("configFiles.form.formatHint") }}</div>
      </el-form-item>

      <el-form-item
        :label="t('configFiles.form.sensitivity')"
        prop="sensitivity"
      >
        <el-select
          v-model="form.sensitivity"
          :placeholder="t('configFiles.form.sensitivityPlaceholder')"
          style="width: 100%"
          clearable
        >
          <el-option
            :label="t('configFiles.form.sensitivity.normal')"
            value="normal"
          />
          <el-option
            :label="t('configFiles.form.sensitivity.secret')"
            value="secret"
          />
        </el-select>
      </el-form-item>

      <el-form-item :label="t('configFiles.form.required')" prop="is_required">
        <el-switch v-model="form.is_required" />
        <span class="form-hint">{{ t("configFiles.form.requiredHint") }}</span>
      </el-form-item>

      <el-form-item
        v-if="isEdit"
        :label="t('configFiles.form.status')"
        prop="status"
      >
        <el-select
          v-model="form.status"
          :placeholder="t('configFiles.form.statusPlaceholder')"
          style="width: 100%"
        >
          <el-option :label="t('status.active')" value="active" />
          <el-option :label="t('status.archived')" value="archived" />
        </el-select>
        <div class="form-hint">
          {{ t("configFiles.form.statusHint") }}
        </div>
      </el-form-item>

      <el-form-item
        :label="t('configFiles.form.secretPaths')"
        prop="secret_paths_raw"
      >
        <el-input
          v-model="form.secret_paths_raw"
          type="textarea"
          :rows="3"
          :disabled="form.sensitivity !== 'secret'"
          :placeholder="t('configFiles.form.secretPathsPlaceholder')"
        />
        <div class="form-hint">
          {{ t("configFiles.form.secretPathsHint") }}
        </div>
      </el-form-item>

      <el-form-item
        :label="t('configFiles.form.description')"
        prop="description"
      >
        <el-input
          v-model="form.description"
          type="textarea"
          :rows="2"
          :placeholder="t('configFiles.form.descriptionPlaceholder')"
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
import { ref, reactive, computed, watch } from "vue";
import type { FormInstance, FormRules } from "element-plus";
import { ElMessage } from "element-plus";
import * as configFilesApi from "@/api/config-files";
import { ApiRequestError } from "@/api/error";
import { getErrorMessage } from "@/shared/constants/error-messages";
import type { ConfigFileSummary } from "@/api/types/config-file";
import { useI18nText } from "@/shared/i18n";

const props = defineProps<{
  projectId: number;
  editTarget?: ConfigFileSummary | null;
}>();

const emit = defineEmits<{
  success: [item: ConfigFileSummary];
}>();

const visible = defineModel<boolean>("visible", { default: false });

const isEdit = computed(() => !!props.editTarget);
const detailLoading = ref(false);
const { t } = useI18nText();

const formRef = ref<FormInstance>();
const submitting = ref(false);

interface FormState {
  code: string;
  name: string;
  format: string;
  sensitivity: string;
  is_required: boolean;
  status: string;
  secret_paths_raw: string;
  description: string;
}

function makeEmptyForm(): FormState {
  return {
    code: "",
    name: "",
    format: "yaml",
    sensitivity: "normal",
    is_required: false,
    status: "active",
    secret_paths_raw: "",
    description: "",
  };
}

const form = reactive<FormState>(makeEmptyForm());
const supportedFormats = [
  { label: "YAML", value: "yaml" },
  { label: "JSON", value: "json" },
  { label: "TOML", value: "toml" },
];
const supportedFormatValues = supportedFormats.map((option) => option.value);
const formatOptions = supportedFormats;
const unsupportedCurrentFormat = ref<string | null>(null);
const showUnsupportedFormatNotice = computed(
  () => unsupportedCurrentFormat.value !== null,
);

const rules: FormRules = {
  code: [
    {
      required: true,
      message: t("validation.configFiles.codeRequired"),
      trigger: "blur",
    },
  ],
  name: [
    {
      required: true,
      message: t("validation.configFiles.nameRequired"),
      trigger: "blur",
    },
  ],
  format: [
    {
      required: true,
      message: t("validation.configFiles.formatRequired"),
      trigger: "change",
    },
  ],
};

function applyConfigFile(target: ConfigFileSummary) {
  form.code = target.code;
  form.name = target.name;
  unsupportedCurrentFormat.value = supportedFormatValues.includes(target.format)
    ? null
    : target.format;
  form.format = supportedFormatValues.includes(target.format)
    ? target.format
    : "";
  form.sensitivity = target.sensitivity || "normal";
  form.is_required = target.is_required;
  form.status = target.status;
  form.secret_paths_raw = (target.secret_paths ?? []).join("\n");
  form.description = target.description ?? "";
}

function resetFormState() {
  unsupportedCurrentFormat.value = null;
  Object.assign(form, makeEmptyForm());
}

async function loadEditTargetDetail(id: number) {
  detailLoading.value = true;
  try {
    const detail = await configFilesApi.getConfigFile(id);
    applyConfigFile(detail);
  } catch (err) {
    if (props.editTarget) {
      applyConfigFile(props.editTarget);
    }

    if (err instanceof ApiRequestError) {
      ElMessage.error(getErrorMessage(err.code, err.message));
    } else {
      ElMessage.error(t("toast.configFiles.loadFailed"));
    }
  } finally {
    detailLoading.value = false;
  }
}

watch(
  [() => visible.value, () => props.editTarget?.id],
  async ([isVisible]) => {
    if (!isVisible) {
      return;
    }

    if (props.editTarget) {
      await loadEditTargetDetail(props.editTarget.id);
      return;
    }

    resetFormState();
  },
  { immediate: true },
);

function parseSecretPaths(raw: string): string[] | null {
  const lines = raw
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.length > 0);
  return lines.length > 0 ? lines : null;
}

async function handleSubmit() {
  const valid = await formRef.value?.validate().catch(() => false);
  if (!valid) return;

  submitting.value = true;
  try {
    const secretPaths =
      form.sensitivity === "secret"
        ? parseSecretPaths(form.secret_paths_raw)
        : null;
    let result: ConfigFileSummary;

    if (isEdit.value && props.editTarget) {
      result = await configFilesApi.updateConfigFile(props.editTarget.id, {
        project_id: props.projectId,
        code: form.code,
        name: form.name,
        format: form.format,
        status: form.status,
        sensitivity: form.sensitivity || null,
        is_required: form.is_required,
        secret_paths: secretPaths,
        description: form.description || null,
      });
    } else {
      result = await configFilesApi.createConfigFile({
        project_id: props.projectId,
        code: form.code,
        name: form.name,
        format: form.format,
        sensitivity: form.sensitivity || null,
        is_required: form.is_required,
        secret_paths: secretPaths,
        description: form.description || null,
      });
    }

    ElMessage.success(
      isEdit.value
        ? t("toast.configFiles.updated")
        : t("toast.configFiles.created"),
    );
    emit("success", result);
    visible.value = false;
  } catch (err) {
    if (err instanceof ApiRequestError) {
      const msg = getErrorMessage(err.code, err.message);
      ElMessage.error(msg);
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
