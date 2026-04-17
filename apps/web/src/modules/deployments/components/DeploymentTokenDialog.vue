<template>
  <el-dialog
    v-model="visible"
    :title="title"
    width="620px"
    :close-on-click-modal="false"
  >
    <el-alert
      :title="t('deployments.tokenDialog.notice')"
      type="warning"
      show-icon
      :closable="false"
      style="margin-bottom: 16px"
    />

    <el-descriptions :column="1" border>
      <el-descriptions-item
        :label="t('deployments.tokenDialog.credentialName')"
      >
        {{ payload?.credential_name }}
      </el-descriptions-item>
      <el-descriptions-item :label="t('deployments.tokenDialog.tokenPreview')">
        {{ payload?.token_preview }}
      </el-descriptions-item>
      <el-descriptions-item :label="t('deployments.tokenDialog.token')">
        <el-input
          :model-value="payload?.token ?? ''"
          type="textarea"
          :rows="4"
          readonly
        />
      </el-descriptions-item>
    </el-descriptions>

    <template #footer>
      <el-button @click="visible = false">{{ t("common.close") }}</el-button>
      <el-button type="primary" @click="handleCopy">
        {{ t("common.copy") }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { ElMessage } from "element-plus";
import type { DeploymentTokenResponse } from "@/api/types/deployment-instance";
import { useI18nText } from "@/shared/i18n";

const props = defineProps<{
  payload: DeploymentTokenResponse | null;
  mode: "activate" | "reset";
}>();

const visible = defineModel<boolean>("visible", { default: false });

const { t } = useI18nText();

const title = computed(() =>
  props.mode === "activate"
    ? t("deployments.tokenDialog.activateTitle")
    : t("deployments.tokenDialog.resetTitle"),
);

async function handleCopy() {
  if (!props.payload) return;

  try {
    const clipboard = globalThis.navigator?.clipboard;
    if (!clipboard) {
      throw new Error("clipboard_unavailable");
    }
    await clipboard.writeText(props.payload.token);
    ElMessage.success(t("toast.deployments.tokenCopied"));
  } catch {
    ElMessage.error(t("toast.operationFailed"));
  }
}
</script>
