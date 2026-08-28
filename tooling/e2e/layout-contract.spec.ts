import { expect, test, type Page } from "@playwright/test";

const TEAM_ID = "39aa8884-22cc-4764-a9e7-7df7c7619ba6";
const RESPONDER_TEAM_ID = "6d1e8c20-b622-4d21-9b1b-111111111111";
const OBSERVER_TEAM_ID = "8b2f9d30-c733-4e32-8c2c-222222222222";
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
    path: `/en/teams/${TEAM_ID}/team#members`,
    kind: "collection",
  },
  {
    name: "team rules",
    path: `/en/teams/${TEAM_ID}/rules`,
    kind: "collection",
  },
  {
    name: "team runs",
    path: `/en/teams/${TEAM_ID}/runs`,
    kind: "collection",
  },
  {
    name: "team integrations",
    path: `/en/teams/${TEAM_ID}/integrations`,
    kind: "collection",
  },
  {
    name: "team settings",
    path: `/en/teams/${TEAM_ID}/team`,
    kind: "collection",
  },
  { name: "account settings", path: "/en/settings", kind: "collection" },
];

async function login(page: Page) {
  await page.goto("/en/login", { waitUntil: "domcontentloaded" });
  await page.getByLabel("Email").fill("manager@opswarden.local");
  await page.getByLabel("Password", { exact: true }).fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//, { timeout: 15_000 });
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
    if (
      message.type() === "error" &&
      (message.text().includes("MISSING_MESSAGE") || message.text().includes("FORMATTING_ERROR"))
    ) {
      missingMessages.push(message.text());
    }
  });
  page.on("pageerror", (error) => {
    if (error.message.includes("MISSING_MESSAGE") || error.message.includes("FORMATTING_ERROR")) {
      missingMessages.push(error.message);
    }
  });

  await login(page);
  try {
    for (const locale of ["en", "fr"] as const) {
      if (locale === "fr") {
        await page.goto("/en/settings");
        await page.getByRole("button", { name: "French", exact: true }).click();
        await expect(page).toHaveURL(/\/fr\/settings$/);
      }

      for (const route of routes) {
        await test.step(`${route.name} has complete ${locale.toUpperCase()} messages`, async () => {
          await page.goto(route.path.replace(/^\/en/, `/${locale}`));
          await expect(page).toHaveURL(new RegExp(`/${locale}(?:/|$)`));
          await expect(page.locator("html")).toHaveAttribute("lang", locale);
          await expect(page.locator('[data-page-layout="true"]')).toBeVisible();
        });
      }
    }
  } finally {
    if (new URL(page.url()).pathname.startsWith("/fr/")) {
      await page.goto("/fr/settings");
      await page.getByRole("button", { name: "Anglais", exact: true }).click();
      await expect(page).toHaveURL(/\/en\/settings$/);
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
    {
      path: `/en/teams/${TEAM_ID}/team#members`,
      current: "Team",
      mobile: "More",
      mobileDestination: "Team settings",
    },
    { path: "/en/teams", current: "Workspace", mobile: "Team directory" },
    { path: "/en/settings", current: "Account", mobile: "More", mobileDestination: "Account" },
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
        if (viewportWidth >= 768 && navigationCase.path === "/en/teams") {
          // Workspace deliberately has no sidebar destination: Teams are selected
          // from the breadcrumb in Team scope and the global directory stands alone.
          await expect(currentItem).toHaveCount(0);
          return;
        }
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
            sheet.getByRole("link", {
              name: navigationCase.mobileDestination ?? navigationCase.current,
              exact: true,
            }),
          ).toHaveAttribute("aria-current", "page");
          await page.keyboard.press("Escape");
        }
      });
    }
  }
});

