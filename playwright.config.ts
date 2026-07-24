import { defineConfig } from "@playwright/test";

const chromePath = process.env.PLAYWRIGHT_CHROME_PATH;

export default defineConfig({
  testDir: "./tooling/e2e",
  globalSetup: "./tooling/e2e/reset-demo.ts",
  globalTeardown: "./tooling/e2e/reset-demo.ts",
  fullyParallel: false,
  workers: 1,
  reporter: "list",
  use: {
    baseURL: process.env.OPSWARDEN_E2E_URL ?? "http://localhost:8081",
    headless: true,
    // CI uses Playwright's bundled Chromium. Nix/local environments can point
    // at their system browser without making that host-specific path mandatory.
    launchOptions: chromePath ? { executablePath: chromePath } : undefined,
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
});
