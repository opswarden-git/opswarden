import { expect, test, type Page } from "@playwright/test";

const TEAM_ID = "39aa8884-22cc-4764-a9e7-7df7c7619ba6";
const INCIDENT_ID = "10000000-0000-4000-8000-000000000001";
const RELEASE_ID = "30000000-0000-4000-8000-000000000001";

type PageKind = "collection" | "detail";

interface RouteContract {
  name: string;
  path: string;
  kind: PageKind;
}

const routes: RouteContract[] = [
  { name: "teams directory", path: "/en/teams", kind: "collection" },
  {
    name: "incidents queue",
    path: `/en/teams/${TEAM_ID}/incidents`,
    kind: "collection",
  },
  {
    name: "incident detail",
    path: `/en/teams/${TEAM_ID}/incidents/${INCIDENT_ID}`,
    kind: "detail",
  },
  {
    name: "releases queue",
    path: `/en/teams/${TEAM_ID}/releases`,
    kind: "collection",
  },
  {
    name: "release detail",
    path: `/en/teams/${TEAM_ID}/releases/${RELEASE_ID}`,
    kind: "detail",
  },
  {
    name: "team overview",
    path: `/en/teams/${TEAM_ID}/overview`,
    kind: "collection",
  },
  {
    name: "team members",
    path: `/en/teams/${TEAM_ID}/members`,
    kind: "collection",
  },
  {
    name: "team rules",
    path: `/en/teams/${TEAM_ID}/automations`,
    kind: "collection",
  },
  {
    name: "team integrations",
    path: `/en/teams/${TEAM_ID}/automations?view=connections`,
    kind: "collection",
  },
  {
    name: "team settings",
    path: `/en/teams/${TEAM_ID}/settings`,
    kind: "collection",
  },
  { name: "account settings", path: "/en/settings", kind: "collection" },
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

test("canonical English and French pages emit no missing-message errors", async ({ page }) => {
  test.setTimeout(90_000);
  const missingMessages: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error" && message.text().includes("MISSING_MESSAGE")) {
      missingMessages.push(message.text());
    }
  });
  page.on("pageerror", (error) => {
    if (error.message.includes("MISSING_MESSAGE")) missingMessages.push(error.message);
  });

  await login(page);
  for (const locale of ["en", "fr"]) {
    for (const route of routes) {
      await test.step(`${route.name} has complete ${locale.toUpperCase()} messages`, async () => {
        await page.goto(route.path.replace(/^\/en/, `/${locale}`));
        await expect(page.locator('[data-page-layout="true"]')).toBeVisible();
      });
    }
  }

  expect(missingMessages, missingMessages.join("\n")).toEqual([]);
});

