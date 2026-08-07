import { expect, test, type Page } from "@playwright/test";

const TEAM_ID = "39aa8884-22cc-4764-a9e7-7df7c7619ba6";
const activityUrl = `/en/teams/${TEAM_ID}/activity`;

async function login(page: Page, email: string) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password", { exact: true }).fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//);
}

test.describe("Team activity (D1)", () => {
  test("Manager reaches automation runs from the navigation", async ({ page }) => {
    await login(page, "manager@opswarden.local");

    await page.getByRole("link", { name: "Activity", exact: true }).first().click();
    await expect(page).toHaveURL(new RegExp(`/teams/${TEAM_ID}/activity$`));
    await expect(page.getByRole("heading", { name: "Activity", level: 1 })).toBeVisible();
    await expect(page.getByRole("tab", { name: "Settings" })).toHaveCount(0);
  });

  test("Responder is not led to a Manager-only surface", async ({ page }) => {
    await login(page, "responder@opswarden.local");
    await expect(page.getByRole("link", { name: "Activity", exact: true })).toHaveCount(0);

    await page.goto(activityUrl);
    await expect(page.getByText("Manager access required")).toBeVisible();
  });
});
