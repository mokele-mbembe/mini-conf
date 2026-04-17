import { test, expect } from "@playwright/test";
import type { Page } from "@playwright/test";

// Smoke test: Login → Project List → Project Detail
// Covers the critical happy path to catch white screens, redirect loops,
// and broken API proxy before merge.

const ADMIN_USERNAME = process.env.E2E_ADMIN_USERNAME ?? "admin";
const ADMIN_PASSWORD = process.env.E2E_ADMIN_PASSWORD ?? "admin123456";

async function login(page: Page) {
  await page.goto("/login");
  await expect(page.locator("form")).toBeVisible();
  await page.getByPlaceholder("请输入用户名").fill(ADMIN_USERNAME);
  await page.getByPlaceholder("请输入密码").fill(ADMIN_PASSWORD);
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page).toHaveURL(/\/projects/, { timeout: 10_000 });
}

async function getAdminProjectId(page: Page): Promise<number> {
  const response = await page.request.get("/api/projects");
  expect(response.ok()).toBeTruthy();
  const payload = (await response.json()) as {
    items: Array<{ id: number; current_user_role?: string }>;
  };
  const project = payload.items.find(
    (item) => item.current_user_role === "admin",
  );
  expect(project).toBeTruthy();
  return project!.id;
}

test("main path: login → project list → project detail", async ({ page }) => {
  await login(page);

  const firstCard = page.locator(".el-card").first();
  await expect(firstCard).toBeVisible({ timeout: 10_000 });

  await firstCard.click();

  await expect(page).toHaveURL(/\/projects\/\d+/, { timeout: 10_000 });
  await expect(page.locator(".el-descriptions")).toBeVisible({
    timeout: 10_000,
  });
});

test("deployment lifecycle path: environment → inactive deployment → activate → reset → deactivate", async ({
  page,
}) => {
  const suffix = Date.now().toString();
  const environmentCode = `e2e-env-${suffix}`;
  const environmentName = `E2E Env ${suffix}`;
  const deploymentName = `E2E Deployment ${suffix}`;
  const deploymentKey = `e2e-deployment-${suffix}`;

  await login(page);
  const projectId = await getAdminProjectId(page);

  await page.goto(`/projects/${projectId}/environments`);
  await expect(page.getByRole("button", { name: "新建环境" })).toBeVisible();
  await page.getByRole("button", { name: "新建环境" }).click();

  const environmentDialog = page.getByRole("dialog", { name: "新建项目环境" });
  await expect(environmentDialog).toBeVisible();
  await environmentDialog
    .getByPlaceholder("如 prod / staging / dev")
    .fill(environmentCode);
  await environmentDialog
    .getByPlaceholder("如 Production")
    .fill(environmentName);
  await environmentDialog.getByRole("button", { name: "创建" }).click();

  await expect(page.locator(".el-table")).toContainText(environmentCode);

  await page.getByRole("tab", { name: "部署实例" }).click();
  await expect(page).toHaveURL(
    new RegExp(`/projects/${projectId}/deployments`),
  );
  await expect(
    page.getByRole("button", { name: "新建部署实例" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "新建部署实例" }).click();

  const deploymentDialog = page.getByRole("dialog", { name: "新建部署实例" });
  await expect(deploymentDialog).toBeVisible();
  await deploymentDialog
    .getByPlaceholder("如 杭州湖滨银泰 001")
    .fill(deploymentName);
  await deploymentDialog
    .getByPlaceholder("如 a-prod-store-001")
    .fill(deploymentKey);
  await deploymentDialog.getByRole("button", { name: "创建" }).click();

  const deploymentRow = page.locator(".el-table__row", {
    hasText: deploymentKey,
  });
  await expect(deploymentRow).toBeVisible();
  await expect(deploymentRow).toContainText("未启用");
  await deploymentRow.getByRole("button", { name: "激活" }).click();

  const activateConfirmDialog = page.getByRole("dialog", {
    name: "激活部署实例",
  });
  await expect(activateConfirmDialog).toContainText("确认激活部署实例");
  await activateConfirmDialog.getByRole("button", { name: "OK" }).click();

  const activateTokenDialog = page.getByRole("dialog", {
    name: "激活成功，已生成访问凭证",
  });
  await expect(activateTokenDialog).toBeVisible();
  await expect(activateTokenDialog.locator("textarea")).not.toHaveValue("");
  await activateTokenDialog.getByRole("button", { name: "关闭" }).click();

  await expect(deploymentRow).toContainText("启用中");
  await deploymentRow.getByRole("button", { name: "查看" }).click();

  await expect(page).toHaveURL(
    new RegExp(`/projects/${projectId}/deployments/\\d+`),
  );
  await expect(page.getByRole("button", { name: "重置 Token" })).toBeVisible();

  await page.getByRole("button", { name: "重置 Token" }).click();
  const resetConfirmDialog = page.getByRole("dialog", {
    name: "重置访问凭证",
  });
  await expect(resetConfirmDialog).toContainText("确认重置部署实例");
  await resetConfirmDialog.getByRole("button", { name: "OK" }).click();

  const resetTokenDialog = page.getByRole("dialog", {
    name: "访问凭证已重置",
  });
  await expect(resetTokenDialog).toBeVisible();
  await expect(resetTokenDialog.locator("textarea")).not.toHaveValue("");
  await resetTokenDialog.getByRole("button", { name: "关闭" }).click();

  await page.getByRole("button", { name: "停用" }).click();
  const deactivateConfirmDialog = page.getByRole("dialog", {
    name: "停用部署实例",
  });
  await expect(deactivateConfirmDialog).toContainText("确认停用部署实例");
  await deactivateConfirmDialog.getByRole("button", { name: "OK" }).click();

  await expect(page.getByRole("button", { name: "激活" })).toBeVisible();
  await expect(page.locator(".deployment-instance-detail-page")).toContainText(
    "未启用",
  );
});
