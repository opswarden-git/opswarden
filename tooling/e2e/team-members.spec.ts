import { expect, test, type Page } from "@playwright/test";

const TEAM_ID = "39aa8884-22cc-4764-a9e7-7df7c7619ba6";
const membersUrl = `/en/teams/${TEAM_ID}/members`;

async function login(page: Page, email: string) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password", { exact: true }).fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//);
}

test.describe("Team roster and members", () => {
  test("Manager can manage members across 4 viewports", async ({ page }) => {
    await login(page, "manager@opswarden.local");

    for (const width of [320, 768, 1280, 1920]) {
      await page.setViewportSize({ width, height: 900 });
      await page.goto(membersUrl);
      
      // Wait for members list to load
      await expect(page.getByRole("heading", { name: "Members" })).toBeVisible();

      // Ensure the search box exists
      await expect(page.getByPlaceholder("Search members by email or role")).toBeVisible();

      // Find the observer row
      const observerRow = page.locator('li').filter({ hasText: 'observer@opswarden.local' });
      await expect(observerRow).toBeVisible();

      // Manager should see the "Message" and "Actions" buttons for other users
      // Note: Because we duplicated the DOM for responsiveness (md:hidden / md:block),
      // we just check that at least one visible instance exists.
      const messageBtn = observerRow.getByRole("button", { name: "Message" });
      const actionsBtn = observerRow.getByRole("button", { name: "Team Actions" });
      
      // Playwright's toBeVisible() will check if *any* matching element is visible if multiple exist
      // But we can filter by visible if there are multiple.
      await expect(messageBtn.locator("visible=true")).toHaveCount(1);
      await expect(actionsBtn.locator("visible=true")).toHaveCount(1);
    }
  });

  test("Responder can view members and send DM but cannot manage members", async ({ page }) => {
    await login(page, "responder@opswarden.local");
    await page.goto(membersUrl);
    
    await expect(page.getByRole("heading", { name: "Members" })).toBeVisible();

    const managerRow = page.locator('li').filter({ hasText: 'manager@opswarden.local' });
    await expect(managerRow).toBeVisible();

    // Responder can message the manager
    const messageBtn = managerRow.getByRole("button", { name: "Message" }).locator("visible=true");
    await expect(messageBtn).toHaveCount(1);

    // Responder CANNOT see management actions
    const actionsBtn = managerRow.getByRole("button", { name: "Team Actions" });
    await expect(actionsBtn).toHaveCount(0);
  });

  test("Observer can view members and send DM but cannot manage members", async ({ page }) => {
    await login(page, "observer@opswarden.local");
    await page.goto(membersUrl);
    
    await expect(page.getByRole("heading", { name: "Members" })).toBeVisible();

    const responderRow = page.locator('li').filter({ hasText: 'responder@opswarden.local' });
    await expect(responderRow).toBeVisible();

    // Observer can message the responder
    const messageBtn = responderRow.getByRole("button", { name: "Message" }).locator("visible=true");
    await expect(messageBtn).toHaveCount(1);

    // Observer CANNOT see management actions
    const actionsBtn = responderRow.getByRole("button", { name: "Team Actions" });
    await expect(actionsBtn).toHaveCount(0);
  });
});
