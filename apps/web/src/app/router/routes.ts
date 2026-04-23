import type { RouteRecordRaw } from "vue-router";
import { ROUTE_NAMES, ROUTE_PATHS } from "@/shared/constants/routes";

import AuthLayout from "@/app/layouts/AuthLayout.vue";
import AppShell from "@/app/layouts/AppShell.vue";
import AdminLayout from "@/app/layouts/AdminLayout.vue";
import ProjectSectionPlaceholderPage from "@/modules/projects/pages/ProjectSectionPlaceholderPage.vue";

const LoginPage = () => import("@/modules/auth/pages/LoginPage.vue");
const ChangePasswordPage = () =>
  import("@/modules/auth/pages/ChangePasswordPage.vue");
const SetupPage = () => import("@/modules/setup/pages/SetupPage.vue");
const ProjectListPage = () =>
  import("@/modules/projects/pages/ProjectListPage.vue");
const ProjectOverviewPage = () =>
  import("@/modules/projects/pages/ProjectOverviewPage.vue");
const ConfigFileListPage = () =>
  import("@/modules/config-files/pages/ConfigFileListPage.vue");
const ProjectEnvironmentListPage = () =>
  import("@/modules/project-environments/pages/ProjectEnvironmentListPage.vue");
const DeploymentInstanceListPage = () =>
  import("@/modules/deployments/pages/DeploymentInstanceListPage.vue");
const DeploymentInstanceDetailPage = () =>
  import("@/modules/deployments/pages/DeploymentInstanceDetailPage.vue");
const DeploymentPreviewPage = () =>
  import("@/modules/deployments/pages/DeploymentPreviewPage.vue");
const DraftEditorPage = () =>
  import("@/modules/drafts/pages/DraftEditorPage.vue");
const ReleaseListPage = () =>
  import("@/modules/releases/pages/ReleaseListPage.vue");
const ReleaseDetailPage = () =>
  import("@/modules/releases/pages/ReleaseDetailPage.vue");
const ReleaseDiffPage = () =>
  import("@/modules/releases/pages/ReleaseDiffPage.vue");

// Admin pages
const AdminUserListPage = () =>
  import("@/modules/admin-users/pages/AdminUserListPage.vue");
const AdminProjectListPage = () =>
  import("@/modules/admin-projects/pages/AdminProjectListPage.vue");
const AdminProjectCreatePage = () =>
  import("@/modules/admin-projects/pages/AdminProjectCreatePage.vue");

