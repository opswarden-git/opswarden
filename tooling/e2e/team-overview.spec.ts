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

const inbox = (page: Page) => page.locator('section[aria-labelledby="attention-title"]');
const facets = (page: Page) => page.getByRole("navigation", { name: "Attention filters" });
// The facet bar lives inside the section, so anchor on the list itself when the
// assertion is about the queued work rather than the filters above it.
const queue = (page: Page) => page.locator("[data-attention-queue]");

test.describe("Team operational overview", () => {
  test("Manager gets cross-resource overview, separated by entity", async ({ page }) => {
    await login(page, "manager@opswarden.local");
    await page.goto(overviewUrl);

    await expect(page.getByRole("heading", { name: "Overview", level: 1 })).toBeVisible();
    await expect(page.locator('select[aria-label="Current team"]:visible')).toHaveCount(1);

    // Incidents section
    const incidentsSection = page.getByRole("region", { name: "Incidents" });
    await expect(incidentsSection).toBeVisible();
    await expect(incidentsSection.getByText("Payment API returning 502 in Europe", { exact: true })).toBeVisible();

    // Releases section
    const releasesSection = page.getByRole("region", { name: "Releases" });
    await expect(releasesSection).toBeVisible();
    await expect(releasesSection.getByText("v2.8.0 — Payment resilience", { exact: true })).toBeVisible();

    // Runs section
    const runsSection = page.getByRole("region", { name: "Runs" });
    await expect(runsSection).toBeVisible();
  });

  test("Responder can access executable Release work", async ({ page }) => {
    await login(page, "responder@opswarden.local");
    await page.goto(overviewUrl);

    const releasesSection = page.getByRole("region", { name: "Releases" });
    await expect(releasesSection.getByRole("link").first()).toHaveAttribute(
      "href",
      new RegExp(`/en/teams/${TEAM_ID}/releases/[0-9a-f-]+$`),
    );
  });

  test("Observer gets a read-only scope without Runs visibility", async ({ page }) => {
    await login(page, "observer@opswarden.local");
    await page.goto(overviewUrl);

    await expect(page.getByRole("region", { name: "Incidents" })).toBeVisible();
    await expect(page.getByRole("region", { name: "Releases" })).toBeVisible();
    // Observers cannot manage automations, so they don't see Runs
    await expect(page.getByRole("region", { name: "Runs" })).toHaveCount(0);
  });

  test("overview keeps its reading order without horizontal overflow", async ({ page }) => {
    await login(page, "manager@opswarden.local");

    for (const width of [320, 768, 1280, 1920]) {
      await page.setViewportSize({ width, height: 900 });
      await page.goto(overviewUrl);
      await expect(page.getByRole("region", { name: "Incidents" })).toBeVisible();

      const overflow = await page.evaluate(
        () => document.documentElement.scrollWidth - window.innerWidth,
      );
      expect(overflow, `horizontal overflow at ${width}px`).toBeLessThanOrEqual(1);
    }
  });
});
