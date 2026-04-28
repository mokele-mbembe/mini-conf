import { mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import ProjectTabs from "./ProjectTabs.vue";
import { ROUTE_NAMES } from "@/shared/constants/routes";

const mockState = vi.hoisted(() => ({
  project: {
    value: {
      id: 42,
      code: "coffee",
      name: "Coffee",
      status: "active",
      description: null,
      current_user_role: "viewer",
    },
  },
  route: {
    name: "ProjectOverview",
    params: {
      projectId: "42",
    },
  },
  push: vi.fn(),
}));

vi.mock("vue-router", () => ({
  useRoute: () => mockState.route,
  useRouter: () => ({
    push: mockState.push,
  }),
}));

vi.mock("@/modules/projects/composables/useProjectContext", () => ({
  useProjectContext: () => ({
    project: mockState.project,
    loading: { value: false },
    error: { value: null },
    fetchProject: vi.fn(),
  }),
}));

function mountTabs() {
  return mount(ProjectTabs, {
    global: {
      stubs: {
        "el-tabs": {
          props: ["modelValue"],
          template:
            '<div class="tabs-stub" :data-active="modelValue"><slot /></div>',
        },
        "el-tab-pane": {
          props: ["label", "name"],
          template:
            '<span class="tab-pane-stub" :data-name="name">{{ label }}</span>',
        },
      },
    },
  });
}

function tabLabels(wrapper: ReturnType<typeof mountTabs>) {
  return wrapper.findAll(".tab-pane-stub").map((tab) => tab.text());
}

describe("ProjectTabs", () => {
  beforeEach(() => {
    mockState.project.value = {
      id: 42,
      code: "coffee",
      name: "Coffee",
      status: "active",
      description: null,
      current_user_role: "viewer",
    };
    mockState.route.name = ROUTE_NAMES.PROJECT_OVERVIEW;
    mockState.route.params.projectId = "42";
    mockState.push.mockClear();
  });

  it("hides admin-only tabs for non-admin project members", () => {
    const wrapper = mountTabs();

    expect(tabLabels(wrapper)).toEqual([
      "项目概览",
      "配置文件",
      "项目环境",
      "部署实例",
      "发布记录",
      "同步记录",
      "心跳上报",
    ]);
  });

  it("shows member and audit tabs for project admins", () => {
    mockState.project.value = {
      ...mockState.project.value,
      current_user_role: "admin",
    };

    const wrapper = mountTabs();

    expect(tabLabels(wrapper)).toEqual([
      "项目概览",
      "配置文件",
      "项目环境",
      "部署实例",
      "发布记录",
      "项目成员",
      "同步记录",
      "心跳上报",
      "审计日志",
    ]);
  });

  it("keeps deployment detail routes under the deployments tab", () => {
    mockState.route.name = ROUTE_NAMES.DEPLOYMENT_DETAIL;

    const wrapper = mountTabs();

    expect(wrapper.find(".tabs-stub").attributes("data-active")).toBe(
      "deployments",
    );
  });

  it("keeps release diff routes under the releases tab", () => {
    mockState.route.name = ROUTE_NAMES.RELEASE_DIFF;

    const wrapper = mountTabs();

    expect(wrapper.find(".tabs-stub").attributes("data-active")).toBe(
      "releases",
    );
  });
});
