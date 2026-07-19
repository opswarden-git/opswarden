import { defineConfig } from "@playwright/test";

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
    launchOptions: {
      executablePath:
        process.env.PLAYWRIGHT_CHROME_PATH ?? "/run/current-system/sw/bin/google-chrome",
    },
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
});
