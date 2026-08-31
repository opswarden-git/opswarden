import { expect, test, type Page } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import * as demo from "./demo-env";

interface InteractiveItem {
  role: string;
  route: string;
  region: string;
  tag: string;
  semanticRole: string;
  label: string;
  disabled: boolean;
}

const inventory: InteractiveItem[] = [];

function saveReport() {
  const targetDir = resolve(process.cwd(), "target");
  mkdirSync(targetDir, { recursive: true });
  writeFileSync(
    resolve(targetDir, "ui-interaction-inventory.json"),
    JSON.stringify(inventory, null, 2),
  );

  const rows = inventory.map(
    (item) =>
      `| ${item.role} | \`${item.route}\` | ${item.region} | \`${item.tag}\` | \`${item.semanticRole}\` | ${item.label.replace(/\|/g, "\\|")} | ${item.disabled ? "Yes" : "No"} |`,
  );
  writeFileSync(
    resolve(targetDir, "ui-interaction-inventory.md"),
    [
      "# OpsWarden interactive-element inventory",
      "",
      "> Discovery report only. Explicit Playwright scenarios own mutation and navigation checks.",
      "",
      `- Elements discovered: ${inventory.length}`,
      "",
      "| Role | Route | Region | Tag | Semantic role | Accessible label | Disabled |",
      "| --- | --- | --- | --- | --- | --- | --- |",
      ...rows,
      "",
    ].join("\n"),
  );
}

async function login(page: Page, email: string) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password", { exact: true }).fill(demo.DEMO_PASSWORD);
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(demo.TEAM_URL_PATTERN);
  await demo.finishGuidedTour(page);
}

async function inventoryRole(page: Page, role: string, email: string) {
  const runtimeErrors: string[] = [];
  const serverErrors: string[] = [];
  page.on("pageerror", (error) => runtimeErrors.push(`${page.url()}: ${error.message}`));
  page.on("response", (response) => {
    if (response.status() >= 500) {
      serverErrors.push(`${response.request().method()} ${response.url()} -> ${response.status()}`);
    }
  });

  await login(page, email);
  const routes = ["overview", "incidents", "releases", "runs", "rules", "integrations", "team"].map(
    (section) => `/en/teams/${demo.DEMO_TEAM_ID}/${section}`,
  );
  routes.push("/en/settings");

  const unnamed: string[] = [];
  for (const route of routes) {
    await page.goto(route);
    await page.waitForLoadState("domcontentloaded");

    const items = await page
      .locator(
        "a[href], button, summary, select, input:not([type='hidden']), textarea, [role='button'], [role='tab'], [role='menuitem'], [role='combobox']",
      )
      .evaluateAll((elements) =>
        elements
          .filter((element) => {
            const style = window.getComputedStyle(element);
            const rect = element.getBoundingClientRect();
            return style.visibility !== "hidden" && style.display !== "none" && rect.width > 0;
          })
          .map((element) => {
            const html = element as HTMLElement;
            const control = element as HTMLInputElement;
            const labelledBy = element.getAttribute("aria-labelledby");
            const labelledText = labelledBy
              ?.split(/\s+/)
              .map((id) => document.getElementById(id)?.textContent ?? "")
              .join(" ");
            const imageText = Array.from(element.querySelectorAll("img[alt]"))
              .map((image) => image.getAttribute("alt") ?? "")
              .join(" ");
            const label = (
              element.getAttribute("aria-label") ||
              labelledText ||
              ("labels" in control
                ? Array.from(control.labels ?? [])
                    .map((item) => item.textContent ?? "")
                    .join(" ")
                : "") ||
              html.innerText ||
              imageText ||
              element.getAttribute("title") ||
              ""
            )
              .trim()
              .replace(/\s+/g, " ")
              .slice(0, 120);
            const region = element.closest("dialog")
              ? "Dialog"
              : element.closest("nav")
                ? "Navigation"
                : element.closest("aside")
                  ? "Aside"
                  : element.closest("table")
                    ? "Table"
                    : "Main";
            return {
              region,
              tag: element.tagName.toLowerCase(),
              semanticRole: element.getAttribute("role") || element.tagName.toLowerCase(),
              label,
              disabled: control.disabled || element.getAttribute("aria-disabled") === "true",
            };
          }),
      );

    for (const item of items) {
      if (!item.label) unnamed.push(`${route}: ${item.tag}[role=${item.semanticRole}]`);
      inventory.push({ ...item, role, route });
    }
    saveReport();
  }

  expect(unnamed, `Unnamed interactive elements:\n${unnamed.join("\n")}`).toEqual([]);
  expect(runtimeErrors, `Runtime errors:\n${runtimeErrors.join("\n")}`).toEqual([]);
  expect(serverErrors, `Server errors:\n${serverErrors.join("\n")}`).toEqual([]);
}

test.describe("interactive-element inventory", () => {
  test("Manager surfaces are named and runtime-safe", async ({ page }) => {
    await inventoryRole(page, "Manager", demo.DEMO_MANAGER_EMAIL);
  });

  test("Responder surfaces are named and runtime-safe", async ({ page }) => {
    await inventoryRole(page, "Responder", demo.DEMO_RESPONDER_EMAIL);
  });

  test("Observer surfaces are named and runtime-safe", async ({ page }) => {
    await inventoryRole(page, "Observer", demo.DEMO_OBSERVER_EMAIL);
  });
});
