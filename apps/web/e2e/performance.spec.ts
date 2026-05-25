import { test, expect } from "@playwright/test";
import type { Page } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

const ADMIN_USERNAME = process.env.E2E_ADMIN_USERNAME ?? "admin";
const ADMIN_PASSWORD = process.env.E2E_ADMIN_PASSWORD ?? "admin123456";
const RESULT_FILE =
  process.env.PERF_WEB_RESULT_FILE ??
  resolve(process.cwd(), "../../target/perf/web-route.json");
const MAX_ROUTE_MS = Number(process.env.PERF_WEB_MAX_ROUTE_MS ?? "250");
const MAX_API_MS = Number(process.env.PERF_WEB_MAX_API_MS ?? "100");

interface PerfEntry {
  kind: "api" | "route";
  name: string;
  duration_ms: number;
  started_at: number;
  ended_at: number;
  status?: number;
  ok?: boolean;
  from?: string;
  to?: string;
}

declare global {
  interface Window {
    __MINI_CONF_PERF__?: {
      snapshot(): PerfEntry[];
    };
  }
}

test("records route and API timing entries in production smoke flow", async ({
  page,
}) => {
  await page.goto("/login");
  await expect(page.getByPlaceholder("请输入用户名")).toBeVisible();
  await page.getByPlaceholder("请输入用户名").fill(ADMIN_USERNAME);
  await page.getByPlaceholder("请输入密码").fill(ADMIN_PASSWORD);
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page).toHaveURL(/\/(projects|admin|setup|change-password)/, {
    timeout: 10_000,
  });

  await completeSetupIfNeeded(page);
  await page.goto("/projects");
  await expect(page).toHaveURL(/\/projects$/, { timeout: 10_000 });
  await expect(page.locator(".project-list-page")).toBeVisible({
    timeout: 10_000,
  });

  const entries = await page.evaluate<PerfEntry[]>(
    () => window.__MINI_CONF_PERF__?.snapshot() ?? [],
  );
  const routeEntries = entries.filter((entry) => entry.kind === "route");
  const apiEntries = entries.filter((entry) => entry.kind === "api");

  expect(routeEntries.length).toBeGreaterThan(0);
  expect(apiEntries.length).toBeGreaterThan(0);
  expect(apiEntries.every((entry) => entry.status !== 0)).toBeTruthy();

  const report = {
    mode: "real",
    flow: "login -> projects",
    thresholds: {
      max_route_ms: MAX_ROUTE_MS,
      max_api_ms: MAX_API_MS,
    },
    route_count: routeEntries.length,
    api_count: apiEntries.length,
    max_route_ms: maxDuration(routeEntries),
    max_api_ms: maxDuration(apiEntries),
    route_entries: routeEntries,
    api_entries: apiEntries,
    violations: [] as Array<{
      metric: string;
      actual: number;
      threshold: number;
    }>,
  };
  if (report.max_route_ms > MAX_ROUTE_MS) {
    report.violations.push({
      metric: "max_route_ms",
      actual: report.max_route_ms,
      threshold: MAX_ROUTE_MS,
    });
  }
  if (report.max_api_ms > MAX_API_MS) {
    report.violations.push({
      metric: "max_api_ms",
      actual: report.max_api_ms,
      threshold: MAX_API_MS,
    });
  }

  mkdirSync(dirname(RESULT_FILE), { recursive: true });
  writeFileSync(RESULT_FILE, `${JSON.stringify(report, null, 2)}\n`);
  expect(report.violations).toEqual([]);
});

function maxDuration(entries: PerfEntry[]): number {
  return entries.reduce((max, entry) => Math.max(max, entry.duration_ms), 0);
}

async function completeSetupIfNeeded(page: Page): Promise<void> {
  if (!page.url().includes("/setup")) {
    return;
  }

  const suffix = Date.now().toString();
  const userResponse = await postWithCsrf(page, "/api/admin/users", {
    data: {
      username: `e2e-perf-setup-admin-${suffix}`,
      password: "Seed12345",
      status: "active",
      is_platform_admin: false,
      must_change_password: true,
    },
  });
  await requireOk(userResponse, "create setup admin");
  const createdUser = (await userResponse.json()) as { id: number };

  const projectResponse = await postWithCsrf(page, "/api/admin/projects", {
    data: {
      code: `e2e-perf-setup-project-${suffix}`,
      name: `E2E Perf Setup Project ${suffix}`,
      description: "Created by the performance smoke setup helper",
      initial_admin_user_id: createdUser.id,
    },
  });
  await requireOk(projectResponse, "create setup project");

  const completeResponse = await postWithCsrf(page, "/api/setup/complete", {});
  await requireOk(completeResponse, "complete setup");
}

async function postWithCsrf(
  page: Page,
  url: string,
  options: Parameters<Page["request"]["post"]>[1],
) {
  const csrfToken = await fetchCsrfToken(page);
  return page.request.post(url, {
    ...options,
    headers: {
      ...options?.headers,
      "X-CSRF-Token": csrfToken,
    },
  });
}

async function fetchCsrfToken(page: Page): Promise<string> {
  const response = await page.request.get("/api/auth/csrf");
  await requireOk(response, "fetch csrf token");
  const setCookie = response.headers()["set-cookie"] ?? "";
  const match = /mini_conf_csrf=([^;]+)/.exec(setCookie);
  if (!match) {
    throw new Error("csrf response did not set mini_conf_csrf cookie");
  }
  return match[1];
}

async function requireOk(
  response: Awaited<ReturnType<Page["request"]["post"]>>,
  label: string,
): Promise<void> {
  if (!response.ok()) {
    throw new Error(
      `${label} failed: ${response.status()} ${await response.text()}`,
    );
  }
}
