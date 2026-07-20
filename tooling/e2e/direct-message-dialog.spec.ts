import { expect, test, type APIRequestContext, type Page } from "@playwright/test";

const API_URL = process.env.OPSWARDEN_API_URL ?? "http://localhost:8080";
const TEAM_ID = "39aa8884-22cc-4764-a9e7-7df7c7619ba6";
const MEMBERS_URL = `/en/teams/${TEAM_ID}/members`;

async function login(page: Page, email: string) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password", { exact: true }).fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//);
  await page.goto(MEMBERS_URL);
}

function messageTrigger(page: Page, peerEmail: string) {
  return page
    .getByRole("listitem")
    .filter({ hasText: peerEmail })
    .getByRole("button", { name: "Message", exact: true });
}

async function signIn(request: APIRequestContext, email: string) {
  const response = await request.post(`${API_URL}/api/auth/sign-in`, {
    data: { email, password: "sudo" },
  });
  expect(response.ok()).toBe(true);
  return ((await response.json()) as { token: string }).token;
}

test("Direct Message owns focus, Escape and trigger restoration", async ({ page }) => {
  await login(page, "manager@opswarden.local");
  const trigger = messageTrigger(page, "responder@opswarden.local");
  await trigger.click();

  const dialog = page.getByRole("dialog", { name: "Direct message" });
  await expect(dialog).toHaveAccessibleDescription("responder@opswarden.local");
  await expect(dialog.getByPlaceholder("Write a message…")).toBeFocused();
  await page.keyboard.press("Escape");

  await expect(dialog).toHaveCount(0);
  await expect(trigger).toBeFocused();
});

test("Observer sends a real private message", async ({ page }) => {
  await login(page, "observer@opswarden.local");
  await messageTrigger(page, "manager@opswarden.local").click();
  const dialog = page.getByRole("dialog", { name: "Direct message" });
  const content = `E2E direct message send ${Date.now()}`;

  await dialog.getByPlaceholder("Write a message…").fill(content);
  await dialog.getByRole("button", { name: "Send", exact: true }).click();

  await expect(dialog.getByText(content, { exact: true })).toBeVisible();
  await expect(dialog.getByPlaceholder("Write a message…")).toHaveValue("");
});

test("only the open peer conversation announces a received message", async ({ page, request }) => {
  await login(page, "manager@opswarden.local");
  await messageTrigger(page, "responder@opswarden.local").click();
  const dialog = page.getByRole("dialog", { name: "Direct message" });

  const responderToken = await signIn(request, "responder@opswarden.local");
  const membersResponse = await request.get(`${API_URL}/api/teams/${TEAM_ID}/members`, {
    headers: { Authorization: `Bearer ${responderToken}` },
  });
  expect(membersResponse.ok()).toBe(true);
  const members = (await membersResponse.json()) as Array<{ user_id: string; email: string }>;
  const managerId = members.find((member) => member.email === "manager@opswarden.local")?.user_id;
  expect(managerId).toBeTruthy();
  const content = `E2E direct message receive ${Date.now()}`;

  const sendResponse = await request.post(`${API_URL}/api/private-messages`, {
    headers: { Authorization: `Bearer ${responderToken}` },
    data: { recipient_id: managerId, content },
  });
  expect(sendResponse.ok()).toBe(true);

  await expect(dialog.getByRole("status")).toHaveText(
    "New message from responder@opswarden.local.",
  );
  await expect(dialog.getByText(content, { exact: true })).toBeVisible();
});

test("send errors are announced and Escape remains available", async ({ page }) => {
  await page.route("**/api/private-messages", async (route) => {
    if (route.request().method() !== "POST") return route.continue();
    await route.fulfill({
      status: 500,
      contentType: "application/json",
      body: JSON.stringify({ code: "unexpected_error" }),
    });
  });
  await login(page, "responder@opswarden.local");
  const trigger = messageTrigger(page, "manager@opswarden.local");
  await trigger.click();
  const dialog = page.getByRole("dialog", { name: "Direct message" });

  await dialog.getByPlaceholder("Write a message…").fill("Rejected message");
  await dialog.getByRole("button", { name: "Send", exact: true }).click();
  await expect(dialog.getByRole("alert")).toHaveText("Failed to send the message");
  await page.keyboard.press("Escape");

  await expect(dialog).toHaveCount(0);
  await expect(trigger).toBeFocused();
});

test("the thread is the only scrolling region at 320 px", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 420 });
  await login(page, "manager@opswarden.local");
  await messageTrigger(page, "responder@opswarden.local").click();
  const dialog = page.getByRole("dialog", { name: "Direct message" });

  const geometry = await dialog.evaluate((element) => {
    const body = element.querySelector<HTMLElement>('[data-dialog-part="body"]')!;
    const footer = element.querySelector<HTMLElement>('[data-dialog-part="footer"]')!;
    const box = element.getBoundingClientRect();
    return {
      bodyOverflow: getComputedStyle(body).overflowY,
      bottom: box.bottom,
      footerBottom: footer.getBoundingClientRect().bottom,
      top: box.top,
    };
  });

  expect(geometry.bodyOverflow).toBe("auto");
  expect(geometry.top).toBeGreaterThanOrEqual(0);
  expect(geometry.bottom).toBeLessThanOrEqual(420);
  expect(geometry.footerBottom).toBeLessThanOrEqual(420);
  expect(
    await page.evaluate(() => document.documentElement.scrollWidth - innerWidth),
  ).toBeLessThanOrEqual(1);
});
