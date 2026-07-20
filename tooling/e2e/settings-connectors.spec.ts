import { expect, test, type Page } from "@playwright/test";

const TEAM_ID = "39aa8884-22cc-4764-a9e7-7df7c7619ba6";

async function login(page: Page) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill("manager@opswarden.local");
  await page.getByLabel("Password", { exact: true }).fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//);
}

test("account settings keeps a Team-scoped connector entry point", async ({ page }) => {
  await login(page);
  await page.goto("/en/settings");

  await expect(page.getByRole("link", { name: "General", exact: true })).toHaveAttribute(
    "aria-current",
    "page",
  );
  await page.getByRole("link", { name: "Connectors", exact: true }).click();

  await expect(page).toHaveURL(/\/en\/settings\?view=connectors$/);
  await expect(page.getByRole("heading", { name: "Connectors", exact: true })).toBeVisible();
  await expect(page.getByRole("combobox", { name: "Team", exact: true })).toHaveValue(TEAM_ID);
  await expect(page.getByRole("heading", { name: "GitHub", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "HTTP", exact: true })).toBeVisible();
  await expect(page.getByRole("link", { name: "Open automations" })).toHaveAttribute(
    "href",
    `/en/teams/${TEAM_ID}/automations?view=connections`,
  );

  for (const width of [320, 768, 1280, 1920]) {
    await page.setViewportSize({ width, height: 900 });
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - window.innerWidth,
    );
    expect(overflow, `horizontal overflow at ${width}px`).toBeLessThanOrEqual(1);
  }
});
