import { writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, test, type APIRequestContext, type Page } from "@playwright/test";

const TEAM_ID = "39aa8884-22cc-4764-a9e7-7df7c7619ba6";
const SECRET = "e2e-alertmanager-secret";
const runtimeConfig = resolve(__dirname, "../../target/e2e-alertmanager/alertmanager.yml");

async function login(page: Page) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill("manager@opswarden.local");
  await page.getByLabel("Password", { exact: true }).fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//);
  return page.evaluate(() => {
    const raw = localStorage.getItem("opswarden-auth-storage");
    return raw ? (JSON.parse(raw).state.token as string) : "";
  });
}

async function clearAutomations(request: APIRequestContext, headers: Record<string, string>) {
  const rules = await request.get(`/api/teams/${TEAM_ID}/automation-rules`, { headers });
  for (const rule of rules.ok() ? await rules.json() : []) {
    await request.delete(`/api/teams/${TEAM_ID}/automation-rules/${rule.id}`, { headers });
  }
  const connections = await request.get(`/api/teams/${TEAM_ID}/service-connections`, { headers });
  for (const connection of connections.ok() ? await connections.json() : []) {
    await request.delete(`/api/teams/${TEAM_ID}/service-connections/${connection.id}`, { headers });
  }
}

async function createRule(
  request: APIRequestContext,
  headers: Record<string, string>,
  connectionId: string,
  triggerKind: string,
) {
  const created = await request.post(`/api/teams/${TEAM_ID}/automation-rules`, {
    headers,
    data: {
      name: `E2E ${triggerKind}`,
      trigger_connection_id: connectionId,
      trigger_kind: triggerKind,
      trigger_config: { severity: "critical", receiver: "opswarden" },
      reaction_kind: "create_incident",
      reaction_connection_id: null,
      reaction_config: {
        severity: "critical",
        title: `E2E ${triggerKind} {{alertname}}`,
      },
    },
  });
  expect(created.status()).toBe(201);
  const rule = await created.json();
  const enabled = await request.patch(`/api/teams/${TEAM_ID}/automation-rules/${rule.id}`, {
    headers,
    data: { enabled: true },
  });
  expect(enabled.ok()).toBeTruthy();
}

function configureAlertmanager(webhookPath: string) {
  writeFileSync(
    runtimeConfig,
    `route:
  receiver: opswarden
  group_by: [alertname]
  group_wait: 1s
  group_interval: 1s
  repeat_interval: 1h

receivers:
  - name: opswarden
    webhook_configs:
      - url: http://server:8080${webhookPath}
        send_resolved: true
        http_config:
          authorization:
            type: Bearer
            credentials: ${SECRET}
`,
  );
}

async function incidentTitles(request: APIRequestContext, headers: Record<string, string>) {
  const response = await request.get(`/api/incidents?team_id=${TEAM_ID}`, { headers });
  expect(response.ok()).toBeTruthy();
  const body = (await response.json()) as { items: Array<{ title: string }> };
  return body.items.map((incident) => incident.title);
}

test("real Alertmanager delivers firing and resolved lifecycle transitions", async ({ page }) => {
  const token = await login(page);
  const headers = { Authorization: `Bearer ${token}` };
  await clearAutomations(page.request, headers);

  try {
    const configured = await page.request.put(
      `/api/teams/${TEAM_ID}/service-connections/by-service/alertmanager`,
      { headers, data: { webhook_signing_secret: SECRET } },
    );
    expect(configured.ok()).toBeTruthy();
    const connection = await configured.json();

    await page.setViewportSize({ width: 320, height: 800 });
    await page.goto(`/en/teams/${TEAM_ID}/automations`);
    await page.getByRole("button", { name: "New rule" }).click();
    await page.getByLabel("Incoming event").selectOption("alert_firing");
    await page.getByLabel("Source connection").selectOption(connection.id);
    await expect(
      page.getByRole("status").filter({ hasText: "independent events" }),
    ).toBeVisible();
    await page.getByLabel("Incoming event").selectOption("alert_resolved");
    await expect(page.getByRole("option", { name: "Alert resolved" })).toBeAttached();
    await expect(
      page.getByRole("status").filter({ hasText: "does not automatically close" }),
    ).toBeVisible();
    await page.getByRole("button", { name: "Cancel" }).click();

    await createRule(page.request, headers, connection.id, "alert_firing");
    await createRule(page.request, headers, connection.id, "alert_resolved");

    configureAlertmanager(connection.webhook_path);
    const reload = await page.request.post("http://127.0.0.1:9093/-/reload");
    expect(reload.ok()).toBeTruthy();

    const startsAt = new Date().toISOString();
    const labels = {
      alertname: "RealAlertmanagerE2E",
      severity: "critical",
      instance: "e2e-api",
    };
    const firing = await page.request.post("http://127.0.0.1:9093/api/v2/alerts", {
      data: [
        {
          labels,
          annotations: { summary: "Real Alertmanager firing delivery" },
          startsAt,
          endsAt: new Date(Date.now() + 300_000).toISOString(),
          generatorURL: "https://prometheus.example/graph",
        },
      ],
    });
    expect(firing.ok()).toBeTruthy();
    await expect
      .poll(async () => incidentTitles(page.request, headers), { timeout: 20_000 })
      .toContain("E2E alert_firing RealAlertmanagerE2E");

    const resolved = await page.request.post("http://127.0.0.1:9093/api/v2/alerts", {
      data: [
        {
          labels,
          annotations: { summary: "Real Alertmanager resolved delivery" },
          startsAt,
          endsAt: new Date().toISOString(),
          generatorURL: "https://prometheus.example/graph",
        },
      ],
    });
    expect(resolved.ok()).toBeTruthy();
    await expect
      .poll(async () => incidentTitles(page.request, headers), { timeout: 20_000 })
      .toContain("E2E alert_resolved RealAlertmanagerE2E");

    const runs = await page.request.get(`/api/teams/${TEAM_ID}/automation-runs`, { headers });
    expect(runs.ok()).toBeTruthy();
    expect(await runs.json()).toHaveLength(2);
  } finally {
    await clearAutomations(page.request, headers);
  }
});
