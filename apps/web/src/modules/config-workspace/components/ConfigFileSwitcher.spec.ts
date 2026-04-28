import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import ConfigFileSwitcher from "./ConfigFileSwitcher.vue";
import type { ConfigFileSummary } from "@/api/types/config-file";

const configFiles: ConfigFileSummary[] = [
  {
    id: 1,
    project_id: 10,
    code: "main",
    name: "Main",
    format: "yaml",
    sensitivity: "normal",
    is_required: true,
    status: "active",
    description: null,
    secret_paths: null,
  },
  {
    id: 2,
    project_id: 10,
    code: "vision",
    name: "Vision",
    format: "json",
    sensitivity: "secret",
    is_required: false,
    status: "active",
    description: null,
    secret_paths: ["token"],
  },
];

function mountSwitcher(
  props?: Partial<InstanceType<typeof ConfigFileSwitcher>["$props"]>,
) {
  return mount(ConfigFileSwitcher, {
    props: {
      configFiles,
      currentConfigFileId: 1,
      previewStatusMap: {},
      ...props,
    },
    global: {
      stubs: {
        "el-radio-group": {
          name: "ElRadioGroup",
          props: ["modelValue"],
          emits: ["update:modelValue"],
          template:
            '<div class="radio-group-stub" :data-model-value="modelValue"><slot /></div>',
        },
        "el-radio-button": {
          props: ["value"],
          template:
            '<button class="radio-button-stub" type="button" :data-value="value"><slot /></button>',
        },
        "el-tag": {
          props: ["type"],
          template: '<span class="tag-stub" :data-type="type"><slot /></span>',
        },
      },
    },
  });
}

describe("ConfigFileSwitcher", () => {
  it("does not render a switcher when there is only one config file", () => {
    const wrapper = mountSwitcher({
      configFiles: [configFiles[0]],
    });

    expect(wrapper.find(".config-file-switcher").exists()).toBe(false);
  });

  it("renders config codes and preview status labels for multiple config files", () => {
    const wrapper = mountSwitcher({
      previewStatusMap: {
        1: "draft",
        2: "missing_required",
      },
    });

    expect(wrapper.text()).toContain("配置文件");
    expect(wrapper.text()).toContain("main");
    expect(wrapper.text()).toContain("vision");
    expect(wrapper.text()).toContain("Draft");
    expect(wrapper.text()).toContain("必选缺失");
    expect(
      wrapper.findAll(".tag-stub").map((tag) => tag.attributes("data-type")),
    ).toEqual(["warning", "danger"]);
  });

  it("emits switch only for numeric radio group values", async () => {
    const wrapper = mountSwitcher();
    const radioGroup = wrapper.findComponent({ name: "ElRadioGroup" });

    await radioGroup.vm.$emit("update:modelValue", 2);
    await radioGroup.vm.$emit("update:modelValue", "2");

    expect(wrapper.emitted("switch")).toEqual([[2]]);
  });
});
