import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import DraftWorkspaceSummary from "./DraftWorkspaceSummary.vue";
import type { ConfigFileSummary } from "@/api/types/config-file";
import type { DeploymentInstanceSummary } from "@/api/types/deployment-instance";

const deployment: DeploymentInstanceSummary = {
  id: 1,
  deployment_uid: "dep-uid",
  project_id: 10,
  environment_id: 20,
  environment_code: "prod",
  environment_name: "Production",
  deployment_key: "store-001",
  name: "Store 001",
  description: null,
  is_template: false,
  template_source_id: null,
  status: "active",
  is_archived: false,
  archived_at: null,
  archived_by: null,
  archive_reason: null,
  deleted_at: null,
  deleted_by: null,
  delete_reason: null,
};

const configFile: ConfigFileSummary = {
  id: 2,
  project_id: 10,
  code: "main",
  name: "Main",
  format: "yaml",
  sensitivity: "normal",
  is_required: true,
  status: "active",
  description: null,
  secret_paths: null,
};

function mountSummary(
  props?: Partial<InstanceType<typeof DraftWorkspaceSummary>["$props"]>,
) {
  return mount(DraftWorkspaceSummary, {
    props: {
      deployment,
      configFile,
      versionLabel: "4",
      canEdit: true,
      draftReady: true,
      discarding: false,
      restoring: false,
      ...props,
    },
    global: {
      stubs: {
        "el-button": {
          emits: ["click"],
          template:
            '<button class="button-stub" type="button" @click="$emit(\'click\')"><slot /></button>',
        },
        "el-descriptions": {
          template: '<dl class="descriptions-stub"><slot /></dl>',
        },
        "el-descriptions-item": {
          props: ["label"],
          template:
            '<div class="description-item-stub"><dt>{{ label }}</dt><dd><slot /></dd></div>',
        },
        "el-tag": {
          template: '<span class="tag-stub"><slot /></span>',
        },
      },
    },
  });
}

describe("DraftWorkspaceSummary", () => {
  it("renders deployment, config file, and draft version metadata", () => {
    const wrapper = mountSummary();

    expect(wrapper.text()).toContain("Store 001");
    expect(wrapper.text()).toContain("store-001");
    expect(wrapper.text()).toContain("实例");
    expect(wrapper.text()).toContain("main");
    expect(wrapper.text()).toContain("yaml");
    expect(wrapper.text()).toContain("Draft 版本");
    expect(wrapper.text()).toContain("4");
  });

  it("renders template deployment type", () => {
    const wrapper = mountSummary({
      deployment: {
        ...deployment,
        is_template: true,
      },
    });

    expect(wrapper.text()).toContain("模板");
  });

  it("emits draft action events when editable", async () => {
    const wrapper = mountSummary();
    const buttons = wrapper.findAll(".button-stub");

    await buttons[0].trigger("click");
    await buttons[1].trigger("click");
    await buttons[2].trigger("click");

    expect(wrapper.emitted("discard")).toHaveLength(1);
    expect(wrapper.emitted("restore-from-release")).toHaveLength(1);
    expect(wrapper.emitted("clone-from-instance")).toHaveLength(1);
  });

  it("hides draft actions when the user cannot edit", () => {
    const wrapper = mountSummary({
      canEdit: false,
    });

    expect(wrapper.find(".draft-editor-page__draft-actions").exists()).toBe(
      false,
    );
  });

  it("hides discard action until a draft exists", () => {
    const wrapper = mountSummary({
      draftReady: false,
    });

    expect(
      wrapper.findAll(".button-stub").map((button) => button.text()),
    ).toEqual(["从最新 Release 恢复", "从其他实例复制"]);
  });
});
