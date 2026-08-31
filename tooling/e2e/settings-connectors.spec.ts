import { expect, test, type Page } from "@playwright/test";
import * as demo from "./demo-env";

async function login(page: Page) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill(demo.DEMO_MANAGER_EMAIL);
  await page.getByLabel("Password", { exact: true }).fill(demo.DEMO_PASSWORD);
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(demo.TEAM_URL_PATTERN);
  await demo.finishGuidedTour(page);
}

test("account settings stays account-scoped and retires the duplicate connector view", async ({
  page,
}) => {
  await login(page);
  await page.goto("/en/settings?view=connectors");

  await expect(page).toHaveURL(/\/en\/settings(\?view=connectors)?$/);
  await expect(page.getByRole("heading", { name: "Settings", exact: true })).toBeVisible();
  await expect(page.getByText(demo.DEMO_MANAGER_EMAIL, { exact: true }).first()).toBeVisible();
  await expect(page.getByRole("link", { name: "Connectors", exact: true })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "GitHub", exact: true })).toHaveCount(0);

  for (const width of [320, 768, 1280, 1920]) {
    await page.setViewportSize({ width, height: 900 });
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - window.innerWidth,
    );
    expect(overflow, `horizontal overflow at ${width}px`).toBeLessThanOrEqual(1);
  }
});
