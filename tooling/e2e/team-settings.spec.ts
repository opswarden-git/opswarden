import { expect, test, type Page } from "@playwright/test";

const TEAM_ID = "39aa8884-22cc-4764-a9e7-7df7c7619ba6";
const teamUrl = `/en/teams/${TEAM_ID}/team`;

async function login(page: Page, email: string) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password", { exact: true }).fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//);
}

test.describe("Team", () => {
  test("Manager sees team identity, invitation, two member rosters and danger actions", async ({
    page,
  }) => {
    await login(page, "manager@opswarden.local");
    await page.goto(teamUrl);

    await expect(page.getByRole("heading", { name: "OpsWarden Demo", level: 2 })).toBeVisible();
    await expect(page.getByRole("button", { name: "Share join code" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Active members", level: 3 })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Banned members", level: 3 })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Danger", level: 2 })).toBeVisible();
  });

  test("Responder cannot manage members or view invitation code", async ({ page }) => {
    await login(page, "responder@opswarden.local");
    await page.goto(teamUrl);

    await expect(page.getByRole("heading", { name: "OpsWarden Demo", level: 2 })).toBeVisible();
    await expect(page.getByRole("button", { name: "Share join code" })).toHaveCount(0);
    await expect(page.getByRole("heading", { name: "Banned members" })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Leave team" })).toBeVisible();
  });
});
