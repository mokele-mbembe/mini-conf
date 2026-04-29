<template>
  <Teleport to="body">
    <div
      v-if="isActive"
      class="draft-editor-overlay"
      role="dialog"
      aria-modal="true"
      @keydown.esc.stop.prevent="requestEditorClose"
    >
      <div class="draft-editor-overlay__backdrop" @click="requestEditorClose" />
      <section
        ref="surfaceRef"
        class="draft-editor-overlay__surface"
        tabindex="-1"
      >
        <DraftEditorPage
          ref="editorRef"
          embedded
          :deployment-id-override="deploymentId"
          :config-file-id-override="activeConfigFileId"
          @close="emit('request-close')"
          @switch-config="switchActiveConfigFile"
        />
      </section>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import DraftEditorPage from "@/modules/drafts/pages/DraftEditorPage.vue";

const props = defineProps<{
  visible: boolean;
  deploymentId?: number | null;
  configFileId: number | null;
}>();

const emit = defineEmits<{
  "request-close": [];
}>();

const activeConfigFileId = ref<number | null>(null);
const isActive = computed(
  () => props.visible && activeConfigFileId.value !== null,
);
const surfaceRef = ref<HTMLElement | null>(null);
const editorRef = ref<{ requestClose: () => Promise<void> | void } | null>(
  null,
);
let previousBodyOverflow: string | null = null;

function setBodyScrollLocked(locked: boolean) {
  const body = globalThis.document?.body;
  if (!body) return;

  if (locked) {
    if (previousBodyOverflow === null) {
      previousBodyOverflow = body.style.overflow;
    }
    body.style.overflow = "hidden";
    return;
  }

  if (previousBodyOverflow !== null) {
    body.style.overflow = previousBodyOverflow;
    previousBodyOverflow = null;
  }
}

async function requestEditorClose() {
  if (editorRef.value?.requestClose) {
    await editorRef.value.requestClose();
    return;
  }

  emit("request-close");
}

function switchActiveConfigFile(configFileId: number) {
  activeConfigFileId.value = configFileId;
}

watch(
  () => [props.visible, props.configFileId] as const,
  ([visible, configFileId]) => {
    activeConfigFileId.value = visible ? configFileId : null;
  },
  { immediate: true },
);

watch(
  isActive,
  async (active) => {
    setBodyScrollLocked(active);
    if (!active) return;

    await nextTick();
    surfaceRef.value?.focus();
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  setBodyScrollLocked(false);
});
</script>

<style scoped>
.draft-editor-overlay {
  position: fixed;
  inset: 0;
  z-index: 1800;
  display: flex;
  padding: clamp(8px, 2vw, 24px);
}

.draft-editor-overlay__backdrop {
  position: absolute;
  inset: 0;
  background: rgb(31 45 61 / 54%);
  backdrop-filter: blur(2px);
}

.draft-editor-overlay__surface {
  position: relative;
  z-index: 1;
  width: min(100%, var(--content-max-width));
  height: 100%;
  margin: 0 auto;
  overflow: auto;
  background: var(--color-bg-card);
  border-radius: var(--border-radius-lg);
  box-shadow: 0 18px 48px rgb(31 45 61 / 28%);
  outline: none;
}

@media (max-width: 768px) {
  .draft-editor-overlay {
    padding: 0;
  }

  .draft-editor-overlay__surface {
    width: 100%;
    border-radius: 0;
  }
}
</style>
