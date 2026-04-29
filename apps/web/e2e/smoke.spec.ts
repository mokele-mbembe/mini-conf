import { test, expect } from "@playwright/test";
import type { Locator, Page } from "@playwright/test";

// Smoke test: Login → Project List → Project Detail
// Covers the critical happy path to catch white screens, redirect loops,
// and broken API proxy before merge.

const ADMIN_USERNAME = process.env.E2E_ADMIN_USERNAME ?? "admin";
const ADMIN_PASSWORD = process.env.E2E_ADMIN_PASSWORD ?? "admin123456";
const PLATFORM_ADMIN_USERNAME =
  process.env.E2E_PLATFORM_ADMIN_USERNAME ?? ADMIN_USERNAME;
const PLATFORM_ADMIN_PASSWORD =
  process.env.E2E_PLATFORM_ADMIN_PASSWORD ?? ADMIN_PASSWORD;

interface SessionUser {
  id: number;
  username: string;
  is_platform_admin: boolean;
}

async function fetchCsrfToken(page: Page): Promise<string> {
  const response = await page.request.get("/api/auth/csrf");
  expect(response.ok()).toBeTruthy();
  const setCookie = response.headers()["set-cookie"] ?? "";
  const match = /mini_conf_csrf=([^;]+)/.exec(setCookie);
  expect(match).toBeTruthy();
  return match![1];
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

async function putWithCsrf(
  page: Page,
  url: string,
  options: Parameters<Page["request"]["put"]>[1],
) {
  const csrfToken = await fetchCsrfToken(page);
  return page.request.put(url, {
    ...options,
    headers: {
      ...options?.headers,
      "X-CSRF-Token": csrfToken,
    },
  });
}

async function getCurrentUser(page: Page): Promise<SessionUser> {
  const response = await page.request.get("/api/auth/me");
  expect(response.ok()).toBeTruthy();
  const payload = (await response.json()) as { user: SessionUser };
  return payload.user;
}

async function completeSetupIfNeeded(page: Page): Promise<void> {
  await page.waitForURL(/\/(setup|projects|admin(?:\/users)?)/, {
    timeout: 10_000,
  });

  if (!page.url().includes("/setup")) {
    return;
  }

  const suffix = Date.now().toString();
  const userResponse = await postWithCsrf(page, "/api/admin/users", {
    data: {
      username: `e2e-setup-admin-${suffix}`,
      password: "Seed12345",
      status: "active",
      is_platform_admin: false,
      must_change_password: true,
    },
  });
  expect(userResponse.ok()).toBeTruthy();
  const createdUser = (await userResponse.json()) as { id: number };

  const projectResponse = await postWithCsrf(page, "/api/admin/projects", {
    data: {
      code: `e2e-setup-project-${suffix}`,
      name: `E2E Setup Project ${suffix}`,
      description: "Created by the setup completion helper",
      initial_admin_user_id: createdUser.id,
    },
  });
  expect(projectResponse.ok()).toBeTruthy();

  const completeResponse = await postWithCsrf(page, "/api/setup/complete", {});
  expect(completeResponse.ok()).toBeTruthy();
  await page.goto("/projects");
  await expect(page).toHaveURL(/\/projects$/, { timeout: 10_000 });
}

async function login(page: Page): Promise<SessionUser> {
  await page.goto("/login");
  await expect(page.getByPlaceholder("请输入用户名")).toBeVisible();
  await page.getByPlaceholder("请输入用户名").fill(ADMIN_USERNAME);
  await page.getByPlaceholder("请输入密码").fill(ADMIN_PASSWORD);
  await page.getByRole("button", { name: "登录" }).click();
  await completeSetupIfNeeded(page);
  await page.goto("/projects");
  await expect(page).toHaveURL(/\/projects/, { timeout: 10_000 });
  return getCurrentUser(page);
}

async function loginAsPlatformAdmin(page: Page): Promise<SessionUser> {
  await page.goto("/login");
  await expect(page.getByPlaceholder("请输入用户名")).toBeVisible();
  await page.getByPlaceholder("请输入用户名").fill(PLATFORM_ADMIN_USERNAME);
  await page.getByPlaceholder("请输入密码").fill(PLATFORM_ADMIN_PASSWORD);
  await page.getByRole("button", { name: "登录" }).click();
  await completeSetupIfNeeded(page);
  await page.goto("/admin/users");
  await expect(page.getByRole("button", { name: "新建用户" })).toBeVisible();
  return getCurrentUser(page);
}

async function createAdminUser(page: Page, username: string) {
  const response = await postWithCsrf(page, "/api/admin/users", {
    data: {
      username,
      password: "Seed12345",
      status: "active",
      is_platform_admin: false,
      must_change_password: false,
    },
  });
  expect(response.ok()).toBeTruthy();
}

async function createProject(page: Page, suffix: string): Promise<number> {
  const currentUser = await getCurrentUser(page);
  const response = await postWithCsrf(page, "/api/projects", {
    data: {
      code: `e2e-project-${suffix}`,
      name: `E2E Project ${suffix}`,
      description: "Isolated Playwright project",
      initial_admin_user_id: currentUser.id,
    },
  });
  if (!response.ok()) {
    throw new Error(
      `createProject failed: ${response.status()} ${await response.text()}`,
    );
  }
  const payload = (await response.json()) as { project: { id: number } };
  return payload.project.id;
}

async function createProjectEnvironment(
  page: Page,
  projectId: number,
  suffix: string,
) {
  const code = `e2e-env-${suffix}`;
  const response = await postWithCsrf(
    page,
    `/api/projects/${projectId}/environments`,
    {
      data: {
        code,
        name: `E2E Env ${suffix}`,
        status: "active",
        sort_order: 10,
      },
    },
  );
  expect(response.ok()).toBeTruthy();
  const payload = (await response.json()) as { id: number; code: string };
  return payload;
}

async function createDeploymentInstance(
  page: Page,
  projectId: number,
  environmentId: number,
  suffix: string,
) {
  const deploymentKey = `e2e-deployment-${suffix}`;
  const response = await postWithCsrf(page, "/api/deployment-instances", {
    data: {
      project_id: projectId,
      environment_id: environmentId,
      deployment_key: deploymentKey,
      name: `E2E Deployment ${suffix}`,
      is_template: false,
    },
  });
  expect(response.ok()).toBeTruthy();
  const payload = (await response.json()) as {
    id: number;
    deployment_key: string;
  };
  return payload;
}

async function activateDeploymentInstance(
  page: Page,
  deploymentId: number,
): Promise<string> {
  const response = await postWithCsrf(
    page,
    `/api/deployment-instances/${deploymentId}/activate`,
    {},
  );
  expect(response.ok()).toBeTruthy();
  const payload = (await response.json()) as { token: string };
  return payload.token;
}

function getInitialAdminOption(page: Page, username: string) {
  return page
    .locator(".el-select-dropdown__item")
    .filter({ has: page.getByText(username, { exact: true }) })
    .first();
}

function getDraftEditor(page: Page) {
  return page.locator(".draft-editor-page__editor .cm-content").first();
}

async function fillDraftEditor(page: Page, value: string) {
  const editor = getDraftEditor(page);
  await editor.click();
  await page.keyboard.press(
    process.platform === "darwin" ? "Meta+A" : "Control+A",
  );
  await page.keyboard.insertText(value);
}

async function expectElementCenterInside(locator: Locator, selector: string) {
  const isInside = await locator.evaluate((element, targetSelector) => {
    const rect = element.getBoundingClientRect();
    const topmost = document.elementFromPoint(
      rect.left + rect.width / 2,
      rect.top + rect.height / 2,
    );
    return Boolean(topmost?.closest(targetSelector));
  }, selector);
  expect(isInside).toBeTruthy();
}

test("setup path: platform admin login redirects to setup and can complete", async ({
  page,
}) => {
  const suffix = Date.now().toString();
  const projectAdminUsername = `e2e-wizard-admin-${suffix}`;
  const projectCode = `e2e-wizard-project-${suffix}`;
  const projectName = `E2E Wizard Project ${suffix}`;

  await page.goto("/login");
  await expect(page.getByPlaceholder("请输入用户名")).toBeVisible();
  await page.getByPlaceholder("请输入用户名").fill(PLATFORM_ADMIN_USERNAME);
  await page.getByPlaceholder("请输入密码").fill(PLATFORM_ADMIN_PASSWORD);
  await page.getByRole("button", { name: "登录" }).click();

  await expect(page).toHaveURL(/\/setup$/, { timeout: 10_000 });
  await expect(page.getByRole("heading", { name: "系统初始化" })).toBeVisible();
  await expect(page.getByText("系统尚未完成初始化")).toBeVisible();

  await page.getByPlaceholder("请输入用户名").fill(projectAdminUsername);
  await page.getByPlaceholder("请输入密码").fill("Seed12345");
  await page.getByRole("button", { name: "创建项目管理员" }).click();
  await expect(page.getByText("项目管理员已创建").first()).toBeVisible();

  await page.getByPlaceholder("如 coffee-main").fill(projectCode);
  await page.getByPlaceholder("如 Coffee Main").fill(projectName);
  await page.getByPlaceholder("可选描述").fill("Created by setup wizard smoke");
  await page.getByRole("button", { name: "创建项目", exact: true }).click();
  await expect(page.getByText("项目已创建").first()).toBeVisible();

  await page.getByRole("button", { name: "标记初始化完成" }).click();
  await expect(page.getByText("系统初始化已标记为完成")).toBeVisible();

  await page.getByRole("button", { name: "进入项目列表" }).click();
  await expect(page).toHaveURL(/\/projects$/, { timeout: 10_000 });
});

test("main path: login → project list → project detail", async ({ page }) => {
  const suffix = Date.now().toString();
  await login(page);
  const projectId = await createProject(page, suffix);

  await page.goto("/projects");

  const projectCard = page.locator(".el-card", {
    hasText: `e2e-project-${suffix}`,
  });
  await expect(projectCard).toBeVisible({ timeout: 10_000 });

  await projectCard.click();

  await expect(page).toHaveURL(new RegExp(`/projects/${projectId}`), {
    timeout: 10_000,
  });
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
  const projectId = await createProject(page, suffix);

  await page.goto(`/projects/${projectId}/environments`);
  await expect(page).toHaveURL(
    new RegExp(`/projects/${projectId}/environments`),
    {
      timeout: 10_000,
    },
  );
  const createEnvironmentButton = page
    .getByRole("button", { name: "新建环境" })
    .first();
  await expect(createEnvironmentButton).toBeVisible({ timeout: 10_000 });
  await createEnvironmentButton.click();

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
  const createDeploymentButton = page
    .getByRole("button", { name: "新建部署实例" })
    .first();
  await expect(createDeploymentButton).toBeVisible();
  await createDeploymentButton.click();

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

test("admin user dialogs retain form values after failed submit", async ({
  page,
}) => {
  const suffix = Date.now().toString();
  const username = `e2e-user-${suffix}`;

  await loginAsPlatformAdmin(page);
  await createAdminUser(page, username);
  await page.goto("/admin/users");

  await page.getByRole("button", { name: "新建用户" }).click();

  const createDialog = page.getByRole("dialog", { name: "新建用户" });
  await expect(createDialog).toBeVisible();
  await createDialog.getByPlaceholder("请输入用户名").fill(username);
  await createDialog.getByPlaceholder("请输入密码").fill("abc12345");
  await createDialog.getByRole("button", { name: "创建" }).click();

  await expect(page.locator(".el-message").last()).toContainText(
    "用户名已存在",
  );
  await expect(createDialog).toBeVisible();
  await expect(createDialog.getByPlaceholder("请输入用户名")).toHaveValue(
    username,
  );
  await expect(createDialog.getByPlaceholder("请输入密码")).toHaveValue(
    "abc12345",
  );

  await createDialog.getByRole("button", { name: "取消" }).click();
  await expect(createDialog).not.toBeVisible();

  await page.getByPlaceholder("搜索用户名").fill(username);
  const userRow = page.locator(".el-table__row", { hasText: username }).first();
  await expect(userRow).toBeVisible();
  await userRow.getByRole("button", { name: "操作" }).click();
  await page.getByRole("menuitem", { name: "重置密码" }).click();

  const resetDialog = page.getByRole("dialog", { name: "重置密码" });
  await expect(resetDialog).toBeVisible();
  const mustChangePasswordCheckbox = resetDialog.locator("label.el-checkbox", {
    hasText: "下次登录时必须修改密码",
  });
  const mustChangePasswordCheckboxInput = mustChangePasswordCheckbox.locator(
    'input[type="checkbox"]',
  );
  await expect(mustChangePasswordCheckboxInput).toBeChecked();
  await mustChangePasswordCheckbox.click();
  await expect(mustChangePasswordCheckboxInput).not.toBeChecked();

  await resetDialog.getByPlaceholder("请输入新密码").fill("aaaaaaaa");
  await resetDialog.getByRole("button", { name: "确认" }).click();

  await expect(page.locator(".el-message").last()).toContainText(
    "密码强度不足，请使用更强的密码",
  );
  await expect(resetDialog).toBeVisible();
  await expect(resetDialog.getByPlaceholder("请输入新密码")).toHaveValue(
    "aaaaaaaa",
  );
  await expect(mustChangePasswordCheckboxInput).not.toBeChecked();
});

test("must-change-password user changes password before continuing", async ({
  page,
}) => {
  const suffix = Date.now().toString();
  const username = `e2e-must-change-${suffix}`;

  await loginAsPlatformAdmin(page);
  const createResponse = await postWithCsrf(page, "/api/admin/users", {
    data: {
      username,
      password: "Seed12345",
      status: "active",
      is_platform_admin: false,
      must_change_password: true,
    },
  });
  expect(createResponse.ok()).toBeTruthy();

  const logoutResponse = await postWithCsrf(page, "/api/auth/logout", {});
  expect(logoutResponse.ok()).toBeTruthy();

  await page.goto("/login");
  await page.getByPlaceholder("请输入用户名").fill(username);
  await page.getByPlaceholder("请输入密码").fill("Seed12345");
  await page.getByRole("button", { name: "登录" }).click();

  await expect(page).toHaveURL(/\/change-password$/, { timeout: 10_000 });
  await expect(page.getByRole("heading", { name: "修改密码" })).toBeVisible();

  await page.getByPlaceholder("请输入当前密码").fill("Seed12345");
  await page.getByPlaceholder("请输入新密码").fill("NewPassword123");
  await page.getByPlaceholder("请再次输入新密码").fill("NewPassword123");
  await page.getByRole("button", { name: "确认修改" }).click();

  await expect(page.getByText("密码已修改").first()).toBeVisible();
  await expect(page).toHaveURL(/\/projects$/, { timeout: 10_000 });

  const currentUser = await getCurrentUser(page);
  expect(currentUser.username).toBe(username);
});

test("admin project create path: remote search → submit → success state", async ({
  page,
}) => {
  const suffix = Date.now().toString();
  const currentUser = await loginAsPlatformAdmin(page);
  const projectCode = `e2e-admin-project-${suffix}`;
  const projectName = `E2E Admin Project ${suffix}`;

  await page.goto("/admin/projects/create");
  await expect(page.getByRole("heading", { name: "创建项目" })).toBeVisible();

  await page.getByPlaceholder("如 coffee-main").fill(projectCode);
  await page.getByPlaceholder("如 Coffee Main").fill(projectName);
  await page
    .getByPlaceholder("可选描述")
    .fill("Playwright admin project create coverage");

  const initialAdminField = page.locator(".el-form-item", {
    hasText: "首个项目管理员",
  });
  await initialAdminField.locator(".el-select__wrapper").click();
  const initialAdminInput = initialAdminField.locator("input.el-select__input");
  await initialAdminInput.fill(currentUser.username);

  const initialAdminOption = getInitialAdminOption(page, currentUser.username);
  await expect(initialAdminOption).toBeVisible({ timeout: 10_000 });
  await initialAdminOption.click();

  await page.getByRole("button", { name: "创建" }).click();

  const successCard = page.locator(".admin-project-create-page__success-card");
  await expect(successCard).toContainText("项目创建成功");
  await expect(successCard).toContainText(projectName);
  await expect(successCard).toContainText(projectCode);
  await expect(successCard).toContainText(currentUser.username);
  await expect(
    page.getByRole("button", { name: "继续创建项目" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "前往平台项目" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "前往项目列表" }),
  ).toBeVisible();

  await page.getByRole("button", { name: "前往平台项目" }).click();
  await expect(page).toHaveURL(/\/admin\/projects$/, { timeout: 10_000 });
  await expect(page.getByRole("heading", { name: "项目管理" })).toBeVisible();
  await expect(page.locator(".el-table")).toContainText(projectCode);
  await expect(page.locator(".el-table")).toContainText(projectName);
});

test("platform admin can return to platform management from project list", async ({
  page,
}) => {
  await loginAsPlatformAdmin(page);

  await page.getByRole("link", { name: "返回项目列表" }).click();
  await expect(page).toHaveURL(/\/projects$/, { timeout: 10_000 });

  await page.getByRole("button", { name: "平台管理" }).click();
  await expect(page).toHaveURL(/\/admin\/users$/, { timeout: 10_000 });
  await expect(page.getByRole("button", { name: "新建用户" })).toBeVisible();
});

test("resource lifecycle: delete unused config file and empty projects", async ({
  page,
}) => {
  const suffix = Date.now().toString();
  await loginAsPlatformAdmin(page);

  const projectId = await createProject(page, `${suffix}-lifecycle`);
  const configCode = `unused-${suffix}`;
  const configResponse = await postWithCsrf(page, "/api/config-files", {
    data: {
      project_id: projectId,
      code: configCode,
      name: `Unused ${suffix}`,
      format: "yaml",
      sensitivity: "normal",
      is_required: false,
    },
  });
  expect(configResponse.ok()).toBeTruthy();

  await page.goto(`/projects/${projectId}/config-files`);
  const configRow = page.locator(".el-table__row", { hasText: configCode });
  await expect(configRow).toBeVisible();
  await configRow.getByRole("button", { name: "删除" }).click();

  const configDeleteDialog = page.getByRole("dialog", { name: "删除配置文件" });
  await expect(configDeleteDialog).toContainText(configCode);
  await configDeleteDialog.getByRole("button", { name: "删除" }).click();
  await expect(
    page.locator(".el-message", { hasText: "配置文件已删除" }),
  ).toBeVisible({ timeout: 5_000 });
  await expect(configRow).toHaveCount(0, { timeout: 5_000 });

  await page.goto(`/projects/${projectId}`);
  await page.getByRole("button", { name: "删除项目" }).click();
  const projectDeleteDialog = page.getByRole("dialog", { name: "删除项目" });
  await expect(projectDeleteDialog).toContainText("e2e-project");
  await projectDeleteDialog.getByRole("button", { name: "删除" }).click();
  await expect(page).toHaveURL(/\/projects$/, { timeout: 10_000 });

  const adminProjectId = await createProject(page, `${suffix}-admin-delete`);
  await page.goto("/admin/projects");
  await page
    .getByPlaceholder("搜索项目标识或名称")
    .fill(`${suffix}-admin-delete`);
  const adminProjectRow = page.locator(".el-table__row", {
    hasText: `${suffix}-admin-delete`,
  });
  await expect(adminProjectRow).toBeVisible({ timeout: 10_000 });
  await adminProjectRow.getByRole("button", { name: "删除" }).click();

  const adminDeleteDialog = page.getByRole("dialog", { name: "删除项目" });
  await expect(adminDeleteDialog).toContainText(`${suffix}-admin-delete`);
  await adminDeleteDialog.getByRole("button", { name: "删除" }).click();
  await expect(
    page.locator(".el-message", { hasText: "项目已删除" }),
  ).toBeVisible({ timeout: 5_000 });

  const countResponse = await page.request.get(
    `/api/projects/${adminProjectId}`,
  );
  expect(countResponse.status()).toBe(404);
});

test("project members: add, change role, and remove member", async ({
  page,
}) => {
  const suffix = Date.now().toString();
  const memberUsername = `e2e-member-${suffix}`;

  await loginAsPlatformAdmin(page);
  await createAdminUser(page, memberUsername);
  const projectId = await createProject(page, `${suffix}-members`);

  await page.goto(`/projects/${projectId}/members`);
  await expect(page.getByText("项目成员绑定已有启用用户")).toBeVisible();

  await page.getByRole("button", { name: "添加成员" }).click();
  const createDialog = page.getByRole("dialog", { name: "添加项目成员" });
  await expect(createDialog).toBeVisible();
  await createDialog.getByPlaceholder("输入已存在用户名").fill(memberUsername);
  await createDialog.getByRole("button", { name: "添加" }).click();
  await expect(
    page.locator(".el-message", { hasText: "项目成员已添加" }),
  ).toBeVisible({ timeout: 5_000 });

  const memberRow = page.locator(".el-table__row", {
    hasText: memberUsername,
  });
  await expect(memberRow).toBeVisible();
  await expect(memberRow).toContainText("只读成员");

  await memberRow.locator(".el-select__wrapper").click();
  await page.getByRole("option", { name: "编辑者" }).click();
  await expect(
    page.locator(".el-message", { hasText: "项目成员角色已更新" }),
  ).toBeVisible({ timeout: 5_000 });
  await expect(memberRow).toContainText("编辑者");

  await memberRow.getByRole("button", { name: "删除" }).click();
  const deleteDialog = page.getByRole("dialog", { name: "移除项目成员" });
  await expect(deleteDialog).toContainText(memberUsername);
  await deleteDialog.getByRole("button", { name: "删除" }).click();
  await expect(
    page.locator(".el-message", { hasText: "项目成员已移除" }),
  ).toBeVisible({ timeout: 5_000 });
  await expect(memberRow).toHaveCount(0, { timeout: 5_000 });
});

test("sync records: list and filter reported client events", async ({
  page,
}) => {
  const suffix = Date.now().toString();
  const projectCode = `e2e-project-${suffix}-sync`;
  const configCode = `sync-main-${suffix}`;

  await loginAsPlatformAdmin(page);
  const projectId = await createProject(page, `${suffix}-sync`);
  const environment = await createProjectEnvironment(page, projectId, suffix);
  const deployment = await createDeploymentInstance(
    page,
    projectId,
    environment.id,
    suffix,
  );
  const token = await activateDeploymentInstance(page, deployment.id);

  const configResponse = await postWithCsrf(page, "/api/config-files", {
    data: {
      project_id: projectId,
      code: configCode,
      name: `Sync Main ${suffix}`,
      format: "yaml",
      sensitivity: "normal",
      is_required: false,
    },
  });
  expect(configResponse.ok()).toBeTruthy();

  const syncResponse = await page.request.post(
    "/api/open/deployment-sync-records",
    {
      headers: {
        Authorization: `Bearer ${token}`,
      },
      data: {
        project: projectCode,
        environment: environment.code,
        deployment_key: deployment.deployment_key,
        config: configCode,
        action: "apply",
        status: "failed",
        message: "apply failed in playwright",
        detail: {
          duration_ms: 42,
        },
        reported_at: "2026-04-27T12:00:00Z",
      },
    },
  );
  expect(syncResponse.ok()).toBeTruthy();

  await page.goto(`/projects/${projectId}/sync-records`);
  await expect(page.getByRole("heading", { name: `E2E Project` })).toBeVisible({
    timeout: 10_000,
  });

  const recordRow = page.locator(".el-table__row", { hasText: configCode });
  await expect(recordRow).toBeVisible({ timeout: 10_000 });
  await expect(recordRow).toContainText("应用");
  await expect(recordRow).toContainText("失败");
  await expect(recordRow).toContainText("apply failed in playwright");

  await page
    .locator(".project-sync-record-list-page__filters .el-select__wrapper")
    .nth(3)
    .click();
  await page.getByRole("option", { name: "成功" }).click();
  await page.getByRole("button", { name: "查询" }).click();
  await expect(page.getByText("暂无同步记录")).toBeVisible();

  await page.getByRole("button", { name: "重置" }).click();
  await expect(recordRow).toBeVisible({ timeout: 10_000 });
});

test("heartbeats: list and filter latest client reports", async ({ page }) => {
  const suffix = `${Date.now()}-heartbeat`;
  const projectCode = `e2e-project-${suffix}`;
  const configCode = `heartbeat-main-${Date.now()}`;

  await loginAsPlatformAdmin(page);
  const projectId = await createProject(page, suffix);
  const environment = await createProjectEnvironment(page, projectId, suffix);
  const deployment = await createDeploymentInstance(
    page,
    projectId,
    environment.id,
    suffix,
  );
  const token = await activateDeploymentInstance(page, deployment.id);

  const configResponse = await postWithCsrf(page, "/api/config-files", {
    data: {
      project_id: projectId,
      code: configCode,
      name: `Heartbeat Main ${suffix}`,
      format: "yaml",
      sensitivity: "normal",
      is_required: false,
    },
  });
  expect(configResponse.ok()).toBeTruthy();

  const heartbeatResponse = await page.request.post("/api/open/heartbeats", {
    headers: {
      Authorization: `Bearer ${token}`,
    },
    data: {
      project: projectCode,
      environment: environment.code,
      deployment_key: deployment.deployment_key,
      config: configCode,
      metadata: {
        status: "ok",
        version: "1.2.3",
        source: "playwright",
      },
      reported_at: "2026-04-27T12:03:00Z",
    },
  });
  expect(heartbeatResponse.ok()).toBeTruthy();

  await page.goto(`/projects/${projectId}/heartbeats`);
  await expect(page.getByRole("heading", { name: `E2E Project` })).toBeVisible({
    timeout: 10_000,
  });
  await expect(
    page.getByText("这里只展示客户端最近一次心跳上报").first(),
  ).toBeVisible();

  const heartbeatRow = page.locator(".el-table__row", { hasText: configCode });
  await expect(heartbeatRow).toBeVisible({ timeout: 10_000 });
  await expect(heartbeatRow).toContainText("1.2.3");

  await page
    .locator(".project-heartbeat-list-page__filters .el-select__wrapper")
    .nth(1)
    .click();
  await page.getByRole("option", { name: new RegExp(configCode) }).click();
  await page.getByRole("button", { name: "查询" }).click();
  await expect(heartbeatRow).toBeVisible({ timeout: 10_000 });

  await page.getByRole("button", { name: "重置" }).click();
  await expect(heartbeatRow).toBeVisible({ timeout: 10_000 });
});

test("audit logs: project admins can list and filter audit events", async ({
  page,
}) => {
  const suffix = `${Date.now()}-audit`;
  const action = "project.created_by_platform_admin";

  await loginAsPlatformAdmin(page);
  const projectId = await createProject(page, suffix);

  await page.goto(`/projects/${projectId}/audit-logs`);
  await expect(page.getByRole("heading", { name: `E2E Project` })).toBeVisible({
    timeout: 10_000,
  });
  await expect(
    page.getByText("审计详情只展示安全元数据").first(),
  ).toBeVisible();

  const auditRow = page.locator(".el-table__row", { hasText: action });
  await expect(auditRow).toBeVisible({ timeout: 10_000 });
  await expect(auditRow).toContainText("project");
  await expect(auditRow).toContainText(String(projectId));

  await page.getByPlaceholder("按动作筛选").fill(action);
  await page.getByRole("button", { name: "查询" }).click();
  await expect(auditRow).toBeVisible({ timeout: 10_000 });

  await page.getByPlaceholder("按资源类型筛选").fill("config_file");
  await page.getByRole("button", { name: "查询" }).click();
  await expect(page.getByText("暂无审计日志")).toBeVisible();

  await page.getByRole("button", { name: "重置" }).click();
  await expect(auditRow).toBeVisible({ timeout: 10_000 });
});

test("admin project create path: other initial admin hides project-list action", async ({
  page,
}) => {
  const suffix = Date.now().toString();
  await loginAsPlatformAdmin(page);

  const initialAdminUsername = `e2e-project-admin-${suffix}`;
  await createAdminUser(page, initialAdminUsername);

  const projectCode = `e2e-admin-other-${suffix}`;
  const projectName = `E2E Admin Other ${suffix}`;

  await page.goto("/admin/projects/create");
  await expect(page.getByRole("heading", { name: "创建项目" })).toBeVisible();

  await page.getByPlaceholder("如 coffee-main").fill(projectCode);
  await page.getByPlaceholder("如 Coffee Main").fill(projectName);
  await page
    .getByPlaceholder("可选描述")
    .fill(
      "Playwright admin project create coverage for non-self initial admin",
    );

  const initialAdminField = page.locator(".el-form-item", {
    hasText: "首个项目管理员",
  });
  await initialAdminField.locator(".el-select__wrapper").click();
  const initialAdminInput = initialAdminField.locator("input.el-select__input");
  await initialAdminInput.fill(initialAdminUsername);

  const createdInitialAdminOption = getInitialAdminOption(
    page,
    initialAdminUsername,
  );
  await expect(createdInitialAdminOption).toBeVisible({ timeout: 10_000 });
  await createdInitialAdminOption.click();

  await page.getByRole("button", { name: "创建" }).click();

  const successCard = page.locator(".admin-project-create-page__success-card");
  await expect(successCard).toContainText("项目创建成功");
  await expect(successCard).toContainText(projectName);
  await expect(successCard).toContainText(projectCode);
  await expect(successCard).toContainText(initialAdminUsername);
  await expect(
    page.getByRole("button", { name: "继续创建项目" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "前往平台项目" }),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: "前往项目列表" })).toHaveCount(
    0,
  );
});

// ---------------------------------------------------------------------------
// Saved Versions: full CRUD flow via the Draft Editor right panel
// ---------------------------------------------------------------------------

async function setupDraftEditorContext(
  page: Page,
  suffix: string,
): Promise<{
  projectId: number;
  deploymentId: number;
  configFileId: number;
}> {
  const projectId = await createProject(page, suffix);

  const envResponse = await postWithCsrf(
    page,
    `/api/projects/${projectId}/environments`,
    {
      data: {
        code: `e2e-env-${suffix}`,
        name: `E2E Env ${suffix}`,
      },
    },
  );
  expect(envResponse.ok()).toBeTruthy();
  const env = (await envResponse.json()) as { id: number };

  const cfResponse = await postWithCsrf(page, "/api/config-files", {
    data: {
      project_id: projectId,
      code: `e2e-cfg-${suffix}`,
      name: `E2E Config ${suffix}`,
      format: "yaml",
    },
  });
  expect(cfResponse.ok()).toBeTruthy();
  const configFile = (await cfResponse.json()) as { id: number };

  const diResponse = await postWithCsrf(page, "/api/deployment-instances", {
    data: {
      project_id: projectId,
      environment_id: env.id,
      deployment_key: `e2e-di-${suffix}`,
      name: `E2E Deployment ${suffix}`,
    },
  });
  expect(diResponse.ok()).toBeTruthy();
  const deployment = (await diResponse.json()) as { id: number };

  return {
    projectId,
    deploymentId: deployment.id,
    configFileId: configFile.id,
  };
}

test("draft overlay: unsaved close confirmation stays above editor", async ({
  page,
}) => {
  const suffix = Date.now().toString();
  await login(page);
  const { projectId, deploymentId, configFileId } =
    await setupDraftEditorContext(page, suffix);

  await page.goto(`/projects/${projectId}/deployments/${deploymentId}`);
  await page.getByRole("button", { name: "打开工作台" }).click();

  await expect(page.locator(".draft-editor-overlay")).toBeVisible({
    timeout: 10_000,
  });
  await expect(page).toHaveURL(new RegExp(`draftConfigFileId=${configFileId}`));

  await fillDraftEditor(page, "key: overlay-confirm");
  await page.getByRole("button", { name: "关闭" }).click();

  const confirmDialog = page.locator(".el-message-box").first();
  await expect(confirmDialog).toBeVisible({ timeout: 5_000 });
  await expect(confirmDialog).toContainText("当前内容未保存");

  const leaveButton = confirmDialog.getByRole("button", { name: "离开" });
  await expect(leaveButton).toBeVisible();
  await expectElementCenterInside(leaveButton, ".el-message-box");

  await confirmDialog.getByRole("button", { name: "取消" }).click();
  await expect(page.locator(".draft-editor-overlay")).toBeVisible();
  await expect(page).toHaveURL(new RegExp(`draftConfigFileId=${configFileId}`));
});

test("draft overlay: deployment detail refreshes workspace hints after saved draft closes", async ({
  page,
}) => {
  const suffix = Date.now().toString();
  const content = `key: detail-refresh-${suffix}`;
  await login(page);
  const { projectId, deploymentId, configFileId } =
    await setupDraftEditorContext(page, suffix);

  await page.goto(`/projects/${projectId}/deployments/${deploymentId}`);

  const detailRow = page
    .locator(".el-table__body tr", { hasText: `e2e-cfg-${suffix}` })
    .first();
  await expect(detailRow).toContainText("Not Configured", { timeout: 10_000 });
  await expect(detailRow).toContainText("Missing Optional");
  await expect(detailRow).toContainText("Saved Versions 0");
  await expect(detailRow).toContainText("无 Release");

  await page.getByRole("button", { name: "打开工作台" }).click();
  await expect(page.locator(".draft-editor-overlay")).toBeVisible({
    timeout: 10_000,
  });
  await expect(page).toHaveURL(new RegExp(`draftConfigFileId=${configFileId}`));

  await fillDraftEditor(page, content);
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.locator(".el-message", { hasText: "已保存" })).toBeVisible({
    timeout: 5_000,
  });
  await page.getByRole("button", { name: "关闭" }).click();

  await expect(page.locator(".draft-editor-overlay")).toHaveCount(0, {
    timeout: 10_000,
  });
  await expect(page).not.toHaveURL(/draftConfigFileId=/);
  await expect(detailRow).toContainText("Current Draft", { timeout: 10_000 });
  await expect(detailRow).toContainText("可预览");
  await expect(detailRow).toContainText("Saved Versions 1");
});

