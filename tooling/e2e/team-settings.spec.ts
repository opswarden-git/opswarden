import { expect, test, type Page } from "@playwright/test";

const TEAM_ID = "50000000-0000-4000-8000-000000000001";
const teamUrl = `/en/teams/${TEAM_ID}/team`;

async function login(page: Page, email: string) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password", { exact: true }).fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//);
}

test.describe("Team", () => {
  test("Manager sees Team identity, a flat member roster and danger actions", async ({ page }) => {
    await login(page, "manager@opswarden.local");
    await page.goto(teamUrl);

    await expect(page.getByRole("heading", { name: "OpsWarden Demo", level: 2 })).toBeVisible();
    await expect(page.getByRole("button", { name: "Add member" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Share join code" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Active members", exact: true })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Inactive members" })).toBeVisible();
    await expect(page.getByLabel("Roles")).toBeAttached();
    await expect(page.getByRole("heading", { name: "Banned members" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Danger", level: 2 })).toBeVisible();
  });

  test("Responder cannot manage members or view invitation code", async ({ page }) => {
    await login(page, "responder@opswarden.local");
    await page.goto(teamUrl);

    await expect(page.getByRole("heading", { name: "OpsWarden Demo", level: 2 })).toBeVisible();
    await expect(page.getByRole("button", { name: "Add member" })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Share join code" })).toHaveCount(0);
    await expect(page.getByRole("heading", { name: "Banned members" })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Leave team" })).toBeVisible();
  });
});
