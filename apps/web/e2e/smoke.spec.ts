import { test, expect } from "@playwright/test";

// Smoke test: Login → Project List → Project Detail
// Covers the critical happy path to catch white screens, redirect loops,
// and broken API proxy before merge.

const ADMIN_USERNAME = process.env.E2E_ADMIN_USERNAME ?? "admin";
const ADMIN_PASSWORD = process.env.E2E_ADMIN_PASSWORD ?? "admin123456";

test("main path: login → project list → project detail", async ({ page }) => {
  // 1. Navigate to login page
  await page.goto("/login");
  await expect(page.locator("form")).toBeVisible();

  // 2. Fill credentials and submit
  await page.getByPlaceholder("请输入用户名").fill(ADMIN_USERNAME);
  await page.getByPlaceholder("请输入密码").fill(ADMIN_PASSWORD);
  await page.getByRole("button", { name: "登录" }).click();

  // 3. Arrive at /projects – wait for at least one project card
  await expect(page).toHaveURL(/\/projects/, { timeout: 10_000 });
  const firstCard = page.locator(".el-card").first();
  await expect(firstCard).toBeVisible({ timeout: 10_000 });

  // 4. Click the first project card
  await firstCard.click();

  // 5. Arrive at /projects/:id – verify basic project detail skeleton
  await expect(page).toHaveURL(/\/projects\/\d+/, { timeout: 10_000 });
  // The overview page renders an el-descriptions with project info
  await expect(page.locator(".el-descriptions")).toBeVisible({
    timeout: 10_000,
  });
});
