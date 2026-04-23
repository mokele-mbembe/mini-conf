import { ref, computed } from "vue";
import { defineStore } from "pinia";
import * as setupApi from "@/api/setup";
import { isApiError } from "@/api/error";
import type { SetupStatusResponse } from "@/api/types/setup";
import { t } from "@/shared/i18n";

export type SetupCheckResult = "required" | "complete" | "error";

export const useSetupStatus = defineStore("setupStatus", () => {
  const status = ref<SetupStatusResponse | null>(null);
  const checked = ref(false);
  const loadError = ref<string | null>(null);

  const setupRequired = computed(() => status.value?.setup_required ?? false);

  async function checkStatus(): Promise<SetupCheckResult> {
    loadError.value = null;

    try {
      const response = await setupApi.getSetupStatus();
      status.value = response;
      checked.value = true;
      return response.setup_required ? "required" : "complete";
    } catch (err) {
      checked.value = true;
      if (isApiError(err)) {
        loadError.value = err.message;
      } else {
        loadError.value = t("setup.statusLoadFailed");
      }
      return "error";
    }
  }

  return {
    status,
    checked,
    loadError,
    setupRequired,
    checkStatus,
  };
});
