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

async function createProject(page: Page, suffix: string): Promise<number> {
  const response = await page.request.post("/api/projects", {
    data: {
      code: `e2e-project-${suffix}`,
      name: `E2E Project ${suffix}`,
      description: "Isolated Playwright project",
    },
  });
  expect(response.ok()).toBeTruthy();
  const payload = (await response.json()) as { id: number };
  return payload.id;
}

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
  const createEnvironmentButton = page
    .getByRole("button", { name: "新建环境" })
    .first();
  await expect(createEnvironmentButton).toBeVisible();
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

  const envResponse = await page.request.post(
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

  const cfResponse = await page.request.post("/api/config-files", {
    data: {
      project_id: projectId,
      code: `e2e-cfg-${suffix}`,
      name: `E2E Config ${suffix}`,
      format: "yaml",
    },
  });
  expect(cfResponse.ok()).toBeTruthy();
  const configFile = (await cfResponse.json()) as { id: number };

  const diResponse = await page.request.post("/api/deployment-instances", {
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

  const editor = page.locator(".draft-editor-page__editor textarea");
  await expect(editor).toBeVisible({ timeout: 10_000 });

  // Saved versions panel should be visible for admin, initially empty
  const panel = page.locator(".draft-editor-page__history");
  await expect(panel).toBeVisible();
  await expect(panel).toContainText("暂无 Saved Version");

  // ---- Save draft #1 -------------------------------------------------------
  await editor.fill("key: value_1");
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

  await editor.fill("key: value_2");
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
  await expect(editor).toHaveValue("key: value_1", { timeout: 5_000 });

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

  const envResponse = await page.request.post(
    `/api/projects/${projectId}/environments`,
    { data: { code: `e2e-env-${suffix}`, name: `E2E Env ${suffix}` } },
  );
  expect(envResponse.ok()).toBeTruthy();
  const env = (await envResponse.json()) as { id: number };

  const cfResponse = await page.request.post("/api/config-files", {
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
  const srcResponse = await page.request.post("/api/deployment-instances", {
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
  const tgtResponse = await page.request.post("/api/deployment-instances", {
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
  const putDraftResponse = await page.request.put(
    `/api/drafts/${srcDeployment.id}/${configFile.id}`,
    { data: { content: draftContent, format: "yaml", base_version: 0 } },
  );
  expect(putDraftResponse.ok()).toBeTruthy();

  // --- navigate to target's draft editor ---
  await page.goto(
    `/projects/${projectId}/deployments/${tgtDeployment.id}/configs/${configFile.id}/draft`,
  );
  const editor = page.locator(".draft-editor-page__editor textarea");
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
  await expect(editor).toHaveValue(draftContent, { timeout: 5_000 });
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

  const envResponse = await page.request.post(
    `/api/projects/${projectId}/environments`,
    { data: { code: `e2e-env-${suffix}`, name: `E2E Env ${suffix}` } },
  );
  expect(envResponse.ok()).toBeTruthy();
  const env = (await envResponse.json()) as { id: number };

  const cfResponse = await page.request.post("/api/config-files", {
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
    const r = await page.request.post("/api/deployment-instances", {
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
    const r = await page.request.put(`/api/drafts/${id}/${configFile.id}`, {
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
  const editor = page.locator(".draft-editor-page__editor textarea");
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
  const editor = page.locator(".draft-editor-page__editor textarea");
  await expect(editor).toBeVisible({ timeout: 10_000 });

  // Write and save a draft
  await editor.fill("greeting: hello-release-test");
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
  const envResponse = await page.request.post(
    `/api/projects/${projectId}/environments`,
    { data: { code: `e2e-env-${suffix}`, name: `E2E Env ${suffix}` } },
  );
  expect(envResponse.ok()).toBeTruthy();
  const env = (await envResponse.json()) as { id: number };

  // Create a template via API
  const tmplResponse = await page.request.post("/api/deployment-instances", {
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
  const instResponse = await page.request.post("/api/deployment-instances", {
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