test("draft overlay: preview page refreshes after saved draft closes", async ({
  page,
}) => {
  const suffix = Date.now().toString();
  const content = `key: overlay-refresh-${suffix}`;
  await login(page);
  const { projectId, deploymentId, configFileId } =
    await setupDraftEditorContext(page, suffix);

  await page.goto(`/projects/${projectId}/deployments/${deploymentId}/preview`);

  const previewRow = page
    .locator(".el-table__body tr", { hasText: `e2e-cfg-${suffix}` })
    .first();
  await expect(previewRow).toContainText("Not Configured", { timeout: 10_000 });
  await expect(previewRow).toContainText("Missing Optional");

  await previewRow.getByRole("button", { name: "编辑 Current Draft" }).click();
  await expect(page.locator(".draft-editor-overlay")).toBeVisible({
    timeout: 10_000,
  });
  await expect(page).toHaveURL(new RegExp(`draftConfigFileId=${configFileId}`));

  await fillDraftEditor(page, content);
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.locator(".el-message", { hasText: "已保存" })).toBeVisible({
    timeout: 5_000,
  });
  await page.getByRole("button", { name: "关闭" }).click();

  await expect(page.locator(".draft-editor-overlay")).toHaveCount(0, {
    timeout: 10_000,
  });
  await expect(page).not.toHaveURL(/draftConfigFileId=/);
  await expect(previewRow).toContainText("可预览", { timeout: 10_000 });
  await expect(
    page.locator(".deployment-preview-page__json textarea"),
  ).toHaveValue(new RegExp(content));
});

