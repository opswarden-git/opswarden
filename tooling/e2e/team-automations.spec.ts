import { expect, test, type Page } from "@playwright/test";
import * as demo from "./demo-env";

const TEAM_ID = demo.DEMO_TEAM_ID;
const rulesUrl = `/en/teams/${TEAM_ID}/rules`;
const integrationsUrl = `/en/teams/${TEAM_ID}/integrations`;

async function openDirectDestination(page: Page, name: "Integrations" | "Rules", width: number) {
  if (width < 768) {
    await page.getByRole("button", { name: "More", exact: true }).click();
    await page
      .getByRole("dialog", { name: "More" })
      .getByRole("link", { name, exact: true })
      .click();
    return;
  }

  await page.getByRole("link", { name, exact: true }).first().click();
}

async function login(page: Page, email: string) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password", { exact: true }).fill(demo.DEMO_PASSWORD);
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(demo.TEAM_URL_PATTERN);
  await demo.finishGuidedTour(page);
}

async function managerToken(page: Page) {
  return page.evaluate(() => {
    const raw = localStorage.getItem("opswarden-auth-storage");
    return raw ? (JSON.parse(raw).state.token as string) : "";
  });
}

async function clearAutomations(page: Page, token: string) {
  const headers = { Authorization: `Bearer ${token}` };
  const rules = await page.request.get(`/api/teams/${TEAM_ID}/automation-rules`, { headers });
  if (rules.ok()) {
    for (const rule of await rules.json()) {
      await page.request.delete(`/api/teams/${TEAM_ID}/automation-rules/${rule.id}`, { headers });
    }
  }
  const connections = await page.request.get(`/api/teams/${TEAM_ID}/service-connections`, {
    headers,
  });
  if (connections.ok()) {
    for (const connection of await connections.json()) {
      await page.request.delete(`/api/teams/${TEAM_ID}/service-connections/${connection.id}`, {
        headers,
      });
    }
  }
}

test.describe("Team automations", () => {
  for (const width of [320, 768, 1280, 1920]) {
    test(`Manager can navigate Rules and Connections at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 800 });
      await login(page, demo.DEMO_MANAGER_EMAIL);
      await page.goto(rulesUrl);
      await expect(page.getByRole("heading", { name: "Rules", level: 1 })).toBeVisible();
      await openDirectDestination(page, "Integrations", width);
      await expect(page).toHaveURL(integrationsUrl);
      await expect(page.getByRole("heading", { name: "Integrations", level: 1 })).toBeVisible();
      await expect(page.getByRole("heading", { name: "GitHub", exact: true })).toBeVisible();
      await expect(page.getByRole("heading", { name: "HTTP", exact: true })).toBeVisible();
    });
  }

  test("Manager connects a service and enables a rule", async ({ page }) => {
    test.setTimeout(45_000);
    await page.setViewportSize({ width: 1280, height: 800 });
    await login(page, demo.DEMO_MANAGER_EMAIL);
    const token = await managerToken(page);
    await clearAutomations(page, token);

    try {
      await page.goto(integrationsUrl);
      await page.getByRole("button", { name: "Connect GitHub" }).click();
      const githubForm = page.getByRole("form", { name: "Connect GitHub" });
      await githubForm.getByLabel("Signing secret").fill("e2e-automation-secret");
      await githubForm.getByRole("button", { name: "Connect", exact: true }).click();
      await expect(page.getByRole("button", { name: "Manage GitHub" })).toBeVisible();

      await openDirectDestination(page, "Rules", 1280);
      await page.getByRole("button", { name: "New rule" }).click();
      await page.getByLabel("Rule name").fill("E2E failed CI to incident");
      await page.getByLabel("Source connection").selectOption({ index: 1 });
      await page.getByRole("button", { name: "Create rule" }).click();
      await expect(page.getByRole("dialog", { name: "New rule" })).toBeHidden();

      const rule = page.getByRole("row", { name: /E2E failed CI to incident/ });
      const ruleState = rule.locator("[data-rule-state]");
      await expect(ruleState).toHaveAttribute("data-rule-state", "disabled");
      await rule.getByRole("button", { name: "Rule actions" }).click();
      await page.getByRole("menuitem", { name: "Enable" }).click();
      await expect(ruleState).toHaveAttribute("data-rule-state", "enabled");
    } finally {
      await clearAutomations(page, token);
    }
  });

  test("non-Managers do not receive configuration controls", async ({ page }) => {
    await login(page, demo.DEMO_RESPONDER_EMAIL);
    await page.goto(`/en/teams/${TEAM_ID}/overview`);
    await expect(page.getByRole("link", { name: "Rules", exact: true })).toHaveCount(0);
    await expect(page.getByRole("link", { name: "Integrations", exact: true })).toHaveCount(0);

    await page.goto(rulesUrl);
    await expect(page.getByText("Manager access required")).toBeVisible();
    await expect(page.getByRole("button", { name: /Connect|New rule/ })).toHaveCount(0);
  });

  test("global Settings no longer exposes ownerless connectors", async ({ page }) => {
    await login(page, demo.DEMO_MANAGER_EMAIL);
    await page.goto("/en/settings");

    await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Connectors" })).toHaveCount(0);
    await expect(page.getByRole("heading", { name: "GitHub" })).toHaveCount(0);
  });
});
