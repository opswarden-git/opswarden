import { loadEnvFile } from "node:process";
import { resolve } from "node:path";
import type { Page } from "@playwright/test";

try {
  loadEnvFile(resolve(process.cwd(), ".env"));
} catch (error) {
  if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
}

const localValue = (name: string, fallback: string) =>
  process.env[`DEMO_LOCAL_${name}`]?.trim() || process.env[`DEMO_${name}`]?.trim() || fallback;

// The browser suite exercises the same single-Team dataset as the demo CLI.
// CI has no .env and therefore keeps the deterministic local fallbacks.
export const DEMO_PASSWORD =
  process.env.OPSWARDEN_E2E_PASSWORD?.trim() || localValue("PASSWORD", "sudo");
export const DEMO_TEAM_ID = localValue("TEAM_ID", "50000000-0000-4000-8000-000000000001");
export const DEMO_TEAM_NAME = localValue("TEAM_NAME", "OpsWarden Demo");
export const DEMO_MANAGER_EMAIL = localValue("MANAGER_EMAIL", "manager@opswarden.local");
export const DEMO_RESPONDER_EMAIL = localValue("RESPONDER_EMAIL", "responder@opswarden.local");
export const DEMO_OBSERVER_EMAIL = localValue("OBSERVER_EMAIL", "observer@opswarden.local");
export const DEMO_CONTRACTOR_EMAIL = localValue("CONTRACTOR_EMAIL", "contractor@opswarden.local");
export const TEAM_URL_PATTERN = /\/(?:en|fr)\/teams\//;

export async function finishGuidedTour(page: Page) {
  await page.evaluate((teamId) => {
    localStorage.setItem(`opswarden-tour:${teamId}`, "1");
    window.dispatchEvent(new Event("opswarden-tour-change"));
  }, DEMO_TEAM_ID);
}