test("deployment list: expanded instance opens workspace without changing the list route", async ({
  page,
}) => {
  const suffix = Date.now().toString();
  await login(page);
  const { projectId, deploymentId } = await setupDraftEditorContext(
    page,
    suffix,
  );

  const extraConfigResponse = await postWithCsrf(page, "/api/config-files", {
    data: {
      project_id: projectId,
      code: `e2e-cfg-extra-${suffix}`,
      name: `E2E Extra Config ${suffix}`,
      format: "json",
    },
  });
  expect(extraConfigResponse.ok()).toBeTruthy();

  await page.goto(`/projects/${projectId}/deployments`);
  const listUrl = page.url();

  const instanceSection = page
    .locator(".deployment-instance-list-page__section")
    .nth(1);
  const instanceRow = instanceSection.locator(".el-table__row", {
    hasText: `e2e-di-${suffix}`,
  });
  await expect(instanceRow).toBeVisible({ timeout: 10_000 });
  await instanceRow.locator(".el-table__expand-icon").click();

  const expanded = instanceSection.locator(
    ".deployment-instance-list-page__expanded",
    { hasText: `e2e-cfg-${suffix}` },
  );
  await expect(expanded).toBeVisible({ timeout: 10_000 });
  await expect(expanded).toContainText(`e2e-cfg-extra-${suffix}`);
  await expect(expanded).toContainText("Not Configured");
  await expect(expanded).toContainText("Missing Optional");

  await expanded.getByRole("button", { name: "打开工作台" }).first().click();
  const overlay = page.locator(".draft-editor-overlay");
  await expect(overlay).toBeVisible({ timeout: 10_000 });
  await expect(page).toHaveURL(listUrl);

  await overlay.getByText(`e2e-cfg-extra-${suffix}`).click();
  await expect(page).toHaveURL(listUrl);

  await page.getByRole("button", { name: "关闭" }).click();
  await expect(overlay).toHaveCount(0, { timeout: 10_000 });
  await expect(page).toHaveURL(listUrl);
  await expect(expanded).toBeVisible();

  expect(deploymentId).toBeGreaterThan(0);
});

