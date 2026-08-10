import { expect, test, type Page } from "@playwright/test";

const TEAM_ID = "39aa8884-22cc-4764-a9e7-7df7c7619ba6";
const INCIDENT_ID = "10000000-0000-4000-8000-000000000001";
const RELEASE_ID = "30000000-0000-4000-8000-000000000001";

async function login(page: Page) {
  await page.goto("/en/login", { waitUntil: "domcontentloaded" });
  await page.getByLabel("Email").fill("manager@opswarden.local");
  await page.getByLabel("Password", { exact: true }).fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//, { timeout: 15_000 });
}

test("detail chrome preserves Release hierarchy and leaves the War Room immersive", async ({
  page,
}) => {
  await login(page);

  await page.goto(`/en/teams/${TEAM_ID}/incidents/${INCIDENT_ID}?view=escalated`);
  await expect(page.getByRole("navigation", { name: "Breadcrumb" })).toHaveCount(0);

  await page.goto(`/en/teams/${TEAM_ID}/releases/${RELEASE_ID}?view=all`);
  const breadcrumb = page.getByRole("navigation", { name: "Breadcrumb" });
  await expect(breadcrumb).toHaveCount(1);
  await expect(breadcrumb.getByRole("link")).toHaveCount(2);
  await expect(breadcrumb).toContainText("OpsWarden Demo/Releases/Release details");
  await expect(
    breadcrumb.getByRole("button", { name: "Current team: OpsWarden Demo" }),
  ).toBeVisible();
  await expect(breadcrumb.getByRole("link", { name: "Releases" })).toHaveAttribute(
    "href",
    `/en/teams/${TEAM_ID}/releases?view=all`,
  );
  await expect(breadcrumb.getByRole("link").last()).toHaveAttribute("aria-current", "page");
});

test("incident records switch morphology without losing operational context", async ({ page }) => {
  await login(page);

  for (const viewportWidth of [320, 768, 1280, 1920]) {
    await page.setViewportSize({ width: viewportWidth, height: 900 });
    await page.goto(`/en/teams/${TEAM_ID}/incidents`);

    const mobile = page.locator('[data-incident-layout="mobile"]');
    const desktop = page.locator('[data-incident-layout="desktop"]');
    if (viewportWidth < 1024) {
      await expect(mobile).toBeVisible();
      await expect(desktop).toBeHidden();
      const record = mobile
        .getByRole("listitem")
        .filter({ hasText: "Payment API returning 502 in Europe" });
      await expect(record.locator('[data-incident-field="identity"]')).toContainText("ID:");
      await expect(record.locator('[data-incident-field="status"]')).toContainText("Open");
      await expect(record.locator('[data-incident-field="assignee"]')).toContainText(
        "responder@opswarden.local",
      );
      await expect(record.locator('[data-incident-field="age"]')).not.toBeEmpty();
      await expect(record.getByRole("link")).toHaveCount(1);
    } else {
      await expect(desktop).toBeVisible();
      await expect(mobile).toBeHidden();
      const table = desktop.getByRole("table", { name: "Incident queue" });
      const rowHeaders = table.getByRole("rowheader");
      await expect(rowHeaders.first()).toBeVisible();
      expect(await table.getByRole("link").count()).toBe(await rowHeaders.count());
    }

    expect(
      await page.evaluate(() => document.documentElement.scrollWidth - innerWidth),
      `incident morphology overflow at ${viewportWidth}px`,
    ).toBeLessThanOrEqual(1);
  }
});

test("Team collections keep scope in a compact breadcrumb", async ({ page }) => {
  await login(page);

  for (const route of [
    { path: `/en/teams/${TEAM_ID}/incidents`, trail: "OpsWarden Demo/Incidents" },
    { path: `/en/teams/${TEAM_ID}/releases`, trail: "OpsWarden Demo/Releases" },
    {
      path: `/en/teams/${TEAM_ID}/integrations`,
      trail: "OpsWarden Demo/Integrations",
    },
  ]) {
    await page.goto(route.path);
    await expect(page.getByRole("button", { name: "Current team: OpsWarden Demo" })).toBeVisible();
    const breadcrumb = page.getByRole("navigation", { name: "Breadcrumb" });
    await expect(breadcrumb).toHaveCount(1);
    await expect(breadcrumb).toContainText(route.trail);
    await expect(breadcrumb.getByRole("link").last()).toHaveAttribute("aria-current", "page");
  }
});

