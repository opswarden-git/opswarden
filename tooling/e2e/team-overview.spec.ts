import { expect, test, type Page } from "@playwright/test";

const TEAM_ID = "39aa8884-22cc-4764-a9e7-7df7c7619ba6";
const overviewUrl = `/en/teams/${TEAM_ID}/overview`;

async function login(page: Page, email: string) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password", { exact: true }).fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//);
}

test.describe("Team operational overview", () => {
  test("Manager gets one cross-resource attention queue with exact drill-downs", async ({
    page,
  }) => {
    await login(page, "manager@opswarden.local");
    await page.goto(overviewUrl);

    await expect(page.getByRole("heading", { name: "OpsWarden Demo" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Operational summary" })).toBeVisible();
    await expect(page.getByLabel("Current team")).toHaveCount(0);
    const attention = page.locator('section[aria-labelledby="attention-title"]');
    await expect(attention.getByText("v2.8.0 — Payment resilience", { exact: true })).toBeVisible();
    await expect(
      attention.getByText("Payment API returning 502 in Europe", { exact: true }),
    ).toBeVisible();
    await expect(page.getByRole("heading", { name: "Your work" })).toBeVisible();

    await page.getByRole("link", { name: /Unacknowledged/ }).click();
    await expect(page).toHaveURL(`/en/teams/${TEAM_ID}/incidents`);

    await page.goto(overviewUrl);
    await page
      .getByRole("link", { name: /Blocked releases/ })
      .first()
      .click();
    await expect(page).toHaveURL(`/en/teams/${TEAM_ID}/releases?view=blocked`);
  });

  test("Responder sees owned incidents and executable Release work", async ({ page }) => {
    await login(page, "responder@opswarden.local");
    await page.goto(overviewUrl);

    await expect(page.getByRole("heading", { name: "Your work" })).toBeVisible();
    await expect(page.getByRole("link", { name: /Assigned to me/ })).toBeVisible();
    const ownedWork = page.locator('section[aria-labelledby="your-work-title"]');
    await expect(ownedWork.getByRole("link").first()).toHaveAttribute(
      "href",
      new RegExp(`/en/teams/${TEAM_ID}/incidents/[0-9a-f-]+$`),
    );
    const attention = page.locator('section[aria-labelledby="attention-title"]');
    await expect(attention.getByText(/Next step ready:/)).toBeVisible();
  });

  test("Observer gets a read-only operational scope without executable prompts", async ({
    page,
  }) => {
    await login(page, "observer@opswarden.local");
    await page.goto(overviewUrl);

    await expect(page.getByRole("heading", { name: "Operational scope" })).toBeVisible();
    await expect(page.getByRole("link", { name: /Active incidents/ })).toBeVisible();
    await expect(page.getByText(/Next step ready:/)).toHaveCount(0);
  });

  test("overview keeps its reading order without horizontal overflow", async ({ page }) => {
    await login(page, "manager@opswarden.local");

    for (const width of [320, 768, 1280, 1920]) {
      await page.setViewportSize({ width, height: 900 });
      await page.goto(overviewUrl);
      await expect(page.getByRole("heading", { name: "Needs your attention" })).toBeVisible();

      const overflow = await page.evaluate(
        () => document.documentElement.scrollWidth - window.innerWidth,
      );
      expect(overflow, `horizontal overflow at ${width}px`).toBeLessThanOrEqual(1);

      const attention = await page
        .locator('section[aria-labelledby="attention-title"]')
        .boundingBox();
      const context = await page.getByLabel("Operational context").boundingBox();
      expect(attention).not.toBeNull();
      expect(context).not.toBeNull();
      if (width < 1024) {
        expect(context!.y).toBeGreaterThan(attention!.y);
      } else {
        expect(context!.x).toBeGreaterThan(attention!.x);
      }
    }
  });
});