test("deployment preview: view releases and restore latest release to current draft", async ({
  page,
}) => {
  const suffix = Date.now().toString();
  const releaseContent = `key: release-source-${suffix}`;
  const draftContent = `key: draft-before-restore-${suffix}`;
  await login(page);
  const { projectId, deploymentId, configFileId } =
    await setupDraftEditorContext(page, suffix);

  const firstDraftResponse = await putWithCsrf(
    page,
    `/api/drafts/${deploymentId}/${configFileId}`,
    {
      data: {
        content: releaseContent,
        format: "yaml",
        base_version: 0,
      },
    },
  );
  expect(firstDraftResponse.ok()).toBeTruthy();
  const firstDraft = (await firstDraftResponse.json()) as { version: number };

  const publishResponse = await postWithCsrf(page, "/api/releases/publish", {
    data: {
      project_id: projectId,
      deployment_instance_id: deploymentId,
      config_file_id: configFileId,
      change_summary: "Preview restore source",
    },
  });
  expect(publishResponse.ok()).toBeTruthy();
  const release = (await publishResponse.json()) as {
    id: number;
    revision: string;
  };

  const secondDraftResponse = await putWithCsrf(
    page,
    `/api/drafts/${deploymentId}/${configFileId}`,
    {
      data: {
        content: draftContent,
        format: "yaml",
        base_version: firstDraft.version,
      },
    },
  );
  expect(secondDraftResponse.ok()).toBeTruthy();

  await page.goto(`/projects/${projectId}/deployments/${deploymentId}/preview`);

  const previewRow = page
    .locator(".el-table__body tr", { hasText: `e2e-cfg-${suffix}` })
    .first();
  await expect(previewRow).toContainText("Current Draft", { timeout: 10_000 });
  await expect(
    previewRow.getByRole("button", {
      name: "恢复 latest release 到 Current Draft",
    }),
  ).toBeVisible();

  await previewRow.getByRole("button", { name: "查看 Releases" }).click();
  await expect(page).toHaveURL(
    new RegExp(
      `/projects/${projectId}/releases\\?deployment_instance_id=${deploymentId}&config_file_id=${configFileId}`,
    ),
    { timeout: 10_000 },
  );
  await expect(page.locator(".release-list-page__table")).toContainText(
    release.revision,
  );
  await expect(page.locator(".release-list-page__table")).toContainText(
    "Preview restore source",
  );

  await page.goto(`/projects/${projectId}/deployments/${deploymentId}/preview`);
  await previewRow
    .getByRole("button", { name: "恢复 latest release 到 Current Draft" })
    .click();

  const restoreDialog = page.getByRole("dialog", { name: "恢复最新 Release" });
  await expect(restoreDialog).toBeVisible();
  await restoreDialog.getByRole("button", { name: "确认恢复" }).click();

  await expect(
    page.locator(".el-message", {
      hasText: "已恢复最新 Release 到 Current Draft",
    }),
  ).toBeVisible({ timeout: 5_000 });
  await expect(
    page.locator(".deployment-preview-page__json textarea"),
  ).toHaveValue(new RegExp(releaseContent), { timeout: 10_000 });
  await expect(
    page.locator(".deployment-preview-page__json textarea"),
  ).not.toHaveValue(new RegExp(draftContent));
});

