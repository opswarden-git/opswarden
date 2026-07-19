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
}

const routes: RouteContract[] = [
  { name: "teams directory", path: "/en/teams", width: "standard", kind: "collection" },
  {
    name: "incidents queue",
    path: `/en/teams/${TEAM_ID}/incidents`,
    width: "standard",
    kind: "collection",
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
          (viewportWidth < 768 ? 24 : 32) + (route.kind === "detail" ? 44 : 0);
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
