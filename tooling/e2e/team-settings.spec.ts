import { expect, test, type Page } from "@playwright/test";

const TEAM_ID = "39aa8884-22cc-4764-a9e7-7df7c7619ba6";
const settingsUrl = `/en/teams/${TEAM_ID}/settings`;

async function login(page: Page, email: string) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password", { exact: true }).fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//);
}

test.describe("Progressive Team Settings (L13)", () => {
  test("Manager can toggle Ownership and Banned Members disclosures", async ({ page }) => {
    await login(page, "manager@opswarden.local");
    await page.goto(settingsUrl);

    await expect(page.getByRole("heading", { name: "Team Identity", level: 2 })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Invitation Code", level: 2 })).toBeVisible();

    // Ownership Transfer section is collapsible and closed by default
    const ownershipBtn = page.getByRole("button", { name: /Manager Ownership/ });
    await expect(ownershipBtn).toBeVisible();
    await expect(ownershipBtn).toHaveAttribute("aria-expanded", "false");
    await expect(page.getByText("Select team member")).toBeHidden();

    // Toggle Ownership Transfer open
    await ownershipBtn.click();
    await expect(ownershipBtn).toHaveAttribute("aria-expanded", "true");
    await expect(page.getByText("Select team member")).toBeVisible();

    // Banned Members section is collapsible and closed by default
    const bansBtn = page.getByRole("button", { name: /Banned Members/ });
    await expect(bansBtn).toBeVisible();
    await expect(bansBtn).toHaveAttribute("aria-expanded", "false");

    // Toggle Banned Members open
    await bansBtn.click();
    await expect(bansBtn).toHaveAttribute("aria-expanded", "true");
    await expect(page.getByRole("tab", { name: "Active" })).toBeVisible();

    // Danger zone is visible at the bottom
    await expect(page.getByRole("heading", { name: "Danger Zone", level: 2 })).toBeVisible();
  });

  test("Responder cannot manage members or view invitation code", async ({ page }) => {
    await login(page, "responder@opswarden.local");
    await page.goto(settingsUrl);

    await expect(page.getByRole("heading", { name: "Team Identity", level: 2 })).toBeVisible();
    await expect(page.getByRole("button", { name: /Manager Ownership/ })).toHaveCount(0);
    await expect(page.getByRole("button", { name: /Banned Members/ })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Leave team" })).toBeVisible();
  });
});