test("desktop shell exposes Operations, Settings and one current destination", async ({ page }) => {
  await login(page);

  for (const viewport of [
    { width: 1280, height: 900 },
    { width: 1920, height: 1080 },
  ]) {
    await page.setViewportSize(viewport);
    await page.goto(`/en/teams/${TEAM_ID}/overview`);

    const navigation = page.getByRole("navigation", { name: "Primary navigation" });
    await expect(navigation.getByRole("heading")).toHaveText(["Operations", "Settings"]);
    await expect(navigation.getByRole("link", { name: "Rules" })).toBeVisible();
    await expect(navigation.getByRole("link", { name: "Integrations" })).toBeVisible();
    await expect(navigation.getByRole("link", { name: /^Members/ })).toHaveCount(0);
    await expect(navigation.getByRole("link", { name: "Team", exact: true })).toBeVisible();
    await expect(page.getByRole("link", { name: "Workspace", exact: true })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Current team: OpsWarden Demo" })).toBeVisible();
    const accountTrigger = page.getByRole("button", { name: "Account", exact: true });
    await expect(accountTrigger).toBeVisible();
    await expect(page.getByRole("navigation", { name: "Breadcrumb" })).toContainText(
      "OpsWarden Demo/Overview",
    );

    await accountTrigger.click();
    const accountDialog = page.getByRole("dialog", { name: "Account" });
    await expect(accountDialog).toBeVisible();
    await expect(accountDialog.getByRole("heading", { name: "Profile" })).toBeVisible();
    await expect(accountDialog.getByRole("heading", { name: "Preferences" })).toBeVisible();
    await expect(accountDialog.getByRole("heading", { name: "Account actions" })).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(accountDialog).toBeHidden();

    const sidebarBox = await page.locator("aside").boundingBox();
    const mainBox = await page.locator("main").boundingBox();
    const activeNavigationBox = await navigation
      .getByRole("link", { name: "Overview", exact: true })
      .boundingBox();
    expect(sidebarBox).not.toBeNull();
    expect(mainBox).not.toBeNull();
    expect(activeNavigationBox).not.toBeNull();
    expect(Math.round(sidebarBox!.width)).toBe(256);
    expect(Math.round(mainBox!.x)).toBe(256);
    expect(Math.round(activeNavigationBox!.height)).toBe(44);

    const logo = page.getByRole("link", { name: "OpsWarden", exact: true });
    await expect(logo).toHaveAttribute("href", `/en/teams/${TEAM_ID}/overview`);
    const logoBox = await logo.boundingBox();
    expect(logoBox).not.toBeNull();
    expect(Math.round(logoBox!.height)).toBe(64);

    const allCurrentItems = page.locator('[data-app-navigation-item="true"][aria-current="page"]');
    await expect(allCurrentItems).toHaveCount(2);
    await expect(allCurrentItems).toHaveText(["Overview", "Overview"]);
    await expect(
      page.locator('[data-app-navigation-item="true"]:visible[aria-current="page"]'),
    ).toHaveCount(1);
  }
});

test("desktop collections share one strict content origin and header height", async ({ page }) => {
  await login(page);

  for (const viewport of [
    { width: 1280, height: 900 },
    { width: 1920, height: 1080 },
  ]) {
    await page.setViewportSize(viewport);

    for (const destination of ["incidents", "releases", "runs", "rules"]) {
      await page.goto(`/en/teams/${TEAM_ID}/${destination}`);
      const content = page.locator('section[data-state="ready"]');
      const surface = content.locator(".surface").first();
      await expect(surface).toBeVisible();

      const contentBox = await content.boundingBox();
      const surfaceBox = await surface.boundingBox();
      expect(contentBox).not.toBeNull();
      expect(surfaceBox).not.toBeNull();
      expect(Math.round(surfaceBox!.x)).toBe(Math.round(contentBox!.x));
      expect(Math.round(surfaceBox!.y)).toBe(Math.round(contentBox!.y));

      const tableHead = surface.locator("thead");
      if ((await tableHead.count()) > 0) {
        const headBox = await tableHead.boundingBox();
        expect(headBox).not.toBeNull();
        expect(Math.round(headBox!.height)).toBe(41);
      }
    }
  }
});

test("Team scope menu preserves legitimate Manager, Responder and Observer navigation", async ({
  page,
}) => {
  await login(page);
  await page.setViewportSize({ width: 1280, height: 900 });

  for (const team of [
    { id: TEAM_ID, name: "OpsWarden Demo", settings: ["Team", "Rules", "Integrations"] },
    { id: RESPONDER_TEAM_ID, name: "Production Europe", settings: ["Team"] },
    { id: OBSERVER_TEAM_ID, name: "Security Lab", settings: ["Team"] },
  ]) {
    await page.goto(`/en/teams/${team.id}/overview`);
    const navigation = page.getByRole("navigation", { name: "Primary navigation" });
    for (const label of ["Overview", "Incidents", "Releases", ...team.settings]) {
      await expect(
        navigation.getByRole("link", { name: new RegExp(`^${label}(?: \\d+)?$`) }),
      ).toBeVisible();
    }
    await expect(navigation.getByRole("link", { name: "Rules" })).toHaveCount(
      team.settings.includes("Rules") ? 1 : 0,
    );
    await expect(navigation.getByRole("link", { name: "Integrations" })).toHaveCount(
      team.settings.includes("Integrations") ? 1 : 0,
    );

    const scope = page.getByRole("button", { name: `Current team: ${team.name}` });
    await expect(scope).toBeVisible();
    await scope.click();
    await expect(page.getByRole("menuitemcheckbox", { name: team.name })).toBeChecked();
    await expect(page.getByRole("menuitem", { name: "All teams" })).toHaveAttribute(
      "href",
      "/en/teams",
    );
    await page.keyboard.press("Escape");
  }

  // Test switching teams from a canonical queue rather than the War Room
  // since the War Room deliberately hides the global team switcher.
  await page.goto(`/en/teams/${TEAM_ID}/incidents`);
  await page.getByRole("button", { name: "Current team: OpsWarden Demo" }).click();
  await page.getByRole("menuitemcheckbox", { name: "Production Europe" }).click();
  await expect(page).toHaveURL(`/en/teams/${RESPONDER_TEAM_ID}/incidents`);
});

test("breadcrumb and page actions share one strict desktop rail", async ({ page }) => {
  test.setTimeout(60_000);
  await login(page);

  for (const viewport of [
    { width: 1280, height: 900 },
    { width: 1920, height: 1080 },
  ]) {
    await page.setViewportSize(viewport);
    await page.goto(`/en/teams/${TEAM_ID}/incidents`);

    const breadcrumbBox = await page.getByRole("navigation", { name: "Breadcrumb" }).boundingBox();
    const actionBox = await page
      .getByRole("button", { name: "New incident", exact: true })
      .boundingBox();
    const layoutBox = await page.locator('[data-page-layout="true"]').boundingBox();

    expect(breadcrumbBox).not.toBeNull();
    expect(actionBox).not.toBeNull();
    expect(layoutBox).not.toBeNull();
    expect(Math.round(breadcrumbBox!.y + breadcrumbBox!.height / 2)).toBe(
      Math.round(actionBox!.y + actionBox!.height / 2),
    );
    expect(Math.round(actionBox!.x + actionBox!.width)).toBe(
      Math.round(layoutBox!.x + layoutBox!.width - 32),
    );
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

test("legacy Team routes resolve to canonical resources", async ({ page }) => {
  await login(page);

  await page.goto(`/en/teams/${TEAM_ID}/members`);
  await expect(page).toHaveURL(`/en/teams/${TEAM_ID}/team#members`);
  await expect(page.getByRole("navigation", { name: "Breadcrumb" })).toContainText(
    "OpsWarden Demo/Team",
  );
  await expect(page.locator("#members")).toBeVisible();

  for (const legacy of [
    { path: "settings", destination: "team" },
    { path: "automations", destination: "rules" },
    { path: "automations?view=connections", destination: "integrations" },
    { path: "activity", destination: "runs" },
  ]) {
    await page.goto(`/en/teams/${TEAM_ID}/${legacy.path}`);
    await expect(page).toHaveURL(`/en/teams/${TEAM_ID}/${legacy.destination}`);
  }
});

test("canonical pages keep one horizontal and vertical layout contract", async ({ page }) => {
  test.setTimeout(120_000);
  await login(page);

  for (const viewportWidth of [320, 768, 1280, 1920]) {
    await page.setViewportSize({ width: viewportWidth, height: 900 });
    let sharedLayout: { x: number; width: number } | null = null;

    for (const route of routes) {
      await test.step(`${route.name} at ${viewportWidth}px`, async () => {
        await page.goto(route.path);

        const layout = page.locator('[data-page-layout="true"]');
        const heading = page.getByRole("heading", { level: 1 });
        const breadcrumb = page.getByRole("navigation", { name: "Breadcrumb" });
        const isTeamRoute = route.path.includes(`/teams/${TEAM_ID}/`);
        const isIncidentRoom = route.name === "incident detail";
        await expect(layout).toHaveAttribute("data-page-width", "workspace");
        await expect(heading).toHaveCount(1);

        const layoutBox = await layout.boundingBox();
        expect(layoutBox).not.toBeNull();
        const currentLayout = {
          x: Math.round(layoutBox!.x),
          width: Math.round(layoutBox!.width),
        };
        if (isIncidentRoom) {
          await expect(breadcrumb).toHaveCount(0);
        } else if (sharedLayout) {
          expect(currentLayout, `${route.name} shared layout at ${viewportWidth}px`).toEqual(
            sharedLayout,
          );
        } else {
          sharedLayout = currentLayout;
        }
        if (isTeamRoute && !isIncidentRoom) {
          await expect(breadcrumb).toBeVisible();
          const headingBox = await heading.boundingBox();
          const firstCrumbBox = await breadcrumb
            .getByRole("button", { name: /^Current team:/ })
            .boundingBox();
          expect(firstCrumbBox).not.toBeNull();
          const expectedPadding = viewportWidth < 640 ? 16 : viewportWidth < 768 ? 24 : 32;
          expect(
            Math.round(firstCrumbBox!.x - layoutBox!.x),
            `${route.name} breadcrumb alignment at ${viewportWidth}px`,
          ).toBe(expectedPadding);
          if (route.kind === "detail") {
            await expect(heading).toBeVisible();
            expect(headingBox).not.toBeNull();
            expect(firstCrumbBox!.y).toBeLessThan(headingBox!.y);
          }
        } else {
          await expect(breadcrumb).toHaveCount(0);
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
