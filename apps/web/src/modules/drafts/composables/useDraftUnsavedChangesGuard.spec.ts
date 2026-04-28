import { computed, ref } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";

const mockState = vi.hoisted(() => ({
  confirm: vi.fn(),
  onBeforeRouteLeave: vi.fn(),
}));

vi.mock("element-plus", () => ({
  ElMessageBox: {
    confirm: mockState.confirm,
  },
}));

vi.mock("vue-router", () => ({
  onBeforeRouteLeave: mockState.onBeforeRouteLeave,
}));

import { useDraftUnsavedChangesGuard } from "./useDraftUnsavedChangesGuard";

function createGuard(options?: { dirty?: boolean }) {
  const isDirty = ref(options?.dirty ?? false);
  const guard = useDraftUnsavedChangesGuard({
    isDirty: computed(() => isDirty.value),
    t: (key) => key,
  });

  return {
    guard,
    isDirty,
  };
}

describe("useDraftUnsavedChangesGuard", () => {
  beforeEach(() => {
    mockState.confirm.mockReset();
    mockState.onBeforeRouteLeave.mockReset();
  });

  it("allows navigation immediately when content is clean", async () => {
    const { guard } = createGuard();

    await expect(guard.confirmIfDirty()).resolves.toBe(true);
    expect(mockState.confirm).not.toHaveBeenCalled();
  });

  it("asks for confirmation when content is dirty", async () => {
    mockState.confirm.mockResolvedValue(undefined);
    const { guard } = createGuard({ dirty: true });

    await expect(guard.confirmIfDirty()).resolves.toBe(true);

    expect(mockState.confirm).toHaveBeenCalledWith(
      "drafts.navigate.prompt",
      "drafts.navigate.title",
      expect.objectContaining({
        confirmButtonText: "drafts.navigate.confirm",
        cancelButtonText: "common.cancel",
      }),
    );
  });

  it("blocks navigation when the dirty confirmation is cancelled", async () => {
    mockState.confirm.mockRejectedValue("cancel");
    const { guard } = createGuard({ dirty: true });

    await expect(guard.confirmIfDirty()).resolves.toBe(false);
  });

  it("registers the route-leave guard", async () => {
    mockState.confirm.mockResolvedValue(undefined);
    createGuard({ dirty: true });

    const routeGuard = mockState.onBeforeRouteLeave.mock.calls[0]?.[0];

    expect(routeGuard).toBeTypeOf("function");
    await expect(routeGuard()).resolves.toBe(true);
    expect(mockState.confirm).toHaveBeenCalledOnce();
  });

  it("prevents browser unload when content is dirty", () => {
    const preventDefault = vi.fn();
    const { guard } = createGuard({ dirty: true });
    const event: { preventDefault: () => void; returnValue?: string } = {
      preventDefault,
    };

    guard.handleBeforeUnload(event);

    expect(preventDefault).toHaveBeenCalledOnce();
    expect(event.returnValue).toBe("");
  });
});
