import {
  expect,
  test,
  type APIRequestContext,
  type Browser,
  type BrowserContext,
  type Page,
} from "@playwright/test";

const API_URL = process.env.OPSWARDEN_API_URL ?? "http://localhost:8080";
const TEAM_ID = "39aa8884-22cc-4764-a9e7-7df7c7619ba6";
const MEMBERS_URL = `/en/teams/${TEAM_ID}/team#members`;

async function login(page: Page, email: string) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password", { exact: true }).fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//);
  await page.goto(MEMBERS_URL);
}

async function openConversation(page: Page, peerEmail: string) {
  await page
    .getByRole("listitem")
    .filter({ hasText: peerEmail })
    .getByRole("link", { name: `Chat with ${peerEmail}` })
    .click();
  await expect(page).toHaveURL(new RegExp(`/messages/[0-9a-f-]+$`));
  return page.getByRole("region", { name: peerEmail });
}

async function signIn(request: APIRequestContext, email: string) {
  const response = await request.post(`${API_URL}/api/auth/sign-in`, {
    data: { email, password: "sudo" },
  });
  expect(response.ok()).toBe(true);
  return ((await response.json()) as { token: string }).token;
}

async function openMemberConversationInContext(
  browser: Browser,
  email: string,
  peerEmail: string,
): Promise<{ context: BrowserContext; page: Page }> {
  const context = await browser.newContext();
  const page = await context.newPage();
  await login(page, email);
  await openConversation(page, peerEmail);
  return { context, page };
}

test("a Team member opens a full routed conversation", async ({ page }) => {
  await login(page, "manager@opswarden.local");
  const room = await openConversation(page, "responder@opswarden.local");

  await expect(room).toBeVisible();
  await expect(page.getByRole("dialog", { name: "Rooms" })).toHaveCount(0);
  await expect(page.getByRole("dialog", { name: "Members" })).toHaveCount(0);
  await expect(page.getByPlaceholder("Write a message…")).toBeVisible();
  await expect(page.getByRole("complementary", { name: "War room navigation" })).toBeVisible();
  await expect(page.getByRole("complementary", { name: "Members" })).toBeVisible();
});

test("Observer sends a real private message", async ({ page }) => {
  await login(page, "observer@opswarden.local");
  const room = await openConversation(page, "manager@opswarden.local");
  const content = `E2E direct message send ${Date.now()}`;

  await room.getByPlaceholder("Write a message…").fill(content);
  await room.getByRole("button", { name: "Send", exact: true }).click();

  await expect(room.getByText(content, { exact: true })).toBeVisible();
  await expect(room.getByPlaceholder("Write a message…")).toHaveValue("");
});

test("a direct conversation sends and renders a GIF", async ({ page }) => {
  const gifUrl = "https://media.giphy.com/media/opswarden-e2e/giphy.gif";
  await page.route("**/api/giphy/search?**", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify([
        {
          id: "gif-e2e",
          title: "Deployment dance",
          url: gifUrl,
          preview_url: gifUrl,
          width: 320,
          height: 180,
        },
      ]),
    });
  });
  await login(page, "manager@opswarden.local");
  const room = await openConversation(page, "responder@opswarden.local");

  await room.getByRole("button", { name: "Search GIFs" }).click();
  await room.getByPlaceholder("Search…").fill("deploy");
  await room.getByRole("button", { name: "Deployment dance" }).click();

  await expect(room.getByRole("img", { name: "GIF" }).last()).toHaveAttribute("src", gifUrl);
});

test("only the open peer conversation announces a received message", async ({ page, request }) => {
  await login(page, "manager@opswarden.local");
  const room = await openConversation(page, "responder@opswarden.local");

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

  await expect(room.getByRole("status")).toHaveText("New message from responder@opswarden.local.");
  await expect(room.getByText(content, { exact: true })).toBeVisible();
});

test("the transcript owns scrolling on a narrow viewport", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 420 });
  await login(page, "manager@opswarden.local");
  const room = await openConversation(page, "responder@opswarden.local");

  await expect(page.getByRole("button", { name: "Rooms" })).toBeVisible();
  const geometry = await room.evaluate((element) => {
    const transcript = element.querySelector<HTMLElement>(
      '[data-direct-message-transcript="true"]',
    )!;
    const box = element.getBoundingClientRect();
    return {
      roomBottom: box.bottom,
      roomTop: box.top,
      transcriptOverflow: getComputedStyle(transcript).overflowY,
    };
  });

  expect(geometry.transcriptOverflow).toBe("auto");
  expect(geometry.roomTop).toBeGreaterThanOrEqual(0);
  expect(geometry.roomBottom).toBeLessThanOrEqual(420);
  expect(
    await page.evaluate(() => document.documentElement.scrollWidth - innerWidth),
  ).toBeLessThanOrEqual(1);
});

test("a rich direct message supports a file and edit without reactions", async ({ page }) => {
  await login(page, "manager@opswarden.local");
  const room = await openConversation(page, "responder@opswarden.local");
  const original = `DM parity ${Date.now()}`;
  const edited = `${original} verified`;

  await page.locator('input[type="file"]').setInputFiles({
    name: "runbook.txt",
    mimeType: "text/plain",
    buffer: Buffer.from("restart the payment gateway"),
  });
  await room.getByPlaceholder("Write a message…").fill(original);
  await room.getByRole("button", { name: "Send", exact: true }).click();

  const item = room.getByRole("listitem").filter({ hasText: original }).last();
  await expect(item.getByText("runbook.txt")).toBeVisible();
  await item.hover();
  await expect(item.getByRole("button", { name: "Add reaction" })).toHaveCount(0);

  await item.getByRole("button", { name: "Edit" }).click();
  await item.getByRole("textbox", { name: "Edit message" }).fill(edited);
  await item.getByRole("button", { name: "Save" }).click();
  await expect(item.getByText(edited, { exact: true })).toBeVisible();

  const download = page.waitForEvent("download");
  await item.getByRole("button", { name: /runbook\.txt/i }).click();
  expect((await download).suggestedFilename()).toBe("runbook.txt");
});

test("presence and typing are live only inside the shared conversation", async ({ browser }) => {
  const manager = await openMemberConversationInContext(
    browser,
    "manager@opswarden.local",
    "responder@opswarden.local",
  );
  const responder = await openMemberConversationInContext(
    browser,
    "responder@opswarden.local",
    "manager@opswarden.local",
  );

  await expect(
    manager.page.getByLabel("Chat with responder@opswarden.local — Online"),
  ).toBeVisible();
  await responder.page.getByPlaceholder("Write a message…").fill("typing signal");
  await expect(manager.page.getByText("responder@opswarden.local typing…")).toBeVisible();

  await responder.context.close();
  await expect(
    manager.page.getByLabel("Chat with responder@opswarden.local — Offline"),
  ).toBeVisible();
  await manager.context.close();
});
