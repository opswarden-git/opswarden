import { expect, test } from "@playwright/test";
const OUT = process.env.SHOT_DIR!;
const TEAM = "39aa8884-22cc-4764-a9e7-7df7c7619ba6";

test("member paths", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/en/login");
  await page.getByLabel(/email/i).first().fill("manager@opswarden.local");
  await page.getByLabel(/password/i).first().fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//);
  await page.goto(`/en/teams/${TEAM}/team`);
  await page.waitForTimeout(1200);
  await page.screenshot({ path: `${OUT}/team-actions.png` });

  await page.getByRole("button", { name: "Add member", exact: true }).click();
  const dlg = page.getByRole("dialog");
  await expect(dlg).toBeVisible();
  await page.waitForTimeout(500);
  console.log("DIALOGUE:", (await dlg.innerText()).replace(/\n/g, " | "));
  await dlg.screenshot({ path: `${OUT}/add-member.png` });
});
