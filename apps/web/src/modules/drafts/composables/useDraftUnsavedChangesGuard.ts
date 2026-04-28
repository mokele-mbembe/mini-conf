import { onBeforeUnmount, onMounted } from "vue";
import { ElMessageBox } from "element-plus";
import { onBeforeRouteLeave } from "vue-router";

type ReadableRef<T> = {
  readonly value: T;
};

interface UseDraftUnsavedChangesGuardOptions {
  isDirty: ReadableRef<boolean>;
  t: (key: string) => string;
}

interface BeforeUnloadEventLike {
  preventDefault: () => void;
  returnValue?: string;
}

export function useDraftUnsavedChangesGuard(
  options: UseDraftUnsavedChangesGuardOptions,
) {
  async function confirmIfDirty(): Promise<boolean> {
    if (!options.isDirty.value) return true;
    try {
      await ElMessageBox.confirm(
        options.t("drafts.navigate.prompt"),
        options.t("drafts.navigate.title"),
        {
          confirmButtonText: options.t("drafts.navigate.confirm"),
          cancelButtonText: options.t("common.cancel"),
          type: "warning",
        },
      );
      return true;
    } catch {
      return false;
    }
  }

  function handleBeforeUnload(e: BeforeUnloadEventLike) {
    if (options.isDirty.value) {
      e.preventDefault();
      e.returnValue = "";
    }
  }

  onBeforeRouteLeave(confirmIfDirty);

  onMounted(() => {
    globalThis.addEventListener("beforeunload", handleBeforeUnload);
  });
  onBeforeUnmount(() => {
    globalThis.removeEventListener("beforeunload", handleBeforeUnload);
  });

  return {
    confirmIfDirty,
    handleBeforeUnload,
  };
}
