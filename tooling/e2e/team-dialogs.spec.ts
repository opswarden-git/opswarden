import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import * as demo from "./demo-env";

const API_URL = process.env.OPSWARDEN_API_URL ?? "http://localhost:8080";
const TEAMS_URL = "/en/teams";

async function login(page: Page, email: string) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password", { exact: true }).fill(demo.DEMO_PASSWORD);
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(demo.TEAM_URL_PATTERN);
  await demo.finishGuidedTour(page);
  await page.goto(TEAMS_URL);
}

async function managerToken(request: APIRequestContext) {
  const response = await request.post(`${API_URL}/api/auth/sign-in`, {
    data: { email: demo.DEMO_MANAGER_EMAIL, password: demo.DEMO_PASSWORD },
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

test("Create team owns focus, Escape, restoration and fresh state", async ({ page }) => {
  await login(page, demo.DEMO_MANAGER_EMAIL);
  const trigger = page.getByRole("button", { name: "Create team", exact: true });
  await trigger.click();

  const dialog = page.getByRole("dialog", { name: "Create new team" });
  const name = dialog.getByLabel("Team name");
  await expect(dialog).toBeVisible();
  await expect(name).toBeFocused();
  await name.fill("Draft workspace");
  await page.keyboard.press("Escape");

  await expect(dialog).toHaveCount(0);
  await expect(trigger).toBeFocused();
  await trigger.click();
  await expect(dialog.getByLabel("Team name")).toHaveValue("");
});

test("Manager creates a Team from the Workspace controls", async ({ page }) => {
  await login(page, demo.DEMO_MANAGER_EMAIL);
  const name = `E2E team dialog create ${Date.now()}`;
  await page.getByRole("button", { name: "Create team", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Create new team" });

  await dialog.getByLabel("Team name").fill(name);
  await dialog.getByRole("button", { name: "Create", exact: true }).click();

  await expect(dialog).toHaveCount(0);
  await expect(page.getByRole("main").getByRole("link").filter({ hasText: name })).toBeVisible();
});

test("Responder joins a Team with a real invitation code", async ({ page, request }) => {
  const team = await createJoinableTeam(request);
  await login(page, demo.DEMO_RESPONDER_EMAIL);
  await page.getByRole("button", { name: "Join team", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Join existing team" });

  await expect(dialog.getByLabel("Invitation code")).toBeFocused();
  await dialog.getByLabel("Invitation code").fill(team.invitation_code.toLowerCase());
  await expect(dialog.getByLabel("Invitation code")).toHaveValue(team.invitation_code);
  await dialog.getByRole("button", { name: "Join", exact: true }).click();

  await expect(dialog).toHaveCount(0);
  await expect(
    page.getByRole("main").getByRole("link").filter({ hasText: team.name }),
  ).toBeVisible();
});

test("Join Team announces errors and clears them on a new open", async ({ page }) => {
  await login(page, demo.DEMO_OBSERVER_EMAIL);
  const trigger = page.getByRole("button", { name: "Join team", exact: true });
  await trigger.click();
  const dialog = page.getByRole("dialog", { name: "Join existing team" });

  await dialog.getByLabel("Invitation code").fill("OPS-NOPE00");
  await dialog.getByRole("button", { name: "Join", exact: true }).click();
  await expect(dialog.getByRole("alert")).toHaveText(
    "The invitation code is invalid or the team could not be joined.",
  );

  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(trigger).toBeFocused();
  await trigger.click();
  await expect(dialog.getByLabel("Invitation code")).toHaveValue("");
  await expect(dialog.getByRole("alert")).toHaveCount(0);
});
