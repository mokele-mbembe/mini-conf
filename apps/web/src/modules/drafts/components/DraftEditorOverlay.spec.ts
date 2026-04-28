import { mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import DraftEditorOverlay from "./DraftEditorOverlay.vue";

const requestClose = vi.fn();

function mountOverlay(
  props?: Partial<InstanceType<typeof DraftEditorOverlay>["$props"]>,
) {
  return mount(DraftEditorOverlay, {
    props: {
      visible: true,
      configFileId: 3,
      ...props,
    },
    global: {
      stubs: {
        teleport: true,
        DraftEditorPage: {
          name: "DraftEditorPage",
          props: {
            embedded: Boolean,
            configFileIdOverride: Number,
          },
          emits: ["close", "switch-config"],
          setup(_props, { emit, expose }) {
            requestClose.mockImplementation(() => emit("close"));
            expose({ requestClose });
          },
          template: `
            <div class="draft-page-stub">
              <button data-test="close" @click="$emit('close')">close</button>
              <button data-test="switch" @click="$emit('switch-config', 4)">switch</button>
            </div>
          `,
        },
      },
    },
  });
}

describe("DraftEditorOverlay", () => {
  beforeEach(() => {
    requestClose.mockReset();
  });

  it("renders the embedded draft editor with the selected config file", () => {
    const wrapper = mountOverlay();
    const editor = wrapper.findComponent({ name: "DraftEditorPage" });

    expect(editor.exists()).toBe(true);
    expect(editor.props("embedded")).toBe(true);
    expect(editor.props("configFileIdOverride")).toBe(3);
    expect(wrapper.find(".draft-editor-overlay").attributes("role")).toBe(
      "dialog",
    );
    expect(wrapper.find(".draft-editor-overlay").attributes("aria-modal")).toBe(
      "true",
    );
  });

  it("does not render when hidden or missing a config file", () => {
    expect(
      mountOverlay({ visible: false }).find(".draft-editor-overlay").exists(),
    ).toBe(false);
    expect(
      mountOverlay({ configFileId: null })
        .find(".draft-editor-overlay")
        .exists(),
    ).toBe(false);
  });

  it("forwards close and config switch events", async () => {
    const wrapper = mountOverlay();

    await wrapper.find('[data-test="close"]').trigger("click");
    await wrapper.find('[data-test="switch"]').trigger("click");

    expect(wrapper.emitted("request-close")).toHaveLength(1);
    expect(wrapper.emitted("switch-config")).toEqual([[4]]);
  });

  it("routes backdrop and Escape close through the embedded editor guard", async () => {
    const wrapper = mountOverlay();

    await wrapper.find(".draft-editor-overlay__backdrop").trigger("click");
    await wrapper
      .find(".draft-editor-overlay")
      .trigger("keydown", { key: "Escape" });

    expect(requestClose).toHaveBeenCalledTimes(2);
    expect(wrapper.emitted("request-close")).toHaveLength(2);
  });

  it("locks page scrolling while visible and restores it on unmount", () => {
    const originalOverflow = document.body.style.overflow;
    document.body.style.overflow = "auto";

    const wrapper = mountOverlay();

    expect(document.body.style.overflow).toBe("hidden");
    wrapper.unmount();
    expect(document.body.style.overflow).toBe("auto");

    document.body.style.overflow = originalOverflow;
  });
});
