import { expect, test, type Browser, type Page } from "@playwright/test";

const TEAM_ID = "50000000-0000-4000-8000-000000000001";
const LINKED_INCIDENT_ID = "51000000-0000-4000-8000-000000000001";
const OPEN_INCIDENT_ID = "51000000-0000-4000-8000-000000000004";
const UNASSIGNED_INCIDENT_ID = "51000000-0000-4000-8000-000000000007";
const LINKED_RELEASE_ID = "54000000-0000-4000-8000-000000000001";

const incidentUrl = (incidentId: string) => `/en/teams/${TEAM_ID}/incidents/${incidentId}`;

async function login(page: Page, email: string) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password", { exact: true }).fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//);
}

test.describe("Incident detail", () => {
  test("Responder can acknowledge and write an operational note", async ({ page }) => {
    await login(page, "responder@opswarden.local");
    await page.goto(incidentUrl(OPEN_INCIDENT_ID));

    await expect(page.locator('[data-system-event="created"]')).toBeVisible();
    await page.getByRole("button", { name: /^Actions/ }).click();
    await page.getByRole("button", { name: "Acknowledge", exact: true }).click();
    await expect(page.getByText("Acknowledged", { exact: true }).first()).toBeVisible();
    await expect(page.getByRole("button", { name: "Escalate", exact: true })).toBeVisible();
    await expect(page.locator('[data-system-event="status_changed"]')).toContainText(
      "changed status",
    );

    const note = `E2E operational update ${Date.now()}`;
    await page.getByLabel("Add a note").fill(note);
    await page.getByRole("button", { name: "Send note" }).click();
    await expect(page.getByText(note, { exact: true })).toBeVisible();

    await page.locator('input[type="file"]').setInputFiles({
      name: "incident-runbook.txt",
      mimeType: "text/plain",
      buffer: Buffer.from("restart the failed worker"),
    });
    await page.getByRole("button", { name: "Send note" }).click();
    const attachment = page.getByRole("button", { name: /incident-runbook\.txt/i }).last();
    await expect(attachment).toBeVisible();
    const download = page.waitForEvent("download");
    await attachment.click();
    expect((await download).suggestedFilename()).toBe("incident-runbook.txt");
  });

  test("Observer sees context without false commands", async ({ page }) => {
    await login(page, "observer@opswarden.local");
    await page.goto(incidentUrl(LINKED_INCIDENT_ID));

    await expect(page.getByRole("region", { name: "War room conversation" })).toBeVisible();
    await expect(page.locator("[data-system-event]")).toHaveCount(5);
    await expect(page.locator('[data-system-event="status_changed"]')).toHaveCount(2);
    await expect(page.getByRole("complementary", { name: "Incident details" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Activity" })).toHaveCount(0);
    await expect(page.getByLabel("Add a note")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Acknowledge", exact: true })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "More incident actions" })).toHaveCount(0);
    await expect(page.getByText("Change assignee", { exact: true })).toHaveCount(0);
  });

  test("War room separates incident context from actionable Team presence", async ({ page }) => {
    await login(page, "manager@opswarden.local");
    await page.setViewportSize({ width: 1280, height: 900 });
    await page.goto(incidentUrl(LINKED_INCIDENT_ID));

    const rooms = page.getByRole("complementary", { name: "War room navigation" });
    await expect(rooms.getByRole("link", { name: /^Incidents\s*\(\d+\)$/ })).toBeVisible();
    await expect(rooms.getByRole("link", { name: /Releases/ })).toHaveCount(0);
    const sectionLabels = await rooms
      .locator("section")
      .evaluateAll((sections) =>
        sections.map((section) => section.querySelector("h2, a")?.textContent?.trim()),
      );
    expect(sectionLabels).toHaveLength(1);
    expect(sectionLabels[0]).toMatch(/^Incidents\s*\(\d+\)$/);

    const context = page.getByRole("complementary", { name: "Incident details" });
    for (const section of [/^Details$/, /^Actions/, /^Assignee/, /^Release$/]) {
      await expect(context.getByRole("button", { name: section })).toHaveAttribute(
        "aria-expanded",
        "false",
      );
    }
    await expect(context.getByRole("button", { name: "Members" })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    await context.getByRole("link", { name: "Chat with responder@opswarden.local" }).click();
    await expect(page).toHaveURL(new RegExp(`/messages/[0-9a-f-]+$`));
    await expect(page.getByRole("region", { name: "responder@opswarden.local" })).toBeVisible();
    await expect(page.getByRole("complementary", { name: "Members" })).toBeVisible();
  });

  test("Manager can assign, inspect delete safely, and follow the linked Release", async ({
    page,
  }) => {
    await login(page, "manager@opswarden.local");
    await page.goto(incidentUrl(UNASSIGNED_INCIDENT_ID));

    await page.getByRole("button", { name: /^Assignee/ }).click();
    await page.getByLabel("Change assignee").selectOption({ label: "responder@opswarden.local" });
    await page.getByRole("button", { name: "Assign", exact: true }).click();
    await expect(page.getByLabel("Change assignee").locator("option:checked")).toHaveText(
      "responder@opswarden.local",
    );

    await page.getByRole("button", { name: /^Actions/ }).click();
    await page.getByRole("button", { name: "Delete incident" }).click();
    await expect(page.getByRole("dialog", { name: "Delete incident" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Cancel" })).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog", { name: "Delete incident" })).toHaveCount(0);

    await page.goto(incidentUrl(LINKED_INCIDENT_ID));
    await page.getByRole("button", { name: "Release" }).click();
    await page
      .locator('aside[data-war-room-context="true"]')
      .getByRole("link", { name: /v2\.8\.0 — Payment resilience/ })
      .click();
    await expect(page).toHaveURL(new RegExp(`/releases/${LINKED_RELEASE_ID}$`));
    await expect(page.getByRole("heading", { name: "v2.8.0 — Payment resilience" })).toBeVisible();
  });

  test("layout stays ordered and has no horizontal overflow", async ({ page }) => {
    await login(page, "manager@opswarden.local");

    for (const width of [320, 768, 1280, 1920]) {
      await page.setViewportSize({ width, height: 900 });
      await page.goto(incidentUrl(LINKED_INCIDENT_ID));
      await expect(page.getByRole("region", { name: "War room conversation" })).toBeVisible();

      const overflow = await page.evaluate(
        () => document.documentElement.scrollWidth - window.innerWidth,
      );
      expect(overflow, `horizontal overflow at ${width}px`).toBeLessThanOrEqual(1);

      const activity = await page.locator('[data-incident-room="true"]').boundingBox();
      expect(activity).not.toBeNull();

      // Below lg the context is an on-demand sheet rather than a stacked panel.
      // The room keeps a fixed frame (D9), so a panel placed under a scrolling
      // transcript would sit behind the entire conversation.
      const contextTrigger = page.getByRole("button", { name: "Incident details", exact: true });
      const contextPanel = page.locator('aside[data-war-room-context="true"]');

      if (width < 1024) {
        await expect(contextTrigger).toBeVisible();
        await expect(contextPanel).toBeHidden();
      } else {
        await expect(contextTrigger).toBeHidden();
        const context = await contextPanel.boundingBox();
        expect(context).not.toBeNull();
        expect(context!.x).toBeGreaterThan(activity!.x);
      }
    }
  });

  test("context opens as a sheet on narrow viewports", async ({ page }) => {
    await login(page, "manager@opswarden.local");
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(incidentUrl(LINKED_INCIDENT_ID));

    await page.getByRole("button", { name: "Rooms" }).click();
    await expect(page.getByRole("dialog", { name: "War room" })).toBeVisible();
    await page.keyboard.press("Escape");

    await page.getByRole("button", { name: "Details" }).click();
    await expect(page.getByRole("dialog", { name: "Incident details" })).toBeVisible();
  });
});

test("two clients see identified incident watchers", async ({ browser }) => {
  const { manager, responder } = await openTwoOperators(browser);
  await manager.goto(incidentUrl(LINKED_INCIDENT_ID));
  await responder.goto(incidentUrl(LINKED_INCIDENT_ID));

  const managerContext = manager.locator('aside[data-war-room-context="true"]');
  const responderContext = responder.locator('aside[data-war-room-context="true"]');
  for (const context of [managerContext, responderContext]) {
    const members = context.getByRole("region", { name: "Members" });
    await expect(
      members.locator('[aria-label*="manager@opswarden.local"][aria-label$="Online"]'),
    ).toBeVisible();
    await expect(
      members.locator('[aria-label*="responder@opswarden.local"][aria-label$="Online"]'),
    ).toBeVisible();
  }

  const responderRoom = responder.locator('[data-incident-room="true"]');
  const roomBounds = await responderRoom.boundingBox();
  expect(roomBounds).not.toBeNull();
  await responder.mouse.move(
    roomBounds!.x + roomBounds!.width * 0.42,
    roomBounds!.y + roomBounds!.height * 0.38,
  );
  await expect(
    manager.locator("[data-collaborator-cursor]").filter({ hasText: "responder" }),
  ).toBeVisible();

  await manager.context().close();
  await responder.context().close();
});

async function openTwoOperators(browser: Browser) {
  const managerContext = await browser.newContext();
  const responderContext = await browser.newContext();
  const manager = await managerContext.newPage();
  const responder = await responderContext.newPage();
  await login(manager, "manager@opswarden.local");
  await login(responder, "responder@opswarden.local");
  return { manager, responder };
}