test("saved versions: save → list → note → restore → delete", async ({
  page,
}) => {
  const suffix = Date.now().toString();
  await login(page);
  const { projectId, deploymentId, configFileId } =
    await setupDraftEditorContext(page, suffix);

  // Navigate to draft editor
  await page.goto(
    `/projects/${projectId}/deployments/${deploymentId}/configs/${configFileId}/draft`,
  );

  const editor = getDraftEditor(page);
  await expect(editor).toBeVisible({ timeout: 10_000 });

  // Saved versions panel should be visible for admin, initially empty
  const panel = page.locator(".draft-editor-page__history");
  await expect(panel).toBeVisible();
  await expect(panel).toContainText("暂无 Saved Version");

  // ---- Save draft #1 -------------------------------------------------------
  await fillDraftEditor(page, "key: value_1");
  await page.getByRole("button", { name: "保存", exact: true }).click();

  // Wait for the success toast to confirm the save completed
  await expect(page.locator(".el-message", { hasText: "已保存" })).toBeVisible({
    timeout: 5_000,
  });

  // Panel should now contain 1 saved version item
  const items = panel.locator(".draft-editor-page__history-item");
  await expect(items).toHaveCount(1, { timeout: 5_000 });

  // First item should be auto-selected; detail section should be visible
  const detail = panel.locator(".draft-editor-page__history-detail");
  await expect(detail.locator(".el-descriptions")).toBeVisible({
    timeout: 5_000,
  });

  // ---- Update note ----------------------------------------------------------
  // Wait for the previous toast to disappear to avoid strict-mode violations
  await expect(page.locator(".el-message")).toHaveCount(0, { timeout: 6_000 });

  const noteTextarea = panel.locator(
    ".draft-editor-page__history-note textarea",
  );
  await noteTextarea.fill("E2E test note");
  await panel.getByRole("button", { name: "保存备注" }).click();
  await expect(
    page.locator(".el-message", { hasText: "备注已更新" }),
  ).toBeVisible({ timeout: 5_000 });

  // ---- Save draft #2 -------------------------------------------------------
  await expect(page.locator(".el-message")).toHaveCount(0, { timeout: 6_000 });

  await fillDraftEditor(page, "key: value_2");
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.locator(".el-message", { hasText: "已保存" })).toBeVisible({
    timeout: 5_000,
  });
  await expect(items).toHaveCount(2, { timeout: 5_000 });

  // ---- Restore older saved version ------------------------------------------
  // Wait for toasts to clear
  await expect(page.locator(".el-message")).toHaveCount(0, { timeout: 6_000 });

  // List is ordered newest-first, so the second item is the older one (value_1)
  await items.nth(1).click();
  await expect(detail.locator(".el-descriptions")).toBeVisible({
    timeout: 5_000,
  });

  await panel.getByRole("button", { name: "恢复到 Current Draft" }).click();

  // Restore confirmation dialog
  const restoreDialog = page.locator(".el-message-box");
  await expect(restoreDialog).toBeVisible();
  await restoreDialog.getByRole("button", { name: "确认恢复" }).click();

  // Editor content should revert to value_1
  await expect(editor).toHaveText("key: value_1", { timeout: 5_000 });

  // Restore does NOT create a new saved version, count stays 2
  await expect(items).toHaveCount(2, { timeout: 5_000 });

  // ---- Delete a saved version -----------------------------------------------
  // Wait for toasts to clear
  await expect(page.locator(".el-message")).toHaveCount(0, { timeout: 6_000 });

  // Select the first item (newest) and delete it
  await items.first().click();
  await expect(detail.locator(".el-descriptions")).toBeVisible({
    timeout: 5_000,
  });

  await panel.getByRole("button", { name: "删除" }).click();

  // Delete confirmation dialog
  const deleteDialog = page.locator(".el-message-box");
  await expect(deleteDialog).toBeVisible();
  await deleteDialog.getByRole("button", { name: "确认删除" }).click();

  await expect(page.locator(".el-message", { hasText: "已删除" })).toBeVisible({
    timeout: 5_000,
  });
  await expect(items).toHaveCount(1, { timeout: 5_000 });
});

