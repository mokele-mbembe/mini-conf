import type { RouteRecordRaw } from "vue-router";
import { ROUTE_NAMES, ROUTE_PATHS } from "@/shared/constants/routes";

import AuthLayout from "@/app/layouts/AuthLayout.vue";
import AppShell from "@/app/layouts/AppShell.vue";

import LoginPage from "@/modules/auth/pages/LoginPage.vue";
import ProjectListPage from "@/modules/projects/pages/ProjectListPage.vue";
import ProjectOverviewPage from "@/modules/projects/pages/ProjectOverviewPage.vue";
import ProjectSectionPlaceholderPage from "@/modules/projects/pages/ProjectSectionPlaceholderPage.vue";
import ConfigFileListPage from "@/modules/config-files/pages/ConfigFileListPage.vue";
import ProjectEnvironmentListPage from "@/modules/project-environments/pages/ProjectEnvironmentListPage.vue";
import DeploymentInstanceListPage from "@/modules/deployments/pages/DeploymentInstanceListPage.vue";
import DeploymentInstanceDetailPage from "@/modules/deployments/pages/DeploymentInstanceDetailPage.vue";
import DeploymentPreviewPage from "@/modules/deployments/pages/DeploymentPreviewPage.vue";
import DraftEditorPage from "@/modules/drafts/pages/DraftEditorPage.vue";
import ReleaseListPage from "@/modules/releases/pages/ReleaseListPage.vue";

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
        component: ProjectSectionPlaceholderPage,
      },
      {
        path: ROUTE_PATHS.RELEASE_DIFF,
        name: ROUTE_NAMES.RELEASE_DIFF,
        component: ProjectSectionPlaceholderPage,
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
    path: "/:pathMatch(.*)*",
    redirect: ROUTE_PATHS.PROJECTS,
  },
];
