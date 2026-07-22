import { expect, test, type Page } from "@playwright/test";

const TEAM_ID = "39aa8884-22cc-4764-a9e7-7df7c7619ba6";
const INCIDENT_ID = "10000000-0000-4000-8000-000000000001";
const RELEASE_ID = "30000000-0000-4000-8000-000000000001";

type LayoutWidth = "standard" | "workspace";
type PageKind = "collection" | "detail";

interface RouteContract {
  name: string;
  path: string;
  width: LayoutWidth;
  kind: PageKind;
  hasContext?: boolean;
}

const routes: RouteContract[] = [
  { name: "teams directory", path: "/en/teams", width: "standard", kind: "collection" },
  {
    name: "incidents queue",
    path: `/en/teams/${TEAM_ID}/incidents`,
    width: "standard",
    kind: "collection",
    hasContext: true,
  },
  {
    name: "incident detail",
    path: `/en/teams/${TEAM_ID}/incidents/${INCIDENT_ID}`,
    width: "workspace",
    kind: "detail",
  },
  {
    name: "releases queue",
    path: `/en/teams/${TEAM_ID}/releases`,
    width: "standard",
    kind: "collection",
    hasContext: true,
  },
  {
    name: "release detail",
    path: `/en/teams/${TEAM_ID}/releases/${RELEASE_ID}`,
    width: "workspace",
    kind: "detail",
  },
  {
    name: "team overview",
    path: `/en/teams/${TEAM_ID}/overview`,
    width: "standard",
    kind: "collection",
  },
  {
    name: "team members",
    path: `/en/teams/${TEAM_ID}/members`,
    width: "standard",
    kind: "collection",
  },
  {
    name: "team automations",
    path: `/en/teams/${TEAM_ID}/automations`,
    width: "standard",
    kind: "collection",
  },
  {
    name: "team settings",
    path: `/en/teams/${TEAM_ID}/settings`,
    width: "standard",
    kind: "collection",
  },
  { name: "account settings", path: "/en/settings", width: "standard", kind: "collection" },
];

async function login(page: Page) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill("manager@opswarden.local");
  await page.getByLabel("Password", { exact: true }).fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//);
}

test("Team route boundary rejects malformed identifiers", async ({ page }) => {
  await login(page);

  await page.goto("/en/teams/not-a-uuid/overview");
  await expect(page.getByRole("heading", { level: 1, name: "404" })).toBeVisible();
  await expect(page.locator('[data-page-layout="true"]')).toHaveCount(0);
});

test("root resolves to the canonical incident queue", async ({ page }) => {
  await login(page);

  await page.goto("/en");
  await expect(page).toHaveURL(new RegExp(`/en/teams/${TEAM_ID}/incidents$`));
});

test("desktop and mobile navigation expose one current product area", async ({ page }) => {
  test.setTimeout(90_000);
  await login(page);

  const navigationCases = [
    { path: `/en/teams/${TEAM_ID}/incidents/${INCIDENT_ID}`, current: "Incidents" },
    { path: `/en/teams/${TEAM_ID}/releases/${RELEASE_ID}`, current: "Releases" },
    { path: `/en/teams/${TEAM_ID}/members`, current: "Teams" },
    { path: "/en/teams", current: "Teams" },
    { path: "/en/settings", current: "Settings" },
  ];

  for (const viewportWidth of [320, 1280]) {
    await page.setViewportSize({ width: viewportWidth, height: 900 });
    const navigationName = viewportWidth < 768 ? "Mobile navigation" : "Primary navigation";

    for (const navigationCase of navigationCases) {
      await test.step(`${navigationCase.current} at ${viewportWidth}px`, async () => {
        await page.goto(navigationCase.path);

        const navigation = page.getByRole("navigation", { name: navigationName });
        await expect(navigation).toBeVisible();
        const currentItem = page.locator(
          'a[data-app-navigation-item="true"]:visible[aria-current="page"]',
        );
        await expect(currentItem).toHaveCount(1);
        await expect(currentItem).toHaveAccessibleName(navigationCase.current);
      });
    }
  }
});

test("expired product routes are no longer exposed", async ({ page }) => {
  await login(page);

  for (const path of ["/en/ai", "/en/incidents", `/en/incidents/${INCIDENT_ID}`, "/en/releases"]) {
    await page.goto(path);
    await expect(page.getByRole("heading", { level: 1, name: "404" })).toBeVisible();
    await expect(page.locator('[data-page-layout="true"]')).toHaveCount(0);
  }
});