// ---------------------------------------------------------------------------
// Clone from other instance: open dialog → search → select → clone draft
// ---------------------------------------------------------------------------

test("clone from other instance: search source → select → clone draft", async ({
  page,
}) => {
  const suffix = Date.now().toString();
  await login(page);

  // --- seed: project + env + config file + 2 deployments ---
  const projectId = await createProject(page, suffix);

  const envResponse = await postWithCsrf(
    page,
    `/api/projects/${projectId}/environments`,
    { data: { code: `e2e-env-${suffix}`, name: `E2E Env ${suffix}` } },
  );
  expect(envResponse.ok()).toBeTruthy();
  const env = (await envResponse.json()) as { id: number };

  const cfResponse = await postWithCsrf(page, "/api/config-files", {
    data: {
      project_id: projectId,
      code: `e2e-cfg-${suffix}`,
      name: `E2E Config ${suffix}`,
      format: "yaml",
    },
  });
  expect(cfResponse.ok()).toBeTruthy();
  const configFile = (await cfResponse.json()) as { id: number };

  // source deployment (will have a draft)
  const srcResponse = await postWithCsrf(page, "/api/deployment-instances", {
    data: {
      project_id: projectId,
      environment_id: env.id,
      deployment_key: `e2e-src-${suffix}`,
      name: `Source ${suffix}`,
    },
  });
  expect(srcResponse.ok()).toBeTruthy();
  const srcDeployment = (await srcResponse.json()) as { id: number };

  // target deployment (clone destination)
  const tgtResponse = await postWithCsrf(page, "/api/deployment-instances", {
    data: {
      project_id: projectId,
      environment_id: env.id,
      deployment_key: `e2e-tgt-${suffix}`,
      name: `Target ${suffix}`,
    },
  });
  expect(tgtResponse.ok()).toBeTruthy();
  const tgtDeployment = (await tgtResponse.json()) as { id: number };

  // seed a draft on the source instance
  const draftContent = `greeting: hello-from-source-${suffix}`;
  const putDraftResponse = await putWithCsrf(
    page,
    `/api/drafts/${srcDeployment.id}/${configFile.id}`,
    { data: { content: draftContent, format: "yaml", base_version: 0 } },
  );
  expect(putDraftResponse.ok()).toBeTruthy();

  // --- navigate to target's draft editor ---
  await page.goto(
    `/projects/${projectId}/deployments/${tgtDeployment.id}/configs/${configFile.id}/draft`,
  );
  const editor = getDraftEditor(page);
  await expect(editor).toBeVisible({ timeout: 10_000 });

  // --- open clone dialog ---
  await page.getByRole("button", { name: "从其他实例复制" }).click();
  const dialog = page.getByRole("dialog", { name: "从其他实例复制配置" });
  await expect(dialog).toBeVisible();

  // --- click the select to open the dropdown ---
  await dialog.locator(".el-select").click();

  // The source instance should appear with "Draft ✓" badge
  const sourceOption = page.locator(".el-select-dropdown__item", {
    hasText: `e2e-src-${suffix}`,
  });
  await expect(sourceOption).toBeVisible({ timeout: 5_000 });
  await expect(sourceOption).toContainText("Draft ✓");

  // Select the source instance
  await sourceOption.click();

  // Draft radio should be enabled (selected by default)
  const draftRadio = dialog.locator(".el-radio", { hasText: "Draft" });
  await expect(draftRadio).toBeVisible();

  // --- submit clone ---
  await dialog.getByRole("button", { name: "复制到当前 Draft" }).click();

  // Success toast
  await expect(
    page.locator(".el-message", { hasText: "已从其他实例复制" }),
  ).toBeVisible({ timeout: 5_000 });

  // Editor should now contain the source draft content
  await expect(editor).toHaveText(draftContent, { timeout: 5_000 });
});

// ---------------------------------------------------------------------------
// Clone dialog: keyword search + load-more carries keyword (full UI flow)
// ---------------------------------------------------------------------------

test("clone dialog: remote search filters by keyword and pagination carries keyword", async ({
  page,
}) => {
  const suffix = Date.now().toString();
  await login(page);

  const projectId = await createProject(page, suffix);

  const envResponse = await postWithCsrf(
    page,
    `/api/projects/${projectId}/environments`,
    { data: { code: `e2e-env-${suffix}`, name: `E2E Env ${suffix}` } },
  );
  expect(envResponse.ok()).toBeTruthy();
  const env = (await envResponse.json()) as { id: number };

  const cfResponse = await postWithCsrf(page, "/api/config-files", {
    data: {
      project_id: projectId,
      code: `e2e-cfg-${suffix}`,
      name: `E2E Config ${suffix}`,
      format: "yaml",
    },
  });
  expect(cfResponse.ok()).toBeTruthy();
  const configFile = (await cfResponse.json()) as { id: number };

  // Create 4 deployments: alpha-a, alpha-b, alpha-c (sources), target-one,
  // plus beta-one (non-alpha source) — 5 total
  const names = [
    "alpha-a",
    "alpha-b",
    "alpha-c",
    "target-one",
    "beta-one",
  ] as const;
  const deploymentIds: number[] = [];
  for (const name of names) {
    const r = await postWithCsrf(page, "/api/deployment-instances", {
      data: {
        project_id: projectId,
        environment_id: env.id,
        deployment_key: `${name}-${suffix}`,
        name: `${name} ${suffix}`,
      },
    });
    expect(r.ok()).toBeTruthy();
    const d = (await r.json()) as { id: number };
    deploymentIds.push(d.id);
  }

  // Seed drafts on alpha-a, alpha-b, alpha-c, and beta-one
  for (const id of [
    deploymentIds[0],
    deploymentIds[1],
    deploymentIds[2],
    deploymentIds[4],
  ]) {
    const r = await putWithCsrf(page, `/api/drafts/${id}/${configFile.id}`, {
      data: { content: `key: val-${id}`, format: "yaml", base_version: 0 },
    });
    expect(r.ok()).toBeTruthy();
  }

  // Intercept clone-sources API responses to simulate limit=2 pagination.
  // We truncate responses with >2 items and set next_cursor so the frontend
  // sees a paginated result and shows the "load more" button.
  // The regex requires a `?` after the path to avoid matching Vite module URLs.
  await page.route(/\/api\/clone-sources\?/, async (route) => {
    const response = await route.fetch();
    const json = (await response.json()) as {
      items: Array<{ deployment_instance_id: number }>;
      next_cursor: number | null;
    };
    if (json.items.length > 2) {
      await route.fulfill({
        status: response.status(),
        headers: response.headers(),
        body: JSON.stringify({
          items: json.items.slice(0, 2),
          next_cursor: json.items[1].deployment_instance_id,
        }),
      });
    } else {
      await route.fulfill({
        status: response.status(),
        headers: response.headers(),
        body: JSON.stringify(json),
      });
    }
  });

  // Target: target-one
  const targetId = deploymentIds[3];
  await page.goto(
    `/projects/${projectId}/deployments/${targetId}/configs/${configFile.id}/draft`,
  );
  const editor = getDraftEditor(page);
  await expect(editor).toBeVisible({ timeout: 10_000 });

  // Open clone dialog
  await page.getByRole("button", { name: "从其他实例复制" }).click();
  const dialog = page.getByRole("dialog", { name: "从其他实例复制配置" });
  await expect(dialog).toBeVisible();

  // Open dropdown
  await dialog.locator(".el-select").click();
  const dropdownItems = page.getByRole("listbox").last().getByRole("option");

  // Initial load (limit=2): shows first 2 of 4 sources
  await expect(dropdownItems).toHaveCount(2, { timeout: 5_000 });

  // Type "alpha" → remote search with keyword (debounce 300ms)
  const selectInput = dialog.getByRole("combobox", { name: "来源实例" });
  // Wait for the keyword search response to settle before proceeding.
  // fill() triggers handleCloneRemoteSearch which debounces 300ms.
  const keywordSearchDone = page.waitForResponse(
    (resp) =>
      resp.url().includes("/api/clone-sources?") &&
      resp.url().includes("keyword=alpha") &&
      resp.status() === 200,
  );
  await selectInput.fill("alpha");
  await keywordSearchDone;

  // After debounce + API (intercepted to 2), shows 2 of 3 alpha sources + load-more
  await expect(dropdownItems).toHaveCount(2, { timeout: 5_000 });
  // All visible items must be alpha-*
  for (const item of await dropdownItems.all()) {
    await expect(item).toContainText("alpha-");
  }

  // "加载更多…" button should be visible
  const loadMoreBtn = page.getByRole("button", { name: "加载更多" });
  await expect(loadMoreBtn).toBeVisible();

  // Click load-more — should fetch page 2 with keyword=alpha
  await Promise.all([
    page.waitForResponse(
      (resp) =>
        resp.url().includes("/api/clone-sources?") && resp.status() === 200,
    ),
    loadMoreBtn.click(),
  ]);

  // Now 3 alpha items total, no beta items
  await expect(dropdownItems).toHaveCount(3, { timeout: 5_000 });
  for (const item of await dropdownItems.all()) {
    await expect(item).toContainText("alpha-");
  }

  // Load-more should be gone (no more pages)
  await expect(loadMoreBtn).not.toBeVisible();

  // Close dialog
  await dialog.getByRole("button", { name: "取消" }).click();
});

