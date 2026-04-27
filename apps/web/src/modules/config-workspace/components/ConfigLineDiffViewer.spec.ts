import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import ConfigLineDiffViewer from "./ConfigLineDiffViewer.vue";

function mountDiff(beforeContent: string, afterContent: string) {
  return mount(ConfigLineDiffViewer, {
    props: {
      beforeContent,
      afterContent,
      beforeTitle: "Before",
      afterTitle: "After",
    },
  });
}

describe("ConfigLineDiffViewer", () => {
  it("renders unchanged lines without changed segments", () => {
    const wrapper = mountDiff(
      "greeting: hello\nfeature: false",
      "greeting: hello\nfeature: false",
    );

    expect(wrapper.text()).toContain("greeting: hello");
    expect(wrapper.text()).toContain("feature: false");
    expect(
      wrapper.find(".config-line-diff-viewer__segment.is-changed").exists(),
    ).toBe(false);
  });

  it("highlights the changed segment inside paired modified lines", () => {
    const wrapper = mountDiff(
      "greeting: hello-release-test",
      "greeting: hello-release-test-v2",
    );

    const changedSegments = wrapper.findAll(
      ".config-line-diff-viewer__segment.is-changed",
    );

    expect(changedSegments).toHaveLength(1);
    expect(changedSegments[0].text()).toBe("-v2");
    expect(
      wrapper.find(".config-line-diff-viewer__cell.is-added").text(),
    ).toContain("greeting: hello-release-test-v2");
  });

  it("marks pure additions and removals with row classes", () => {
    const wrapper = mountDiff(
      "greeting: hello\nlegacy: true",
      "greeting: hello\nfeature: enabled",
    );

    expect(
      wrapper.find(".config-line-diff-viewer__cell.is-removed").text(),
    ).toContain("legacy: true");
    expect(
      wrapper.find(".config-line-diff-viewer__cell.is-added").text(),
    ).toContain("feature: enabled");
  });
});