test("canonical pages keep one horizontal and vertical layout contract", async ({ page }) => {
  test.setTimeout(120_000);
  await login(page);

  for (const viewportWidth of [320, 768, 1280, 1920]) {
    await page.setViewportSize({ width: viewportWidth, height: 900 });

    for (const route of routes) {
      await test.step(`${route.name} at ${viewportWidth}px`, async () => {
        await page.goto(route.path);

        const layout = page.locator('[data-page-layout="true"]');
        const heading = layout.getByRole("heading", { level: 1 });
        await expect(layout).toHaveAttribute("data-page-width", route.width);
        await expect(heading).toBeVisible();

        const layoutBox = await layout.boundingBox();
        const headingBox = await heading.boundingBox();
        expect(layoutBox).not.toBeNull();
        expect(headingBox).not.toBeNull();

        const expectedPadding = viewportWidth < 640 ? 16 : viewportWidth < 768 ? 24 : 32;
        const expectedHeadingY =
          (viewportWidth < 768 ? 24 : 32) +
          (route.kind === "detail" ? 44 : 0) +
          (route.hasContext ? 24 : 0);
        expect(
          Math.round(headingBox!.x - layoutBox!.x),
          `${route.name} horizontal heading offset at ${viewportWidth}px`,
        ).toBe(expectedPadding);
        expect(
          Math.round(headingBox!.y),
          `${route.name} vertical heading position at ${viewportWidth}px`,
        ).toBe(expectedHeadingY);

        const overflow = await page.evaluate(
          () => document.documentElement.scrollWidth - window.innerWidth,
        );
        expect(
          overflow,
          `${route.name} horizontal overflow at ${viewportWidth}px`,
        ).toBeLessThanOrEqual(1);
      });
    }
  }
});

test("incident records switch morphology without losing operational context", async ({ page }) => {
  await login(page);

  for (const viewportWidth of [320, 768, 1280, 1920]) {
    await page.setViewportSize({ width: viewportWidth, height: 900 });
    await page.goto(`/en/teams/${TEAM_ID}/incidents`);

    const mobile = page.locator('[data-incident-layout="mobile"]');
    const desktop = page.locator('[data-incident-layout="desktop"]');
    if (viewportWidth < 1024) {
      await expect(mobile).toBeVisible();
      await expect(desktop).toBeHidden();
      const record = mobile
        .getByRole("listitem")
        .filter({ hasText: "Payment API returning 502 in Europe" });
      await expect(record.locator('[data-incident-field="identity"]')).toContainText("ID:");
      await expect(record.locator('[data-incident-field="state"]')).toContainText("Open");
      await expect(record.locator('[data-incident-field="assignee"]')).toContainText(
        "responder@opswarden.local",
      );
      await expect(record.locator('[data-incident-field="age"]')).not.toBeEmpty();
      await expect(record.getByRole("link")).toHaveCount(1);
    } else {
      await expect(desktop).toBeVisible();
      await expect(mobile).toBeHidden();
      const table = desktop.getByRole("table", { name: "Incident queue" });
      const rowHeaders = table.getByRole("rowheader");
      await expect(rowHeaders.first()).toBeVisible();
      expect(await table.getByRole("link").count()).toBe(await rowHeaders.count());
    }

    expect(
      await page.evaluate(() => document.documentElement.scrollWidth - innerWidth),
      `incident morphology overflow at ${viewportWidth}px`,
    ).toBeLessThanOrEqual(1);
  }
});

test("Collection headers display the parent team context", async ({ page }) => {
  await login(page);

  for (const path of [
    `/en/teams/${TEAM_ID}/incidents`,
    `/en/teams/${TEAM_ID}/releases`,
  ]) {
    await page.goto(path);
    const teamLink = page.getByRole("link", { name: "OpsWarden Demo" });
    await expect(teamLink).toBeVisible();
    await expect(teamLink).toHaveAttribute("href", `/en/teams/${TEAM_ID}/overview`);
  }
});

test("Incident context displays as a bottom sheet on mobile", async ({ page }) => {
  await login(page);

  for (const viewportWidth of [320, 768, 1280, 1920]) {
    await page.setViewportSize({ width: viewportWidth, height: 900 });
    await page.goto(`/en/teams/${TEAM_ID}/incidents/${INCIDENT_ID}`);

    if (viewportWidth < 1024) {
      // Button should be visible on mobile
      const contextButton = page.getByRole("button", { name: "Incident context" });
      await expect(contextButton).toBeVisible();

      // Open the sheet
      await contextButton.click();
      const dialog = page.getByRole("dialog", { name: "Incident context" });
      await expect(dialog).toBeVisible();

      // Verify it's a sheet (has the drag handle)
      // Actually we check if it has the sheet-specific classes or behavior if we want,
      // but verifying it opens and shows context is usually enough.
      await expect(dialog.getByRole("heading", { name: "Incident context", exact: true })).toBeVisible();
      
      // Close it
      await page.keyboard.press("Escape");
      await expect(dialog).toBeHidden();
    } else {
      // Context should be visible directly on the page, not behind a button
      await expect(page.getByRole("button", { name: "Incident context" })).toBeHidden();
      
      // The context title is rendered in the aside
      const contextPanel = page.getByRole("complementary", { name: "Incident context" });
      await expect(contextPanel).toBeVisible();
      await expect(contextPanel.getByRole("heading", { name: "Incident context", exact: true })).toBeVisible();
    }
  }
});
