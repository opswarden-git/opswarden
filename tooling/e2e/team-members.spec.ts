import { expect, test, type Page } from "@playwright/test";
import * as demo from "./demo-env";

const TEAM_ID = demo.DEMO_TEAM_ID;
const membersUrl = `/en/teams/${TEAM_ID}/team#members`;

async function login(page: Page, email: string) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password", { exact: true }).fill(demo.DEMO_PASSWORD);
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(demo.TEAM_URL_PATTERN);
  await demo.finishGuidedTour(page);
}

test.describe("Team roster and members", () => {
  test("Manager can manage members across 4 viewports", async ({ page }) => {
    await login(page, demo.DEMO_MANAGER_EMAIL);

    for (const width of [320, 768, 1280, 1920]) {
      await page.setViewportSize({ width, height: 900 });
      await page.goto(membersUrl);

      // Wait for members list to load
      await expect(
        page.getByRole("heading", { name: "Active members", exact: true }),
      ).toBeVisible();

      await expect(page.getByRole("textbox", { name: "Search by email or role" })).toBeVisible();

      // Find the observer row
      const observerRow = page.locator("li").filter({ hasText: demo.DEMO_OBSERVER_EMAIL });
      await expect(observerRow).toBeVisible();

      // Manager should see the "Chat with" link and "Team actions" buttons for other users
      // Note: Because we duplicated the DOM for responsiveness (md:hidden / md:block),
      // we just check that at least one visible instance exists.
      const messageLink = observerRow.getByRole("link", {
        name: `Chat with ${demo.DEMO_OBSERVER_EMAIL}`,
      });
      const actionsBtn = observerRow.getByRole("button", { name: "Team actions" });

      // Playwright's toBeVisible() will check if *any* matching element is visible if multiple exist
      // But we can filter by visible if there are multiple.
      await expect(messageLink).toHaveCount(1);
      await expect(actionsBtn.locator("visible=true")).toHaveCount(1);
    }
  });

  test("Responder can view members and send DM but cannot manage members", async ({ page }) => {
    await login(page, demo.DEMO_RESPONDER_EMAIL);
    await page.goto(membersUrl);

    await expect(page.getByRole("heading", { name: "Active members", exact: true })).toBeVisible();

    const managerRow = page.locator("li").filter({ hasText: demo.DEMO_MANAGER_EMAIL });
    await expect(managerRow).toBeVisible();

    // Responder can message the manager
    const messageLink = managerRow.getByRole("link", {
      name: `Chat with ${demo.DEMO_MANAGER_EMAIL}`,
    });
    await expect(messageLink).toHaveCount(1);

    // Responder CANNOT see management actions
    const actionsBtn = managerRow.getByRole("button", { name: "Team actions" });
    await expect(actionsBtn).toHaveCount(0);
  });

  test("Observer can view members and send DM but cannot manage members", async ({ page }) => {
    await login(page, demo.DEMO_OBSERVER_EMAIL);
    await page.goto(membersUrl);

    await expect(page.getByRole("heading", { name: "Active members", exact: true })).toBeVisible();

    const responderRow = page.locator("li").filter({ hasText: demo.DEMO_RESPONDER_EMAIL });
    await expect(responderRow).toBeVisible();

    // Observer can message the responder
    const messageLink = responderRow.getByRole("link", {
      name: `Chat with ${demo.DEMO_RESPONDER_EMAIL}`,
    });
    await expect(messageLink).toHaveCount(1);

    // Observer CANNOT see management actions
    const actionsBtn = responderRow.getByRole("button", { name: "Team actions" });
    await expect(actionsBtn).toHaveCount(0);
  });
});
