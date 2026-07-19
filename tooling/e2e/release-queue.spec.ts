import { expect, test, type Page } from "@playwright/test";

const TEAM_ID = "39aa8884-22cc-4764-a9e7-7df7c7619ba6";
const releasesUrl = `/en/teams/${TEAM_ID}/releases`;
const BLOCKED_RELEASE_ID = "30000000-0000-4000-8000-000000000001";
const ACTIVE_RELEASE_ID = "30000000-0000-4000-8000-000000000003";

async function login(page: Page, email: string) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password", { exact: true }).fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//);
}

test.describe("Release queue", () => {
  test("Manager understands a blocked release from the list", async ({ page }) => {
    await login(page, "manager@opswarden.local");
    await page.goto(releasesUrl);

    await expect(page.getByRole("heading", { name: "Releases" })).toBeVisible();
    await expect(page.getByRole("link", { name: /Active\s+1/ })).toHaveAttribute(
      "aria-current",
      "page",
    );
    await expect(
      page.getByText("v2.9.0 — Observability foundations", { exact: true }),
    ).toBeVisible();

    await page.getByRole("link", { name: /Blocked\s+1/ }).click();
    await expect(page).toHaveURL(/view=blocked/);

    const row = page.getByRole("row").filter({ hasText: "v2.8.0 — Payment resilience" });
    await expect(row).toContainText("2/4");
    await expect(row).toContainText("Run payment smoke tests");
    await expect(
      row.getByRole("link", { name: "Payment API returning 502 in Europe" }),
    ).toBeVisible();

    await row.getByRole("link", { name: "Open", exact: true }).click();
    await expect(page).toHaveURL(/releases\/30000000-0000-4000-8000-000000000001\?view=blocked/);
    await expect(
      page.getByRole("heading", { name: "v2.8.0 — Payment resilience" }).first(),
    ).toBeVisible();
    const blocker = page.getByRole("alert").filter({ hasText: "Release blocked" });
    await expect(blocker).toContainText("Payment API returning 502 in Europe");

    await page.goBack();
    await expect(page).toHaveURL(/releases\?view=blocked$/);
    await page.goForward();
    await expect(page).toHaveURL(new RegExp(`/releases/${BLOCKED_RELEASE_ID}\\?view=blocked$`));
    await page.getByRole("link", { name: "Releases", exact: true }).first().click();
    await expect(page).toHaveURL(/view=blocked$/);
  });

  test("Observer gets a read-only queue without document overflow", async ({ page }) => {
    await login(page, "observer@opswarden.local");
    await page.setViewportSize({ width: 320, height: 800 });
    await page.goto(`${releasesUrl}?view=all`);

    await expect(page.getByRole("link", { name: /All\s+4/ })).toHaveAttribute(
      "aria-current",
      "page",
    );
    await expect(page.getByRole("button", { name: "New release" })).toHaveCount(0);
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - window.innerWidth,
    );
    expect(overflow).toBeLessThanOrEqual(1);
  });

  test("Observer understands blockers without action controls", async ({ page }) => {
    await login(page, "observer@opswarden.local");
    await page.goto(`${releasesUrl}/${BLOCKED_RELEASE_ID}`);

    const blocker = page.getByRole("alert").filter({ hasText: "Release blocked" });
    await expect(blocker).toContainText("Release blocked");
    await expect(
      blocker.getByRole("link", { name: "Payment API returning 502 in Europe" }),
    ).toBeVisible();
    await expect(page.getByRole("button", { name: "Validate next step" })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "More release actions" })).toHaveCount(0);
  });

  test("Responder validates only the next ordered step", async ({ page }) => {
    await login(page, "responder@opswarden.local");
    await page.goto(`${releasesUrl}/${ACTIVE_RELEASE_ID}`);

    await expect(page.getByText("Publish dashboards", { exact: true }).first()).toBeVisible();
    await expect(page.getByRole("button", { name: "Validate next step" })).toHaveCount(1);
    await page.getByRole("button", { name: "Validate next step" }).click();
    await expect(page.getByText("1/3", { exact: true })).toBeVisible();
    await expect(page.getByText("Enable tracing sampler", { exact: true }).first()).toBeVisible();
    await expect(page.getByRole("button", { name: "More release actions" })).toHaveCount(0);
  });

  test("Manager creates a release from keyboard-reordered steps", async ({ page }) => {
    await login(page, "manager@opswarden.local");
    await page.goto(releasesUrl);
    await page.getByRole("button", { name: "New release" }).click();

    const dialog = page.getByRole("dialog", { name: "New release" });
    await dialog.getByLabel("Title").fill("E2E ordered deployment");
    await dialog.getByLabel("Step 1").fill("Build artifacts");
    await dialog.getByRole("button", { name: "Add step" }).click();
    await dialog.getByLabel("Step 2").fill("Verify production");
    await dialog.getByRole("button", { name: "Add step" }).click();
    await dialog.getByLabel("Step 3").fill("Temporary cleanup");
    await dialog.getByRole("button", { name: "Remove Temporary cleanup" }).click();
    await dialog.getByLabel("Step 2").press("Alt+ArrowUp");
    await dialog.getByRole("button", { name: "Create", exact: true }).click();

    await expect(page).toHaveURL(new RegExp(`${releasesUrl}/[0-9a-f-]+$`));
    await expect(page.getByRole("heading", { name: "E2E ordered deployment" })).toBeVisible();
    const stepper = page.getByRole("list", { name: "Deployment steps" });
    const items = stepper.getByRole("listitem");
    await expect(items.nth(0)).toContainText("Verify production");
    await expect(items.nth(1)).toContainText("Build artifacts");

    await page.getByRole("button", { name: "More release actions" }).click();
    await page.getByRole("menuitem", { name: "Cancel release" }).click();
    await expect(page.getByRole("dialog", { name: "Cancel release" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Keep" })).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog", { name: "Cancel release" })).toHaveCount(0);
  });
});
