import type { RouteRecordRaw } from "vue-router";
import { ROUTE_NAMES, ROUTE_PATHS } from "@/shared/constants/routes";

import AuthLayout from "@/app/layouts/AuthLayout.vue";
import AppShell from "@/app/layouts/AppShell.vue";

import LoginPage from "@/modules/auth/pages/LoginPage.vue";
import ProjectListPage from "@/modules/projects/pages/ProjectListPage.vue";
import ProjectOverviewPage from "@/modules/projects/pages/ProjectOverviewPage.vue";

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
    ],
  },
  {
    path: "/:pathMatch(.*)*",
    redirect: ROUTE_PATHS.PROJECTS,
  },
];