export const routes: RouteRecordRaw[] = [
  {
    path: ROUTE_PATHS.LOGIN,
    component: AuthLayout,
    meta: { requiresAuth: false },
    children: [
      {
        path: "",
        name: ROUTE_NAMES.LOGIN,
        component: LoginPage,
        meta: { requiresAuth: false },
      },
    ],
  },
  {
    path: ROUTE_PATHS.SETUP,
    component: AuthLayout,
    meta: { requiresAuth: false },
    children: [
      {
        path: "",
        name: ROUTE_NAMES.SETUP,
        component: SetupPage,
        meta: { requiresAuth: false },
      },
    ],
  },
  {
    path: ROUTE_PATHS.CHANGE_PASSWORD,
    component: AuthLayout,
    meta: { requiresAuth: true },
    children: [
      {
        path: "",
        name: ROUTE_NAMES.CHANGE_PASSWORD,
        component: ChangePasswordPage,
      },
    ],
  },
  {
    path: "/",
    component: AppShell,
    meta: { requiresAuth: true },
    children: [
      {
        path: "",
        redirect: ROUTE_PATHS.PROJECTS,
      },
      {
        path: ROUTE_PATHS.PROJECTS,
        name: ROUTE_NAMES.PROJECTS,
        component: ProjectListPage,
      },
      {
        path: ROUTE_PATHS.PROJECT,
        name: ROUTE_NAMES.PROJECT_OVERVIEW,
        component: ProjectOverviewPage,
      },
      {
        path: ROUTE_PATHS.CONFIG_FILE_LIST,
        name: ROUTE_NAMES.CONFIG_FILE_LIST,
        component: ConfigFileListPage,
      },
      {
        path: ROUTE_PATHS.PROJECT_ENVIRONMENT_LIST,
        name: ROUTE_NAMES.PROJECT_ENVIRONMENT_LIST,
        component: ProjectEnvironmentListPage,
      },
      {
        path: ROUTE_PATHS.DEPLOYMENT_LIST,
        name: ROUTE_NAMES.DEPLOYMENT_LIST,
        component: DeploymentInstanceListPage,
      },
      {
        path: ROUTE_PATHS.DEPLOYMENT_DETAIL,
        name: ROUTE_NAMES.DEPLOYMENT_DETAIL,
        component: DeploymentInstanceDetailPage,
      },
      {
        path: ROUTE_PATHS.DEPLOYMENT_PREVIEW,
        name: ROUTE_NAMES.DEPLOYMENT_PREVIEW,
        component: DeploymentPreviewPage,
      },
      {
        path: ROUTE_PATHS.DRAFT_EDITOR,
        name: ROUTE_NAMES.DRAFT_EDITOR,
        component: DraftEditorPage,
      },
      {
        path: ROUTE_PATHS.RELEASE_LIST,
        name: ROUTE_NAMES.RELEASE_LIST,
        component: ReleaseListPage,
      },
      {
        path: ROUTE_PATHS.RELEASE_DETAIL,
        name: ROUTE_NAMES.RELEASE_DETAIL,
        component: ReleaseDetailPage,
      },
      {
        path: ROUTE_PATHS.RELEASE_DIFF,
        name: ROUTE_NAMES.RELEASE_DIFF,
        component: ReleaseDiffPage,
      },
      {
        path: ROUTE_PATHS.PROJECT_MEMBERS,
        name: ROUTE_NAMES.PROJECT_MEMBERS,
        component: ProjectSectionPlaceholderPage,
      },
      {
        path: ROUTE_PATHS.SYNC_RECORD_LIST,
        name: ROUTE_NAMES.SYNC_RECORD_LIST,
        component: ProjectSectionPlaceholderPage,
      },
      {
        path: ROUTE_PATHS.HEARTBEAT_LIST,
        name: ROUTE_NAMES.HEARTBEAT_LIST,
        component: ProjectSectionPlaceholderPage,
      },
      {
        path: ROUTE_PATHS.AUDIT_LOG_LIST,
        name: ROUTE_NAMES.AUDIT_LOG_LIST,
        component: ProjectSectionPlaceholderPage,
      },
    ],
  },
  {
    path: ROUTE_PATHS.ADMIN_DASHBOARD,
    name: ROUTE_NAMES.ADMIN_DASHBOARD,
    component: AdminLayout,
    meta: { requiresAuth: true, requiresPlatformAdmin: true },
    children: [
      {
        path: "",
        redirect: ROUTE_PATHS.ADMIN_USERS,
      },
      {
        path: ROUTE_PATHS.ADMIN_USERS,
        name: ROUTE_NAMES.ADMIN_USERS,
        component: AdminUserListPage,
      },
      {
        path: ROUTE_PATHS.ADMIN_PROJECTS,
        name: ROUTE_NAMES.ADMIN_PROJECTS,
        component: AdminProjectListPage,
      },
      {
        path: ROUTE_PATHS.ADMIN_CREATE_PROJECT,
        name: ROUTE_NAMES.ADMIN_CREATE_PROJECT,
        component: AdminProjectCreatePage,
      },
    ],
  },
  {
    path: "/:pathMatch(.*)*",
    redirect: ROUTE_PATHS.PROJECTS,
  },
];

// Add Vue 3 route metadata typing
declare module "vue-router" {
  interface RouteMeta {
    requiresAuth?: boolean;
    requiresPlatformAdmin?: boolean;
  }
}
