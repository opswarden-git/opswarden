import { expect, test, type Page } from "@playwright/test";

const TEAM_ID = "39aa8884-22cc-4764-a9e7-7df7c7619ba6";
const automationsUrl = `/en/teams/${TEAM_ID}/automations`;

async function login(page: Page, email: string) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password", { exact: true }).fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//);
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
    test(`Manager can navigate Rules, Connections and Runs at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 800 });
      await login(page, "manager@opswarden.local");
      const token = await managerToken(page);
      await clearAutomations(page, token);

      try {
        await page.goto(automationsUrl);

        await expect(page.getByRole("heading", { name: "Automations" })).toBeVisible();
        await expect(page.getByRole("heading", { name: "No automation rules" })).toBeVisible();

        await page
          .getByRole("link", { name: /Connections/ })
          .last()
          .click();
        await expect(page).toHaveURL(`${automationsUrl}?view=connections`);
        const github = page
          .getByRole("heading", { name: "GitHub" })
          .locator("xpath=ancestor::section[1]");
        await expect(page.getByRole("heading", { name: "HTTP" })).toBeVisible();
        await expect(page.getByRole("button", { name: "Connect" })).toHaveCount(2);

        await github.getByRole("button", { name: "Connect" }).click();
        await page.getByLabel("Signing secret").fill("e2e-automation-secret");
        await page.getByRole("button", { name: "Save connection" }).click();
        await expect(github.getByRole("button", { name: "Copy webhook URL" })).toBeVisible();

        await page.getByRole("link", { name: /Rules/ }).last().click();
        await page.getByRole("button", { name: "New rule" }).click();
        await page.getByLabel("Rule name").fill("E2E failed CI to incident");
        await page.getByLabel("Source connection").selectOption({ index: 1 });
        await page.getByRole("button", { name: "Create rule" }).click();
        await expect(page.getByRole("dialog")).toBeHidden({ timeout: 5000 });

        const ruleContainer = width < 1024 ? page.locator("li") : page.locator("tr");
        const rule = ruleContainer.filter({ hasText: /E2E failed CI to incident/ });
        await expect(rule.getByText("Disabled")).toBeVisible();
        await rule.getByRole("button", { name: "Rule actions" }).click();
        await page.getByRole("menuitem", { name: "Enable" }).click();
        await expect(rule.getByText("Enabled")).toBeVisible();

        await page.getByRole("link", { name: /Runs/ }).click();
        await expect(page).toHaveURL(`${automationsUrl}?view=runs`);
        await expect(page.getByRole("heading", { name: "No automation runs" })).toBeVisible();
      } finally {
        await clearAutomations(page, token);
      }
    });
  }

  test("non-Managers do not receive configuration controls", async ({ page }) => {
    await login(page, "responder@opswarden.local");
    await page.goto(`/en/teams/${TEAM_ID}/overview`);
    await expect(page.getByRole("link", { name: "Automations", exact: true })).toHaveCount(0);

    await page.goto(automationsUrl);
    await expect(page.getByText("Manager access required")).toBeVisible();
    await expect(page.getByRole("button", { name: /Connect|New rule/ })).toHaveCount(0);
  });

  test("global Settings no longer exposes ownerless connectors", async ({ page }) => {
    await login(page, "manager@opswarden.local");
    await page.goto("/en/settings");

    await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Connectors" })).toHaveCount(0);
    await expect(page.getByRole("heading", { name: "GitHub" })).toHaveCount(0);
  });
});