// ---------------------------------------------------------------------------
// Release detail & diff: publish → detail page → diff page
// ---------------------------------------------------------------------------

test("release detail and diff: publish draft → view detail → view diff", async ({
  page,
}) => {
  const suffix = Date.now().toString();
  await login(page);
  const { projectId, deploymentId, configFileId } =
    await setupDraftEditorContext(page, suffix);

  // Navigate to draft editor and create a draft
  await page.goto(
    `/projects/${projectId}/deployments/${deploymentId}/configs/${configFileId}/draft`,
  );
  const editor = getDraftEditor(page);
  await expect(editor).toBeVisible({ timeout: 10_000 });

  // Write and save a draft
  await fillDraftEditor(page, "greeting: hello-release-test");
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.locator(".el-message", { hasText: "已保存" })).toBeVisible({
    timeout: 5_000,
  });
  await expect(page.locator(".el-message")).toHaveCount(0, { timeout: 6_000 });

  // Publish the draft
  await page.getByRole("button", { name: "发布 Release" }).click();
  const publishDialog = page.locator(".el-message-box");
  await expect(publishDialog).toBeVisible();
  await publishDialog.locator("textarea").fill("E2E release test");
  await publishDialog.getByRole("button", { name: "发布" }).click();

  // After publish, should redirect to release detail page
  await expect(page).toHaveURL(
    new RegExp(`/projects/${projectId}/releases/\\d+`),
    { timeout: 10_000 },
  );

  // Release detail: verify meta info is visible
  await expect(page.locator(".el-descriptions")).toBeVisible({
    timeout: 10_000,
  });

  // Verify revision is displayed
  const metaSection = page.locator(".release-detail-page__meta");
  await expect(metaSection).toBeVisible();

  // Verify published time is shown
  await expect(metaSection).toContainText("发布时间");

  // Verify readonly content is displayed
  const content = page.locator(".release-detail-page__content");
  await expect(content).toBeVisible();
  await expect(content).toContainText("greeting: hello-release-test");

  // Verify readonly hint
  await expect(page.locator(".release-detail-page__alert")).toBeVisible();

  // Click "查看 Diff" to navigate to diff page
  await page.getByRole("button", { name: "查看 Diff" }).click();
  await expect(page).toHaveURL(
    new RegExp(`/projects/${projectId}/releases/\\d+/diff`),
    { timeout: 10_000 },
  );

  // Diff page: verify meta is visible
  await expect(page.locator(".release-diff-page__meta")).toBeVisible({
    timeout: 10_000,
  });

  // First release should show "首个发布版本" hint
  await expect(page.locator(".release-diff-page__alert")).toContainText(
    "首个发布版本",
  );

  // Current version content should be visible
  const diffContent = page.locator(".release-diff-page__content");
  await expect(diffContent.first()).toBeVisible();
  await expect(diffContent.first()).toContainText(
    "greeting: hello-release-test",
  );

  // Navigate back to detail
  await page.getByRole("button", { name: "返回发布详情" }).click();
  await expect(page).toHaveURL(
    new RegExp(`/projects/${projectId}/releases/\\d+$`),
    { timeout: 10_000 },
  );

  // Navigate back to release list
  await page.getByRole("button", { name: "返回发布列表" }).click();
  await expect(page).toHaveURL(new RegExp(`/projects/${projectId}/releases$`), {
    timeout: 10_000,
  });

  // Release list: verify "查看" and "Diff" action buttons are present
  const releaseRow = page.locator(".el-table__row").first();
  await expect(releaseRow).toBeVisible({ timeout: 10_000 });
  await expect(releaseRow.getByRole("button", { name: "查看" })).toBeVisible();
  await expect(releaseRow.getByRole("button", { name: "Diff" })).toBeVisible();

  // Click "查看" to go to detail again
  await releaseRow.getByRole("button", { name: "查看" }).click();
  await expect(page).toHaveURL(
    new RegExp(`/projects/${projectId}/releases/\\d+`),
    { timeout: 10_000 },
  );
  await expect(page.locator(".release-detail-page__content")).toBeVisible({
    timeout: 10_000,
  });

  // Publish a second release and verify line-level diff markers.
  await page.goto(
    `/projects/${projectId}/deployments/${deploymentId}/configs/${configFileId}/draft`,
  );
  await expect(getDraftEditor(page)).toBeVisible({ timeout: 10_000 });
  await fillDraftEditor(
    page,
    "greeting: hello-release-test-v2\nfeature: enabled",
  );
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.locator(".el-message", { hasText: "已保存" })).toBeVisible({
    timeout: 5_000,
  });
  await expect(page.locator(".el-message")).toHaveCount(0, { timeout: 6_000 });

  await page.getByRole("button", { name: "发布 Release" }).click();
  await expect(publishDialog).toBeVisible();
  await publishDialog.locator("textarea").fill("E2E release diff test");
  await publishDialog.getByRole("button", { name: "发布" }).click();
  await expect(page).toHaveURL(
    new RegExp(`/projects/${projectId}/releases/\\d+`),
    { timeout: 10_000 },
  );

  await page.getByRole("button", { name: "查看 Diff" }).click();
  await expect(page).toHaveURL(
    new RegExp(`/projects/${projectId}/releases/\\d+/diff`),
    { timeout: 10_000 },
  );
  const lineDiff = page.locator(".config-line-diff-viewer");
  await expect(lineDiff).toBeVisible({ timeout: 10_000 });
  await expect(
    lineDiff.locator(".is-removed").filter({
      hasText: "greeting: hello-release-test",
    }),
  ).toBeVisible();
  await expect(
    lineDiff.locator(".is-added").filter({
      hasText: "greeting: hello-release-test-v2",
    }),
  ).toBeVisible();
  await expect(
    lineDiff.locator(".config-line-diff-viewer__segment.is-changed", {
      hasText: "-v2",
    }),
  ).toBeVisible();
  await expect(
    lineDiff.locator(".is-added").filter({ hasText: "feature: enabled" }),
  ).toBeVisible();
});

test("release detail: restore historical release to current draft", async ({
  page,
}) => {
  const suffix = Date.now().toString();
  const releaseContent = `key: historical-release-${suffix}`;
  const currentDraftContent = `key: current-draft-${suffix}`;
  await login(page);
  const { projectId, deploymentId, configFileId } =
    await setupDraftEditorContext(page, suffix);

  const firstDraftResponse = await putWithCsrf(
    page,
    `/api/drafts/${deploymentId}/${configFileId}`,
    {
      data: {
        content: releaseContent,
        format: "yaml",
        base_version: 0,
      },
    },
  );
  expect(firstDraftResponse.ok()).toBeTruthy();
  const firstDraft = (await firstDraftResponse.json()) as { version: number };

  const firstReleaseResponse = await postWithCsrf(
    page,
    "/api/releases/publish",
    {
      data: {
        project_id: projectId,
        deployment_instance_id: deploymentId,
        config_file_id: configFileId,
        change_summary: "Historical restore source",
      },
    },
  );
  expect(firstReleaseResponse.ok()).toBeTruthy();
  const firstRelease = (await firstReleaseResponse.json()) as {
    id: number;
    revision: string;
  };

  const secondDraftResponse = await putWithCsrf(
    page,
    `/api/drafts/${deploymentId}/${configFileId}`,
    {
      data: {
        content: currentDraftContent,
        format: "yaml",
        base_version: firstDraft.version,
      },
    },
  );
  expect(secondDraftResponse.ok()).toBeTruthy();

  const secondReleaseResponse = await postWithCsrf(
    page,
    "/api/releases/publish",
    {
      data: {
        project_id: projectId,
        deployment_instance_id: deploymentId,
        config_file_id: configFileId,
        change_summary: "Newer release",
      },
    },
  );
  expect(secondReleaseResponse.ok()).toBeTruthy();

  await page.goto(`/projects/${projectId}/releases/${firstRelease.id}`);

  const restoreButton = page.getByRole("button", {
    name: "恢复此发布版本到 Current Draft",
  });
  await expect(restoreButton).toBeVisible({ timeout: 10_000 });
  await restoreButton.click();

  const restoreDialog = page.getByRole("dialog", { name: "恢复发布版本" });
  await expect(restoreDialog).toContainText(firstRelease.revision);
  await restoreDialog.getByRole("button", { name: "确认恢复" }).click();

  await expect(
    page.locator(".el-message", {
      hasText: `Release ${firstRelease.revision} 已恢复到 Current Draft`,
    }),
  ).toBeVisible({ timeout: 5_000 });

  await page.goto(
    `/projects/${projectId}/deployments/${deploymentId}/configs/${configFileId}/draft`,
  );
  const editor = getDraftEditor(page);
  await expect(editor).toBeVisible({ timeout: 10_000 });
  await expect(editor).toHaveText(releaseContent, { timeout: 10_000 });
});

