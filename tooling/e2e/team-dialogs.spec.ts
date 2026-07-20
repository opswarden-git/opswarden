import { expect, test, type APIRequestContext, type Page } from "@playwright/test";

const API_URL = process.env.OPSWARDEN_API_URL ?? "http://localhost:8080";
const TEAMS_URL = "/en/teams";

async function login(page: Page, email: string) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password", { exact: true }).fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//);
  await page.goto(TEAMS_URL);
}

async function managerToken(request: APIRequestContext) {
  const response = await request.post(`${API_URL}/api/auth/sign-in`, {
    data: { email: "manager@opswarden.local", password: "sudo" },
  });
  expect(response.ok()).toBe(true);
  return ((await response.json()) as { token: string }).token;
}

async function createJoinableTeam(request: APIRequestContext) {
  const token = await managerToken(request);
  const name = `E2E team dialog join ${Date.now()}`;
  const response = await request.post(`${API_URL}/api/teams`, {
    headers: { Authorization: `Bearer ${token}` },
    data: { name },
  });
  expect(response.ok()).toBe(true);
  return (await response.json()) as {
    invitation_code: string;
    name: string;
    team_id: string;
  };
}

test("Create Team owns focus, Escape, restoration and fresh state", async ({ page }) => {
  await login(page, "manager@opswarden.local");
  const trigger = page.getByRole("button", { name: "Create Team", exact: true });
  await trigger.click();

  const dialog = page.getByRole("dialog", { name: "Create New Team" });
  const name = dialog.getByLabel("Organization Name");
  await expect(dialog).toBeVisible();
  await expect(name).toBeFocused();
  await name.fill("Draft workspace");
  await page.keyboard.press("Escape");

  await expect(dialog).toHaveCount(0);
  await expect(trigger).toBeFocused();
  await trigger.click();
  await expect(dialog.getByLabel("Organization Name")).toHaveValue("");
});

test("Manager creates a Team through the shared footer", async ({ page }) => {
  await login(page, "manager@opswarden.local");
  const name = `E2E team dialog create ${Date.now()}`;
  await page.getByRole("button", { name: "Create Team", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Create New Team" });

  await dialog.getByLabel("Organization Name").fill(name);
  await dialog.getByRole("button", { name: "Create", exact: true }).click();

  await expect(dialog).toHaveCount(0);
  await expect(page.getByText(name, { exact: true })).toBeVisible();
});

test("Responder joins a Team with a real invitation code", async ({ page, request }) => {
  const team = await createJoinableTeam(request);
  await login(page, "responder@opswarden.local");
  await page.getByRole("button", { name: "Join Team", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Join Existing Team" });

  await expect(dialog.getByLabel("Invitation Code")).toBeFocused();
  await dialog.getByLabel("Invitation Code").fill(team.invitation_code.toLowerCase());
  await expect(dialog.getByLabel("Invitation Code")).toHaveValue(team.invitation_code);
  await dialog.getByRole("button", { name: "Join", exact: true }).click();

  await expect(dialog).toHaveCount(0);
  await expect(page.getByText(team.name, { exact: true })).toBeVisible();
});

test("Join Team announces errors and clears them on a new open", async ({ page }) => {
  await login(page, "observer@opswarden.local");
  const trigger = page.getByRole("button", { name: "Join Team", exact: true });
  await trigger.click();
  const dialog = page.getByRole("dialog", { name: "Join Existing Team" });

  await dialog.getByLabel("Invitation Code").fill("OPS-NOPE00");
  await dialog.getByRole("button", { name: "Join", exact: true }).click();
  await expect(dialog.getByRole("alert")).toHaveText("Failed to join team. Check your code.");

  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(trigger).toBeFocused();
  await trigger.click();
  await expect(dialog.getByLabel("Invitation Code")).toHaveValue("");
  await expect(dialog.getByRole("alert")).toHaveCount(0);
});
