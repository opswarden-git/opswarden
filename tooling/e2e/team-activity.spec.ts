import { expect, test, type Page } from "@playwright/test";

const TEAM_ID = "50000000-0000-4000-8000-000000000001";
const activityUrl = `/en/teams/${TEAM_ID}/activity`;

async function login(page: Page, email: string) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password", { exact: true }).fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//);
}

test.describe("Automation runs", () => {
  test("Manager reaches Runs from Operations", async ({ page }) => {
    await login(page, "manager@opswarden.local");

    await expect(page.getByRole("link", { name: "Activity", exact: true })).toHaveCount(0);
    await page.getByRole("link", { name: "Runs", exact: true }).first().click();
    await expect(page).toHaveURL(new RegExp(`/teams/${TEAM_ID}/runs$`));
    await expect(page.getByRole("heading", { name: "Runs", level: 1 })).toBeVisible();
    await expect(page.getByRole("link", { name: "Back to rules" })).toHaveCount(0);
  });

  test("Responder is not led to a Manager-only surface", async ({ page }) => {
    await login(page, "responder@opswarden.local");
    await expect(page.getByRole("link", { name: "Activity", exact: true })).toHaveCount(0);
    await expect(page.getByRole("link", { name: "Rules", exact: true })).toHaveCount(0);
    await expect(page.getByRole("link", { name: "Runs", exact: true })).toHaveCount(0);

    await page.goto(activityUrl);
    await expect(page).toHaveURL(new RegExp(`/teams/${TEAM_ID}/runs$`));
    await expect(page.getByText("Manager access required")).toBeVisible();
  });
});
