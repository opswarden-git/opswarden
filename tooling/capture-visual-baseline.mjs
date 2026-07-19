import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { chromium } from "@playwright/test";

const baseUrl = process.env.OPSWARDEN_E2E_URL ?? "http://localhost:8081";
const chromePath = process.env.PLAYWRIGHT_CHROME_PATH ?? "/run/current-system/sw/bin/google-chrome";
const outputRoot = resolve(process.cwd(), "../.other/audit/baselines");
const screenshotRoot = resolve(outputRoot, "screenshots");
const teamId = "39aa8884-22cc-4764-a9e7-7df7c7619ba6";

const routes = [
  { name: "teams-directory", path: "/en/teams" },
  { name: "incidents-queue", path: `/en/teams/${teamId}/incidents` },
  {
    name: "incident-detail",
    path: `/en/teams/${teamId}/incidents/10000000-0000-4000-8000-000000000001`,
  },
  { name: "releases-queue", path: `/en/teams/${teamId}/releases` },
  {
    name: "release-detail",
    path: `/en/teams/${teamId}/releases/30000000-0000-4000-8000-000000000001`,
  },
  { name: "team-overview", path: `/en/teams/${teamId}/overview` },
  { name: "team-members", path: `/en/teams/${teamId}/members` },
  { name: "team-automations", path: `/en/teams/${teamId}/automations` },
  { name: "team-settings", path: `/en/teams/${teamId}/settings` },
  { name: "account-settings", path: "/en/settings" },
];
const widths = [320, 768, 1280, 1920];

await mkdir(screenshotRoot, { recursive: true });
const browser = await chromium.launch({ executablePath: chromePath, headless: true });

try {
  const authContext = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const authPage = await authContext.newPage();
  await authPage.goto(`${baseUrl}/en/login`);
  await authPage.getByLabel("Email").fill("manager@opswarden.local");
  await authPage.getByLabel("Password", { exact: true }).fill("sudo");
  await authPage.getByRole("button", { name: "Log in", exact: true }).click();
  await authPage.waitForURL(/\/en\/teams\//);
  const storageState = await authContext.storageState();
  await authContext.close();

  const measurements = [];
  for (const width of widths) {
    const context = await browser.newContext({
      viewport: { width, height: 900 },
      storageState,
      reducedMotion: "reduce",
    });
    const page = await context.newPage();

    for (const route of routes) {
      await page.goto(`${baseUrl}${route.path}`, { waitUntil: "networkidle" });
      const layout = page.locator('[data-page-layout="true"]');
      await layout.waitFor({ state: "visible" });
      await page.addStyleTag({
        content: "*, *::before, *::after { caret-color: transparent !important; }",
      });

      const box = await layout.boundingBox();
      const headingBox = await page.getByRole("heading", { level: 1 }).boundingBox();
      const overflow = await page.evaluate(
        () => document.documentElement.scrollWidth - window.innerWidth,
      );
      const widthToken = await layout.getAttribute("data-page-width");
      if (!box || !headingBox || !widthToken) {
        throw new Error(`Unable to measure ${route.name} at ${width}px`);
      }

      measurements.push({
        route: route.path,
        name: route.name,
        viewportWidth: width,
        widthToken,
        layout: {
          x: Math.round(box.x),
          width: Math.round(box.width),
        },
        heading: {
          x: Math.round(headingBox.x),
          y: Math.round(headingBox.y),
        },
        horizontalOverflow: Math.round(overflow),
      });

      await page.screenshot({
        path: resolve(screenshotRoot, `${route.name}-${width}.png`),
        fullPage: true,
        animations: "disabled",
      });
    }

    await context.close();
  }

  await writeFile(
    resolve(outputRoot, "visual-layout-baseline.json"),
    `${JSON.stringify(
      {
        generatedAt: new Date().toISOString(),
        standardWidthDecision: "max-w-6xl",
        widths,
        routes: routes.map((route) => route.path),
        measurements,
      },
      null,
      2,
    )}\n`,
  );
  console.log(`Captured ${measurements.length} visual baselines in ${outputRoot}`);
} finally {
  await browser.close();
}
