<template>
  <div
    class="config-code-editor"
    :class="{ 'is-readonly': readonly }"
    :style="editorStyle"
  >
    <div ref="editorHost" class="config-code-editor__surface" />
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, shallowRef, watch } from "vue";
import { basicSetup } from "codemirror";
import { EditorState, Compartment, type Extension } from "@codemirror/state";
import { EditorView, placeholder as editorPlaceholder } from "@codemirror/view";
import { StreamLanguage } from "@codemirror/language";
import { json } from "@codemirror/lang-json";
import { yaml } from "@codemirror/lang-yaml";
import { toml } from "@codemirror/legacy-modes/mode/toml";

const props = withDefaults(
  defineProps<{
    modelValue: string;
    format?: string;
    readonly?: boolean;
    ariaLabel?: string;
    placeholder?: string;
    minHeight?: number;
  }>(),
  {
    format: "text",
    readonly: false,
    ariaLabel: "Config editor",
    placeholder: "",
    minHeight: 520,
  },
);

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

const editorHost = shallowRef<globalThis.HTMLDivElement | null>(null);
const editorView = shallowRef<EditorView | null>(null);
const editableCompartment = new Compartment();
const languageCompartment = new Compartment();
const contentAttributesCompartment = new Compartment();

const editorStyle = computed(() => ({
  "--config-code-editor-min-height": `${props.minHeight}px`,
}));

function languageExtension(format: string): Extension {
  switch (format.toLowerCase()) {
    case "json":
      return json();
    case "yaml":
    case "yml":
      return yaml();
    case "toml":
      return StreamLanguage.define(toml);
    default:
      return [];
  }
}

function contentAttributesExtension(): Extension {
  return EditorView.contentAttributes.of({
    "aria-label": props.ariaLabel,
  });
}

function editorExtensions(): Extension[] {
  return [
    basicSetup,
    EditorView.lineWrapping,
    editorPlaceholder(props.placeholder),
    EditorView.updateListener.of((update) => {
      if (update.docChanged) {
        emit("update:modelValue", update.state.doc.toString());
      }
    }),
    editableCompartment.of(EditorView.editable.of(!props.readonly)),
    languageCompartment.of(languageExtension(props.format)),
    contentAttributesCompartment.of(contentAttributesExtension()),
    EditorView.theme({
      "&": {
        minHeight: "var(--config-code-editor-min-height)",
        fontSize: "13px",
        borderRadius: "4px",
        border: "1px solid var(--el-border-color, #dcdfe6)",
        backgroundColor: "var(--el-bg-color, #ffffff)",
      },
      "&.cm-focused": {
        outline: "none",
        borderColor: "var(--el-color-primary)",
        boxShadow: "0 0 0 1px var(--el-color-primary-light-7)",
      },
      ".cm-scroller": {
        minHeight: "var(--config-code-editor-min-height)",
        fontFamily:
          'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
        lineHeight: "1.55",
      },
      ".cm-content": {
        minHeight: "var(--config-code-editor-min-height)",
        padding: "12px 0",
      },
      ".cm-line": {
        padding: "0 12px",
      },
      ".cm-gutters": {
        borderRight: "1px solid var(--el-border-color-lighter, #ebeef5)",
        backgroundColor: "var(--el-fill-color-lighter, #fafafa)",
        color: "var(--el-text-color-secondary)",
      },
      ".cm-activeLine": {
        backgroundColor: "var(--el-fill-color-light, #f5f7fa)",
      },
      ".cm-activeLineGutter": {
        backgroundColor: "var(--el-fill-color-light, #f5f7fa)",
      },
      ".cm-placeholder": {
        color: "var(--el-text-color-placeholder)",
      },
    }),
  ];
}

function syncDocument(value: string) {
  const view = editorView.value;
  if (!view) return;

  const current = view.state.doc.toString();
  if (current === value) return;

  view.dispatch({
    changes: {
      from: 0,
      to: current.length,
      insert: value,
    },
  });
}

onMounted(() => {
  if (!editorHost.value) return;

  editorView.value = new EditorView({
    parent: editorHost.value,
    state: EditorState.create({
      doc: props.modelValue,
      extensions: editorExtensions(),
    }),
  });
});

onBeforeUnmount(() => {
  editorView.value?.destroy();
  editorView.value = null;
});

watch(
  () => props.modelValue,
  (value) => syncDocument(value),
);

watch(
  () => props.readonly,
  (readonly) => {
    editorView.value?.dispatch({
      effects: editableCompartment.reconfigure(
        EditorView.editable.of(!readonly),
      ),
    });
  },
);

watch(
  () => props.format,
  (format) => {
    editorView.value?.dispatch({
      effects: languageCompartment.reconfigure(languageExtension(format)),
    });
  },
);

watch(
  () => props.ariaLabel,
  () => {
    editorView.value?.dispatch({
      effects: contentAttributesCompartment.reconfigure(
        contentAttributesExtension(),
      ),
    });
  },
);
</script>

<style scoped>
.config-code-editor {
  width: 100%;
}

.config-code-editor__surface {
  width: 100%;
}

.config-code-editor.is-readonly :deep(.cm-content) {
  cursor: default;
}
</style>
