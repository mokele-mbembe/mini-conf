export const ROUTE_NAMES = {
  LOGIN: "Login",
  PROJECTS: "Projects",
  PROJECT_OVERVIEW: "ProjectOverview",
} as const;

export const ROUTE_PATHS = {
  LOGIN: "/login",
  PROJECTS: "/projects",
  PROJECT: "/projects/:projectId",
} as const;