// ---------------------------------------------------------------------------
// Deployment list split: template section vs instance section
// ---------------------------------------------------------------------------

test("deployment list page: templates and instances in separate sections", async ({
  page,
}) => {
  const suffix = Date.now().toString();
  await login(page);

  const projectId = await createProject(page, suffix);

  // Create environment via API
  const envResponse = await postWithCsrf(
    page,
    `/api/projects/${projectId}/environments`,
    { data: { code: `e2e-env-${suffix}`, name: `E2E Env ${suffix}` } },
  );
  expect(envResponse.ok()).toBeTruthy();
  const env = (await envResponse.json()) as { id: number };

  // Create a template via API
  const tmplResponse = await postWithCsrf(page, "/api/deployment-instances", {
    data: {
      project_id: projectId,
      environment_id: env.id,
      deployment_key: `tmpl-${suffix}`,
      name: `Template ${suffix}`,
      is_template: true,
    },
  });
  expect(tmplResponse.ok()).toBeTruthy();

  // Create a normal instance via API
  const instResponse = await postWithCsrf(page, "/api/deployment-instances", {
    data: {
      project_id: projectId,
      environment_id: env.id,
      deployment_key: `inst-${suffix}`,
      name: `Instance ${suffix}`,
      is_template: false,
    },
  });
  expect(instResponse.ok()).toBeTruthy();

  // Navigate to deployments page
  await page.goto(`/projects/${projectId}/deployments`);

  // Wait for both sections to be visible
  const sections = page.locator(
    ".deployment-instance-list-page__section-title",
  );
  await expect(sections).toHaveCount(2, { timeout: 10_000 });

  // Template section: should show the template, not the instance
  const templateSection = page
    .locator(".deployment-instance-list-page__section")
    .first();
  await expect(templateSection).toContainText(`tmpl-${suffix}`);
  await expect(templateSection).not.toContainText(`inst-${suffix}`);

  // Template row should have "创建实例" button and no "激活" button
  const templateRow = templateSection.locator(".el-table__row", {
    hasText: `tmpl-${suffix}`,
  });
  await expect(templateRow).toBeVisible();
  await expect(
    templateRow.getByRole("button", { name: "创建实例" }),
  ).toBeVisible();
  await expect(
    templateRow.getByRole("button", { name: "激活" }),
  ).not.toBeVisible();

  // Instance section: should show the instance, not the template
  const instanceSection = page
    .locator(".deployment-instance-list-page__section")
    .nth(1);
  await expect(instanceSection).toContainText(`inst-${suffix}`);
  await expect(instanceSection).not.toContainText(`tmpl-${suffix}`);

  // Instance row should have "激活" button and no "创建实例" button
  const instanceRow = instanceSection.locator(".el-table__row", {
    hasText: `inst-${suffix}`,
  });
  await expect(instanceRow).toBeVisible();
  await expect(instanceRow.getByRole("button", { name: "激活" })).toBeVisible();
  await expect(
    instanceRow.getByRole("button", { name: "创建实例" }),
  ).not.toBeVisible();

  // Template section should only have the environment filter; page-size selects
  // live outside the filter toolbar and are intentionally ignored here.
  const templateFilters = templateSection.locator(
    ".deployment-instance-list-page__filters .el-select",
  );
  await expect(templateFilters).toHaveCount(1);

  // Instance section should have environment + status filters.
  const instanceFilters = instanceSection.locator(
    ".deployment-instance-list-page__filters .el-select",
  );
  await expect(instanceFilters).toHaveCount(2);
});

test("deployment archive drawer: archive → restore → permanently delete → reuse key", async ({
  page,
}) => {
  const suffix = Date.now().toString();
  const deploymentKey = `archive-${suffix}`;
  const deploymentName = `Archive Target ${suffix}`;

  await login(page);
  const projectId = await createProject(page, suffix);

  const envResponse = await postWithCsrf(
    page,
    `/api/projects/${projectId}/environments`,
    { data: { code: `archive-env-${suffix}`, name: `Archive Env ${suffix}` } },
  );
  expect(envResponse.ok()).toBeTruthy();
  const env = (await envResponse.json()) as { id: number };

  const deploymentResponse = await postWithCsrf(
    page,
    "/api/deployment-instances",
    {
      data: {
        project_id: projectId,
        environment_id: env.id,
        deployment_key: deploymentKey,
        name: deploymentName,
        is_template: false,
      },
    },
  );
  expect(deploymentResponse.ok()).toBeTruthy();

  await page.goto(`/projects/${projectId}/deployments`);

  const instanceSection = page
    .locator(".deployment-instance-list-page__section")
    .nth(1);
  const row = instanceSection.locator(".el-table__row", {
    hasText: deploymentKey,
  });
  await expect(row).toBeVisible({ timeout: 10_000 });
  await expect(row).toContainText("未启用");

  await row.getByRole("button", { name: "归档" }).click();
  const archiveConfirm = page.getByRole("dialog", { name: "归档部署实例" });
  await expect(archiveConfirm).toContainText("确认归档部署实例");
  await archiveConfirm.getByRole("button", { name: "OK" }).click();
  await expect(row).not.toBeVisible({ timeout: 10_000 });

  await page.getByRole("button", { name: "已归档实例" }).click();
  const drawer = page.getByRole("dialog", { name: "已归档实例" });
  await expect(drawer).toBeVisible({ timeout: 10_000 });

  const archivedRow = drawer.locator(".el-table__row", {
    hasText: deploymentKey,
  });
  await expect(archivedRow).toBeVisible({ timeout: 10_000 });
  await archivedRow.getByRole("button", { name: "恢复" }).click();
  const restoreConfirm = page.getByRole("dialog", { name: "恢复部署实例" });
  await expect(restoreConfirm).toContainText("确认恢复部署实例");
  await restoreConfirm.getByRole("button", { name: "OK" }).click();
  await expect(archivedRow).not.toBeVisible({ timeout: 10_000 });

  await page.keyboard.press("Escape");
  await expect(drawer).not.toBeVisible({ timeout: 10_000 });
  await expect(row).toBeVisible({ timeout: 10_000 });
  await expect(row).toContainText("未启用");

  await row.getByRole("button", { name: "归档" }).click();
  await page
    .getByRole("dialog", { name: "归档部署实例" })
    .getByRole("button", { name: "OK" })
    .click();
  await expect(row).not.toBeVisible({ timeout: 10_000 });

  await page.getByRole("button", { name: "已归档实例" }).click();
  const deleteDrawer = page.getByRole("dialog", { name: "已归档实例" });
  const deleteRow = deleteDrawer.locator(".el-table__row", {
    hasText: deploymentKey,
  });
  await expect(deleteRow).toBeVisible({ timeout: 10_000 });

  await deleteRow.getByRole("button", { name: "永久删除" }).click();
  const deletePrompt = page.getByRole("dialog", {
    name: "永久删除部署实例",
  });
  await expect(deletePrompt).toContainText("此操作不可撤销");
  await deletePrompt.getByRole("textbox").fill(deploymentKey);
  await deletePrompt.getByRole("button", { name: "永久删除" }).click();
  await expect(deleteRow).not.toBeVisible({ timeout: 10_000 });

  const reusedResponse = await postWithCsrf(page, "/api/deployment-instances", {
    data: {
      project_id: projectId,
      environment_id: env.id,
      deployment_key: deploymentKey,
      name: `Reused ${deploymentName}`,
      is_template: false,
    },
  });
  expect(reusedResponse.ok()).toBeTruthy();

  await page.goto(`/projects/${projectId}/deployments`);
  const reusedRow = instanceSection.locator(".el-table__row", {
    hasText: `Reused ${deploymentName}`,
  });
  await expect(reusedRow).toBeVisible({ timeout: 10_000 });
  await expect(reusedRow).toContainText(deploymentKey);
});