test("collection filters live in table headers on desktop and one sheet on mobile", async ({
  page,
}) => {
  await login(page);

  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto(`/en/teams/${TEAM_ID}/incidents`);
  const table = page.getByRole("table", { name: "Incident queue" });
  await expect(table.getByRole("combobox", { name: "Status" })).toBeAttached();
  await expect(table.getByRole("combobox", { name: "Assignee" })).toBeAttached();
  await expect(table.getByRole("combobox", { name: "Severity" })).toBeAttached();
  await expect(table.getByRole("button", { name: "Age" })).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Incident views" })).toHaveCount(0);

  await page.setViewportSize({ width: 320, height: 800 });
  await expect(page.getByRole("button", { name: /Incident filters/ })).toBeVisible();
  await page.getByRole("button", { name: /Incident filters/ }).click();
  const filters = page.getByRole("dialog", { name: "Incident filters" });
  await expect(filters.getByRole("combobox", { name: "Status" })).toBeVisible();
  await expect(filters.getByRole("combobox", { name: "Severity" })).toBeVisible();
  await expect(filters.getByRole("combobox", { name: "Assignee" })).toBeVisible();
});

test("Rules and Runs reuse the collection header contract", async ({ page }) => {
  await login(page);
  await page.setViewportSize({ width: 1280, height: 900 });

  await page.goto(`/en/teams/${TEAM_ID}/rules`);
  const rules = page.getByRole("table", { name: "Automation rules" });
  await expect(rules.getByRole("combobox", { name: "Status" })).toBeAttached();
  await expect(rules.getByRole("button", { name: "Next run" })).toBeVisible();
  await expect(rules.getByRole("button", { name: "Updated" })).toBeVisible();

  await page.route(`**/api/teams/${TEAM_ID}/automation-runs*`, async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify([
        {
          id: "50000000-0000-4000-8000-000000000001",
          delivery_id: "60000000-0000-4000-8000-000000000001",
          rule_id: null,
          status: "succeeded",
          incident_id: null,
          error_code: null,
          started_at: "2026-08-10T08:00:00Z",
          finished_at: "2026-08-10T08:00:03Z",
        },
      ]),
    });
  });
  await page.goto(`/en/teams/${TEAM_ID}/runs`);
  const runs = page.getByRole("table");
  await expect(runs.getByRole("combobox", { name: "Rule" })).toBeAttached();
  await expect(runs.getByRole("combobox", { name: "Status" })).toBeAttached();
  await expect(runs.getByRole("button", { name: "Started" })).toBeVisible();
  await expect(runs.getByRole("button", { name: "Duration" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Refresh" })).toBeVisible();
});

test("Incident details display as a bottom sheet on mobile", async ({ page }) => {
  await login(page);

  for (const viewportWidth of [320, 768, 1280, 1920]) {
    await page.setViewportSize({ width: viewportWidth, height: 900 });
    await page.goto(`/en/teams/${TEAM_ID}/incidents/${INCIDENT_ID}`);

    if (viewportWidth < 1024) {
      const contextButton = page.getByRole("button", { name: "Details" });
      await expect(contextButton).toBeVisible();

      // Open the sheet
      await contextButton.click();
      const dialog = page.getByRole("dialog", { name: "Incident details" });
      await expect(dialog).toBeVisible();

      await expect(
        dialog.getByRole("heading", { name: "Incident details", exact: true }),
      ).toBeVisible();

      // Close it
      await page.keyboard.press("Escape");
      await expect(dialog).toBeHidden();
    } else {
      await expect(page.getByRole("button", { name: "Details" })).toBeHidden();

      const contextPanel = page.getByRole("complementary", { name: "Incident details" });
      await expect(contextPanel).toBeVisible();
    }
  }
});
