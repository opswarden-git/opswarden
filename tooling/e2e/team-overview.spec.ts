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
  test("Manager gets one cross-resource inbox, each item listed once", async ({ page }) => {
    await login(page, "manager@opswarden.local");
    await page.goto(overviewUrl);

    await expect(page.getByRole("heading", { name: "OpsWarden Demo" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Needs your attention" })).toBeVisible();
    await expect(page.getByLabel("Current team")).toHaveCount(0);

    // A blocked Release and the Incident blocking it both belong in one queue:
    // that is what makes this screen an inbox rather than a list of Incidents.
    await expect(
      inbox(page).getByText("v2.8.0 — Payment resilience", { exact: true }),
    ).toBeVisible();
    await expect(
      inbox(page).getByText("Payment API returning 502 in Europe", { exact: true }),
    ).toBeVisible();

    // It used to appear twice -- once here, once in a side panel repeating it.
    await expect(page.getByText("v2.8.0 — Payment resilience", { exact: true })).toHaveCount(1);
  });

  test("a facet narrows the queue in place instead of leaving the screen", async ({ page }) => {
    await login(page, "manager@opswarden.local");
    await page.goto(overviewUrl);

    const blocked = facets(page).getByRole("link", { name: /Blocked releases/ });
    await expect(blocked).toBeVisible();
    await blocked.click();

    // Still on the overview: a facet is a view onto the inbox, not a drill-down.
    await expect(page).toHaveURL(`${overviewUrl}?view=blocked`);
    await expect(page.getByRole("heading", { name: "Needs your attention" })).toBeVisible();
    await expect(
      inbox(page).getByText("v2.8.0 — Payment resilience", { exact: true }),
    ).toBeVisible();
    await expect(
      inbox(page).getByText("Payment API returning 502 in Europe", { exact: true }),
    ).toHaveCount(0);

    await facets(page).getByRole("link", { name: /All/ }).click();
    await expect(page).toHaveURL(overviewUrl);
  });

  test("Responder gets an assigned facet and executable Release work", async ({ page }) => {
    await login(page, "responder@opswarden.local");
    await page.goto(overviewUrl);

    await expect(facets(page).getByRole("link", { name: /Assigned to me/ })).toBeVisible();
    await expect(inbox(page).getByText(/Next step ready:/)).toBeVisible();
    await expect(queue(page).getByRole("link").first()).toHaveAttribute(
      "href",
      new RegExp(`/en/teams/${TEAM_ID}/(incidents|releases)/[0-9a-f-]+$`),
    );
  });

  test("Observer gets a read-only scope without an assignment facet", async ({ page }) => {
    await login(page, "observer@opswarden.local");
    await page.goto(overviewUrl);

    // Observers hold no assignments, so the facet would always read zero.
    await expect(facets(page).getByRole("link", { name: /Assigned to me/ })).toHaveCount(0);
    await expect(facets(page).getByRole("link", { name: /Unacknowledged/ })).toBeVisible();
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

      // The facets lead the queue they filter, at every width.
      const facetBar = await facets(page).boundingBox();
      const queue = await inbox(page).boundingBox();
      expect(facetBar).not.toBeNull();
      expect(queue).not.toBeNull();
      expect(facetBar!.y).toBeGreaterThanOrEqual(queue!.y);
    }
  });
});
