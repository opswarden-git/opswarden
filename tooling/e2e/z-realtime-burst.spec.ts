import { expect, test, type Browser, type Page } from "@playwright/test";

const API_URL = process.env.OPSWARDEN_API_URL ?? "http://localhost:8080";
const TEAM_ID = "39aa8884-22cc-4764-a9e7-7df7c7619ba6";
const INCIDENT_IDS = [
  "10000000-0000-4000-8000-000000000001",
  "10000000-0000-4000-8000-000000000007",
  "10000000-0000-4000-8000-000000000008",
];
const INCIDENT_TITLES = [
  "Payment API returning 502 in Europe",
  "Customer export job stalled",
  "Elevated worker memory usage",
];

async function login(page: Page, email: string) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password", { exact: true }).fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//);
}

async function openTwoOperators(browser: Browser) {
  const managerContext = await browser.newContext();
  const responderContext = await browser.newContext();
  const manager = await managerContext.newPage();
  const responder = await responderContext.newPage();
  await Promise.all([
    login(manager, "manager@opswarden.local"),
    login(responder, "responder@opswarden.local"),
  ]);
  return { manager, responder };
}

async function startVisualMeasurement(page: Page) {
  await page.evaluate(() => {
    const target = document.querySelector("tbody");
    if (!target) throw new Error("Incident table body not found");

    const metrics = { layoutShift: 0, mutationBatches: 0 };
    const mutationObserver = new MutationObserver(() => {
      metrics.mutationBatches += 1;
    });
    mutationObserver.observe(target, { childList: true, subtree: true, characterData: true });

    const layoutObserver = new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        const shift = entry as PerformanceEntry & { hadRecentInput?: boolean; value?: number };
        if (!shift.hadRecentInput) metrics.layoutShift += shift.value ?? 0;
      }
    });
    layoutObserver.observe({ type: "layout-shift", buffered: true });

    Object.assign(window, { __r3QueueMetrics: metrics });
  });
}

async function readVisualMeasurement(page: Page) {
  return page.evaluate(() => {
    const metrics = (
      window as typeof window & {
        __r3QueueMetrics: { layoutShift: number; mutationBatches: number };
      }
    ).__r3QueueMetrics;
    return {
      ...metrics,
      horizontalOverflow: document.documentElement.scrollWidth - window.innerWidth,
    };
  });
}

test("two queues absorb a simultaneous WebSocket burst without visual agitation", async ({
  browser,
}) => {
  const { manager, responder } = await openTwoOperators(browser);

  try {
    const queueUrl = `/en/teams/${TEAM_ID}/incidents`;
    await Promise.all([manager.goto(queueUrl), responder.goto(queueUrl)]);
    await Promise.all(
      [manager, responder].flatMap((page) =>
        INCIDENT_TITLES.map((title) =>
          expect(page.getByText(title, { exact: true })).toBeVisible(),
        ),
      ),
    );

    const managerHeadingBefore = await manager
      .getByRole("heading", { name: "Incidents", exact: true })
      .boundingBox();
    const responderHeadingBefore = await responder
      .getByRole("heading", { name: "Incidents", exact: true })
      .boundingBox();
    expect(managerHeadingBefore).not.toBeNull();
    expect(responderHeadingBefore).not.toBeNull();

    let managerQueueRequests = 0;
    let responderQueueRequests = 0;
    manager.on("response", (response) => {
      if (response.url().includes("/api/incidents?") && response.request().method() === "GET") {
        managerQueueRequests += 1;
      }
    });
    responder.on("response", (response) => {
      if (response.url().includes("/api/incidents?") && response.request().method() === "GET") {
        responderQueueRequests += 1;
      }
    });
    await Promise.all([startVisualMeasurement(manager), startVisualMeasurement(responder)]);

    const token = await manager.evaluate(() => {
      const raw = localStorage.getItem("opswarden-auth-storage");
      return raw ? (JSON.parse(raw).state.token as string) : null;
    });
    expect(token).toBeTruthy();

    const responses = await Promise.all(
      INCIDENT_IDS.map((incidentId) =>
        manager.request.put(`${API_URL}/api/incidents/${incidentId}/status`, {
          headers: { Authorization: `Bearer ${token}`, "Content-Type": "application/json" },
          data: { status: "acknowledged" },
        }),
      ),
    );
    for (const response of responses) expect(response.ok()).toBe(true);

    await Promise.all(
      [manager, responder].flatMap((page) =>
        INCIDENT_TITLES.map((title) =>
          expect(page.getByText(title, { exact: true })).toHaveCount(0),
        ),
      ),
    );
    await manager.waitForTimeout(250);

    const [managerMetrics, responderMetrics] = await Promise.all([
      readVisualMeasurement(manager),
      readVisualMeasurement(responder),
    ]);
    const managerHeadingAfter = await manager
      .getByRole("heading", { name: "Incidents", exact: true })
      .boundingBox();
    const responderHeadingAfter = await responder
      .getByRole("heading", { name: "Incidents", exact: true })
      .boundingBox();

    expect(managerHeadingAfter?.x).toBe(managerHeadingBefore?.x);
    expect(managerHeadingAfter?.y).toBe(managerHeadingBefore?.y);
    expect(responderHeadingAfter?.x).toBe(responderHeadingBefore?.x);
    expect(responderHeadingAfter?.y).toBe(responderHeadingBefore?.y);
    for (const metrics of [managerMetrics, responderMetrics]) {
      expect(metrics.horizontalOverflow).toBeLessThanOrEqual(1);
      expect(metrics.layoutShift).toBeLessThan(0.1);
      expect(metrics.mutationBatches).toBeLessThanOrEqual(3);
    }
    expect(managerQueueRequests).toBeLessThanOrEqual(3);
    expect(responderQueueRequests).toBeLessThanOrEqual(3);

    console.log(
      "R3 WebSocket burst measurement",
      JSON.stringify({
        manager: { ...managerMetrics, queueRequests: managerQueueRequests },
        responder: { ...responderMetrics, queueRequests: responderQueueRequests },
      }),
    );
  } finally {
    await manager.context().close();
    await responder.context().close();
  }
});
