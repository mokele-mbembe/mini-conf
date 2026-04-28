import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import DraftCloneSourceDialog, {
  type DraftCloneSourceKind,
} from "./DraftCloneSourceDialog.vue";
import type { CloneSourceSummary } from "@/api/types/clone-source";

const cloneSources: CloneSourceSummary[] = [
  {
    deployment_instance_id: 1,
    deployment_key: "store-001",
    name: "Store 001",
    environment_id: 10,
    environment_name: "Production",
    is_template: false,
    available_sources: {
      draft: true,
      latest_release: true,
    },
  },
  {
    deployment_instance_id: 2,
    deployment_key: "template-default",
    name: "Default Template",
    environment_id: 10,
    environment_name: "Production",
    is_template: true,
    available_sources: {
      draft: true,
      latest_release: false,
    },
  },
  {
    deployment_instance_id: 3,
    deployment_key: "empty",
    name: "Empty",
    environment_id: 10,
    environment_name: "Production",
    is_template: false,
    available_sources: {
      draft: false,
      latest_release: false,
    },
  },
];

function mountDialog(
  props?: Partial<InstanceType<typeof DraftCloneSourceDialog>["$props"]>,
) {
  return mount(DraftCloneSourceDialog, {
    props: {
      modelValue: true,
      cloneSources,
      sourceInstanceId: 1,
      sourceKind: "draft" as DraftCloneSourceKind,
      loadError: false,
      instancesLoading: false,
      loadingMore: false,
      nextCursor: null,
      selectedSource: cloneSources[0],
      draftOptionDisabled: false,
      releaseOptionDisabled: false,
      selectedSourceDraftUnavailable: false,
      selectedSourceReleaseUnavailable: false,
      submitDisabled: false,
      cloning: false,
      ...props,
    },
    global: {
      stubs: {
        "el-alert": {
          template: '<div class="alert-stub"><slot /></div>',
        },
        "el-button": {
          props: ["disabled"],
          emits: ["click"],
          template:
            '<button class="button-stub" type="button" :disabled="disabled" @click="$emit(\'click\')"><slot /></button>',
        },
        "el-dialog": {
          props: ["modelValue", "title"],
          emits: ["update:modelValue"],
          template:
            '<section class="dialog-stub" :data-title="title"><slot /><footer><slot name="footer" /></footer></section>',
        },
        "el-form": {
          template: '<form class="form-stub"><slot /></form>',
        },
        "el-form-item": {
          props: ["label"],
          template:
            '<label class="form-item-stub"><span>{{ label }}</span><slot /></label>',
        },
        "el-option": {
          props: ["label", "value"],
          template:
            '<div class="option-stub" :data-label="label" :data-value="value"><slot /></div>',
        },
        "el-radio": {
          props: ["value", "disabled"],
          template:
            '<label class="radio-stub" :data-value="value" :data-disabled="String(disabled)"><slot /></label>',
        },
        "el-radio-group": {
          name: "ElRadioGroup",
          props: ["modelValue"],
          emits: ["update:modelValue"],
          template:
            '<div class="radio-group-stub" :data-model-value="modelValue"><slot /></div>',
        },
        "el-select": {
          name: "ElSelect",
          props: ["modelValue", "remoteMethod"],
          emits: ["update:modelValue"],
          template:
            '<div class="select-stub" :data-model-value="modelValue"><slot /><slot name="footer" /></div>',
        },
        "el-tag": {
          template: '<span class="tag-stub"><slot /></span>',
        },
      },
    },
  });
}

describe("DraftCloneSourceDialog", () => {
  it("renders source options, availability, and template markers", () => {
    const wrapper = mountDialog();

    expect(wrapper.find(".dialog-stub").attributes("data-title")).toBe(
      "从其他实例复制配置",
    );
    expect(wrapper.text()).toContain("Store 001");
    expect(wrapper.text()).toContain("store-001");
    expect(wrapper.text()).toContain("Draft ✓");
    expect(wrapper.text()).toContain("Release ✓");
    expect(wrapper.text()).toContain("Default Template");
    expect(wrapper.text()).toContain("模板");
    expect(wrapper.text()).toContain("无可复制内容");
    expect(wrapper.findAll(".option-stub")[1].attributes("data-label")).toBe(
      "Default Template (template-default) [模板]",
    );
  });

  it("renders loading errors and load-more controls", () => {
    const wrapper = mountDialog({
      loadError: true,
      nextCursor: 4,
    });

    expect(wrapper.find(".alert-stub").text()).toContain(
      "加载实例列表失败，请关闭后重试",
    );
    expect(wrapper.text()).toContain("加载更多");
  });

  it("shows selected source availability hints", () => {
    const wrapper = mountDialog({
      selectedSource: cloneSources[1],
      draftOptionDisabled: false,
      releaseOptionDisabled: true,
      selectedSourceDraftUnavailable: false,
      selectedSourceReleaseUnavailable: true,
    });

    expect(wrapper.text()).toContain("模板实例无法发布 Release");
    expect(
      wrapper
        .findAll(".radio-stub")
        .map((radio) => radio.attributes("data-disabled")),
    ).toEqual(["false", "true"]);
  });

  it("emits model, selection, source kind, load-more, and submit events", async () => {
    const wrapper = mountDialog();

    wrapper
      .findComponent({ name: "ElSelect" })
      .vm.$emit("update:modelValue", 2);
    wrapper
      .findComponent({ name: "ElRadioGroup" })
      .vm.$emit("update:modelValue", "latest_release");
    await wrapper.findAll(".button-stub")[0].trigger("click");
    await wrapper.findAll(".button-stub")[1].trigger("click");

    expect(wrapper.emitted("update:sourceInstanceId")).toEqual([[2]]);
    expect(wrapper.emitted("update:sourceKind")).toEqual([["latest_release"]]);
    expect(wrapper.emitted("update:modelValue")).toEqual([[false]]);
    expect(wrapper.emitted("submit")).toHaveLength(1);
  });

  it("normalizes invalid source model updates", () => {
    const wrapper = mountDialog();

    wrapper
      .findComponent({ name: "ElSelect" })
      .vm.$emit("update:modelValue", "not-a-number");
    wrapper
      .findComponent({ name: "ElRadioGroup" })
      .vm.$emit("update:modelValue", "unsupported");

    expect(wrapper.emitted("update:sourceInstanceId")).toEqual([[null]]);
    expect(wrapper.emitted("update:sourceKind")).toBeUndefined();
  });
});
