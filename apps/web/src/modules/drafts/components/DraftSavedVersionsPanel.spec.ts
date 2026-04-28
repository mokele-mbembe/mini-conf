import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import DraftSavedVersionsPanel from "./DraftSavedVersionsPanel.vue";
import { ApiRequestError } from "@/api/error";
import type {
  SavedVersionDetail,
  SavedVersionSummary,
} from "@/api/types/saved-version";

const savedVersions: SavedVersionSummary[] = [
  {
    id: 11,
    project_id: 1,
    deployment_instance_id: 2,
    config_file_id: 3,
    title: "Auto save 11",
    note: "before release",
    format: "yaml",
    source_draft_version: 4,
    created_by: 5,
    created_by_username: "alice",
    created_at: "2026-04-27T10:00:00Z",
  },
  {
    id: 12,
    project_id: 1,
    deployment_instance_id: 2,
    config_file_id: 3,
    title: "Auto save 12",
    note: null,
    format: "yaml",
    source_draft_version: 5,
    created_by: 6,
    created_by_username: "bob",
    created_at: "2026-04-27T11:00:00Z",
  },
];

const savedVersionDetail: SavedVersionDetail = {
  ...savedVersions[0],
  content: "greeting: hello",
};

function mountPanel(
  props?: Partial<InstanceType<typeof DraftSavedVersionsPanel>["$props"]>,
) {
  return mount(DraftSavedVersionsPanel, {
    props: {
      savedVersions: [],
      loading: false,
      error: null,
      selectedSavedVersionId: null,
      savedVersionDetail: null,
      detailLoading: false,
      note: "",
      noteMaxLength: 500,
      updatingNote: false,
      restoring: false,
      deleting: false,
      ...props,
    },
    global: {
      stubs: {
        "el-alert": {
          props: ["title", "description"],
          template:
            '<div class="alert-stub"><span>{{ title }}</span><span>{{ description }}</span></div>',
        },
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
        "el-divider": {
          template: '<hr class="divider-stub" />',
        },
        "el-empty": {
          props: ["description"],
          template: '<div class="empty-stub">{{ description }}</div>',
        },
        "el-input": {
          props: ["modelValue"],
          emits: ["update:modelValue"],
          template:
            '<textarea class="input-stub" :value="modelValue" @input="$emit(\'update:modelValue\', $event.target.value)" />',
        },
        "el-skeleton": {
          template: '<div class="skeleton-stub" />',
        },
        "el-tag": {
          template: '<span class="tag-stub"><slot /></span>',
        },
      },
    },
  });
}

describe("DraftSavedVersionsPanel", () => {
  it("renders loading and empty states", () => {
    const loadingWrapper = mountPanel({ loading: true });
    expect(loadingWrapper.find(".skeleton-stub").exists()).toBe(true);

    const emptyWrapper = mountPanel();
    expect(emptyWrapper.find(".empty-stub").text()).toBe("暂无 Saved Version");
  });

  it("renders API load errors with the translated error detail", () => {
    const wrapper = mountPanel({
      error: new ApiRequestError(404, {
        code: "saved_version_not_found",
        message: "Saved version not found",
      }),
    });

    expect(wrapper.find(".alert-stub").text()).toContain(
      "加载 Saved Versions 失败",
    );
    expect(wrapper.find(".alert-stub").text()).toContain(
      "Saved Version 不存在",
    );
  });

  it("renders version list and selected detail", () => {
    const wrapper = mountPanel({
      savedVersions,
      selectedSavedVersionId: 11,
      savedVersionDetail,
      note: "before release",
    });

    expect(wrapper.find(".tag-stub").text()).toBe("2");
    expect(wrapper.text()).toContain("Auto save 11");
    expect(wrapper.text()).toContain("Auto save 12");
    expect(wrapper.text()).toContain("Draft 版本 4");
    expect(wrapper.text()).toContain("alice");
    expect(wrapper.text()).toContain("标题");
    expect(wrapper.text()).toContain("保存人");
    expect(
      wrapper.find(".draft-saved-versions-panel__item.is-active").text(),
    ).toContain("Auto save 11");
  });

  it("emits selection, note, and action events", async () => {
    const wrapper = mountPanel({
      savedVersions,
      selectedSavedVersionId: 11,
      savedVersionDetail,
      note: "before release",
    });

    await wrapper
      .findAll(".draft-saved-versions-panel__item")[1]
      .trigger("click");
    await wrapper.find(".input-stub").setValue("ready to publish");

    const buttons = wrapper.findAll(".button-stub");
    await buttons[0].trigger("click");
    await buttons[1].trigger("click");
    await buttons[2].trigger("click");

    expect(wrapper.emitted("select")).toEqual([[12]]);
    expect(wrapper.emitted("update:note")).toEqual([["ready to publish"]]);
    expect(wrapper.emitted("save-note")).toHaveLength(1);
    expect(wrapper.emitted("restore")).toHaveLength(1);
    expect(wrapper.emitted("delete")).toHaveLength(1);
  });
});
