import { expect, test } from "@playwright/test";

test("signup collects only persisted data and recovers from creation failure", async ({ page }) => {
  await page.route("**/api/auth/sign-up", (route) =>
    route.fulfill({ status: 500, contentType: "application/json", body: "{}" }),
  );

  await page.goto("/en/signup");
  await expect(page.getByText("Step 1 of 3", { exact: true })).toBeVisible();
  await expect(page.getByLabel("Name")).toHaveCount(0);

  await page.getByLabel("Email").fill("new-operator@example.com");
  await page.getByLabel(/^Password/).fill("correct-horse");
  await page.getByRole("button", { name: "Sign up", exact: true }).click();

  const persistedDraft = await page.evaluate(() =>
    sessionStorage.getItem("opswarden_onboarding_draft"),
  );
  expect(persistedDraft).not.toContain("correct-horse");
  expect(persistedDraft).not.toContain("password");
  expect(persistedDraft).not.toContain("step");

  await expect(page.getByText("Step 2 of 3", { exact: true })).toBeVisible();
  await page.getByLabel("Team").fill("Platform Operations");
  await expect(page.getByLabel("Timezone")).toHaveCount(0);
  await page.getByRole("button", { name: "Next", exact: true }).click();

  await expect(page.getByText("Step 3 of 3", { exact: true })).toBeVisible();
  await expect(page.getByText("Account creation failed.", { exact: false })).toBeVisible();
  await expect(page.getByText(/GENERATING|PROMETHEUS|SECURITY PROTOCOLS/)).toHaveCount(0);

  await page.getByRole("button", { name: "Back", exact: true }).click();
  await expect(page.getByText("Step 2 of 3", { exact: true })).toBeVisible();
  await expect(page.getByLabel("Team")).toHaveValue("Platform Operations");
});
