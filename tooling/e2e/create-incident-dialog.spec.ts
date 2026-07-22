import { expect, test, type Page } from "@playwright/test";

const TEAM_ID = "39aa8884-22cc-4764-a9e7-7df7c7619ba6";
const INCIDENTS_URL = `/en/teams/${TEAM_ID}/incidents`;

async function loginAsManager(page: Page) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill("manager@opswarden.local");
  await page.getByLabel("Password", { exact: true }).fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//);
  await page.goto(INCIDENTS_URL);
}

async function openCreateIncident(page: Page) {
  const trigger = page.getByRole("button", { name: "Declare Incident", exact: true });
  await trigger.click();
  const dialog = page.getByRole("dialog", { name: "Declare New Incident" });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByLabel("Title", { exact: true })).toBeFocused();
  return { dialog, trigger };
}

test("Manager can cancel with Escape and returns to the trigger", async ({ page }) => {
  await loginAsManager(page);
  const { dialog, trigger } = await openCreateIncident(page);

  await dialog.getByLabel("Title", { exact: true }).fill("Draft incident");
  await page.keyboard.press("Escape");

  await expect(dialog).toHaveCount(0);
  await expect(trigger).toBeFocused();
});

test("Manager can declare an incident through the shared footer", async ({ page }) => {
  await loginAsManager(page);
  const { dialog } = await openCreateIncident(page);
  const title = `E2E dialog contract ${Date.now()}`;

  await dialog.getByLabel("Title", { exact: true }).fill(title);
  await dialog.getByLabel("Description", { exact: true }).fill("Created by the L03 browser test.");
  await dialog.getByLabel("Severity", { exact: true }).selectOption("critical");
  await dialog.getByRole("button", { name: "Declare", exact: true }).click();

  await expect(dialog).toHaveCount(0);
  await expect(
    page.locator("[data-incident-layout]:visible").getByRole("link", { name: title, exact: true }),
  ).toBeVisible();
});

test("server errors stay in the dialog and are announced", async ({ page }) => {
  await page.route("**/api/incidents", async (route) => {
    if (route.request().method() !== "POST") return route.continue();
    await route.fulfill({
      status: 500,
      contentType: "application/json",
      body: JSON.stringify({ code: "unexpected_error" }),
    });
  });
  await loginAsManager(page);
  const { dialog } = await openCreateIncident(page);

  await dialog.getByLabel("Title", { exact: true }).fill("Rejected incident");
  await dialog.getByRole("button", { name: "Declare", exact: true }).click();

  await expect(dialog.getByRole("alert")).toHaveText("Failed to create incident");
  await expect(dialog.locator('[data-dialog-part="footer"]')).toBeVisible();
});

test("small viewport scrolls the body while keeping the footer visible and focus trapped", async ({
  page,
}) => {
  await page.setViewportSize({ width: 320, height: 320 });
  await loginAsManager(page);
  const { dialog } = await openCreateIncident(page);
  const body = dialog.locator('[data-dialog-part="body"]');
  const footer = dialog.locator('[data-dialog-part="footer"]');

  const geometry = await dialog.evaluate((element) => {
    const body = element.querySelector<HTMLElement>('[data-dialog-part="body"]')!;
    const footer = element.querySelector<HTMLElement>('[data-dialog-part="footer"]')!;
    const dialogBox = element.getBoundingClientRect();
    const footerBox = footer.getBoundingClientRect();
    return {
      bodyScrolls: body.scrollHeight > body.clientHeight,
      dialogTop: dialogBox.top,
      dialogBottom: dialogBox.bottom,
      footerBottom: footerBox.bottom,
    };
  });

  expect(geometry.bodyScrolls).toBe(true);
  expect(geometry.dialogTop).toBeGreaterThanOrEqual(0);
  expect(geometry.dialogBottom).toBeLessThanOrEqual(320);
  expect(geometry.footerBottom).toBeLessThanOrEqual(320);
  await expect(footer).toBeVisible();
  await expect(body).toBeVisible();

  for (let step = 0; step < 8; step += 1) await page.keyboard.press("Tab");
  expect(
    await dialog.evaluate((element) => element.contains(document.activeElement)),
    "focus remains inside the modal",
  ).toBe(true);
});