test("desktop and mobile navigation expose one current product area", async ({ page }) => {
  test.setTimeout(90_000);
  await login(page);

  const navigationCases = [
    {
      path: `/en/teams/${TEAM_ID}/incidents/${INCIDENT_ID}`,
      current: "Incidents",
      mobile: "Incidents",
    },
    {
      path: `/en/teams/${TEAM_ID}/releases/${RELEASE_ID}`,
      current: "Releases",
      mobile: "Releases",
    },
    { path: `/en/teams/${TEAM_ID}/members`, current: "Members", mobile: "More" },
    { path: "/en/teams", current: "Team directory", mobile: "Team directory" },
    { path: "/en/settings", current: "Settings", mobile: "More" },
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
          '[data-app-navigation-item="true"]:visible[aria-current="page"]',
        );
        await expect(currentItem).toHaveCount(1);
        await expect(currentItem).toHaveAccessibleName(
          new RegExp(
            `^${viewportWidth < 768 ? navigationCase.mobile : navigationCase.current}(?: \\d+)?$`,
          ),
        );

        if (viewportWidth < 768 && navigationCase.mobile === "More") {
          await currentItem.click();
          const sheet = page.getByRole("dialog", { name: "More" });
          await expect(sheet).toBeVisible();
          await expect(
            sheet.getByRole("link", { name: navigationCase.current, exact: true }),
          ).toHaveAttribute("aria-current", "page");
          await page.keyboard.press("Escape");
        }
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
        const heading = page.getByRole("heading", { level: 1 });
        const breadcrumb = page.getByRole("navigation", { name: "Breadcrumb" });
        await expect(layout).toHaveAttribute("data-page-width", "workspace");
        await expect(heading).toHaveCount(1);
        await expect(heading).toBeVisible();
        await expect(breadcrumb).toBeVisible();

        const layoutBox = await layout.boundingBox();
        const headingBox = await heading.boundingBox();
        const firstCrumbBox = await breadcrumb.getByRole("link").first().boundingBox();
        expect(layoutBox).not.toBeNull();
        expect(headingBox).not.toBeNull();
        expect(firstCrumbBox).not.toBeNull();

        const expectedPadding = viewportWidth < 640 ? 16 : viewportWidth < 768 ? 24 : 32;
        expect(
          Math.round(firstCrumbBox!.x - layoutBox!.x),
          `${route.name} breadcrumb alignment at ${viewportWidth}px`,
        ).toBe(expectedPadding);
        if (route.kind === "detail") {
          expect(firstCrumbBox!.y).toBeLessThan(headingBox!.y);
        } else {
          await expect(breadcrumb.getByRole("heading", { level: 1 })).toBeVisible();
        }

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

test("detail breadcrumbs expose hierarchy and preserve list context", async ({ page }) => {
  await login(page);

  await page.goto(`/en/teams/${TEAM_ID}/incidents/${INCIDENT_ID}?view=escalated`);
  const breadcrumb = page.getByRole("navigation", { name: "Breadcrumb" });
  await expect(breadcrumb).toHaveCount(1);
  await expect(breadcrumb.getByRole("link")).toHaveCount(3);
  await expect(breadcrumb.getByRole("link", { name: "Incidents" })).toHaveAttribute(
    "href",
    `/en/teams/${TEAM_ID}/incidents?view=escalated`,
  );
  await expect(breadcrumb.getByRole("link").last()).toHaveAttribute("aria-current", "page");
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

  for (const path of [`/en/teams/${TEAM_ID}/incidents`, `/en/teams/${TEAM_ID}/releases`]) {
    await page.goto(path);
    const teamLink = page.getByRole("link", { name: "OpsWarden Demo" });
    await expect(teamLink).toBeVisible();
    await expect(teamLink).toHaveAttribute("href", `/en/teams/${TEAM_ID}/overview`);
  }
});

test("Incident details display as a bottom sheet on mobile", async ({ page }) => {
  await login(page);

  for (const viewportWidth of [320, 768, 1280, 1920]) {
    await page.setViewportSize({ width: viewportWidth, height: 900 });
    await page.goto(`/en/teams/${TEAM_ID}/incidents/${INCIDENT_ID}`);

    if (viewportWidth < 1024) {
      const contextButton = page.getByRole("button", { name: "Incident details" });
      await expect(contextButton).toBeVisible();

      // Open the sheet
      await contextButton.click();
      const dialog = page.getByRole("dialog", { name: "Incident details" });
      await expect(dialog).toBeVisible();

      await expect(
        dialog.getByRole("heading", { name: "Incident details", exact: true }),
      ).toBeVisible();

      // Close it
      await page.keyboard.press("Escape");
      await expect(dialog).toBeHidden();
    } else {
      await expect(page.getByRole("button", { name: "Incident details" })).toBeHidden();

      const contextPanel = page.getByRole("complementary", { name: "Incident details" });
      await expect(contextPanel).toBeVisible();
      await expect(
        contextPanel.getByRole("heading", { name: "Incident details", exact: true }),
      ).toBeVisible();
    }
  }
});
