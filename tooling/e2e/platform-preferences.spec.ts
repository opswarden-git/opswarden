import { expect, test, type Locator, type Page } from "@playwright/test";

const TEAM_ID = "39aa8884-22cc-4764-a9e7-7df7c7619ba6";

async function login(page: Page) {
  await page.goto("/en/login");
  await page.getByLabel("Email").fill("manager@opswarden.local");
  await page.getByLabel("Password", { exact: true }).fill("sudo");
  await page.getByRole("button", { name: "Log in", exact: true }).click();
  await expect(page).toHaveURL(/\/en\/teams\//);
}

async function renderedContrast(locator: Locator, pseudoElement?: "::placeholder") {
  return locator.evaluate((element, pseudo) => {
    type Color = [red: number, green: number, blue: number, alpha: number];

    const parse = (value: string): Color => {
      const canvas = document.createElement("canvas");
      canvas.width = 1;
      canvas.height = 1;
      const context = canvas.getContext("2d", { willReadFrequently: true });
      if (!context) throw new Error("Canvas color parser is unavailable");
      context.clearRect(0, 0, 1, 1);
      context.fillStyle = value;
      context.fillRect(0, 0, 1, 1);
      const [red, green, blue, alpha] = context.getImageData(0, 0, 1, 1).data;
      return [red, green, blue, alpha / 255];
    };

    const over = (foreground: Color, background: Color): Color => {
      const alpha = foreground[3] + background[3] * (1 - foreground[3]);
      if (alpha === 0) return [0, 0, 0, 0];
      return [
        (foreground[0] * foreground[3] + background[0] * background[3] * (1 - foreground[3])) /
          alpha,
        (foreground[1] * foreground[3] + background[1] * background[3] * (1 - foreground[3])) /
          alpha,
        (foreground[2] * foreground[3] + background[2] * background[3] * (1 - foreground[3])) /
          alpha,
        alpha,
      ];
    };

    let background: Color = [0, 0, 0, 0];
    for (let current: Element | null = element; current; current = current.parentElement) {
      background = over(background, parse(getComputedStyle(current).backgroundColor));
      if (background[3] >= 0.999) break;
    }

    const foreground = over(parse(getComputedStyle(element, pseudo).color), background);
    const luminance = ([red, green, blue]: Color) => {
      const channels = [red, green, blue].map((channel) => {
        const normalized = channel / 255;
        return normalized <= 0.04045 ? normalized / 12.92 : ((normalized + 0.055) / 1.055) ** 2.4;
      });
      return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
    };

    const foregroundLuminance = luminance(foreground);
    const backgroundLuminance = luminance(background);
    return (
      (Math.max(foregroundLuminance, backgroundLuminance) + 0.05) /
      (Math.min(foregroundLuminance, backgroundLuminance) + 0.05)
    );
  }, pseudoElement);
}

test("reduced motion keeps progress but neutralizes decorative motion", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/en/login");

  const result = await page.evaluate(() => {
    const decorative = document.createElement("span");
    decorative.className = "animate-pulse";
    const progress = document.createElement("span");
    progress.className = "ow-progress-spinner animate-spin";
    document.body.append(decorative, progress);

    const transition = getComputedStyle(document.querySelector("input")!).transitionDuration;
    const decorativeDuration = getComputedStyle(decorative).animationDuration;
    const progressDuration = getComputedStyle(progress).animationDuration;
    decorative.remove();
    progress.remove();
    return { decorativeDuration, progressDuration, transition };
  });

  const seconds = (duration: string) => Number.parseFloat(duration.replace("s", ""));
  expect(seconds(result.transition)).toBeLessThanOrEqual(0.00001);
  expect(seconds(result.decorativeDuration)).toBeLessThanOrEqual(0.00001);
  expect(seconds(result.progressDuration)).toBeGreaterThan(0.00001);
});

test("forced colors preserve focus, selection and action-menu geometry", async ({ page }) => {
  const session = await page.context().newCDPSession(page);
  await session.send("Emulation.setEmulatedMedia", {
    media: "screen",
    features: [{ name: "forced-colors", value: "active" }],
  });

  await page.goto("/en/login");
  expect(await page.evaluate(() => matchMedia("(forced-colors: active)").matches)).toBe(true);

  const email = page.getByLabel("Email");
  await email.focus();
  await expect(email).toBeFocused();
  expect(await email.evaluate((element) => getComputedStyle(element).outlineWidth)).toBe("2px");

  await login(page);
  const currentNavigationItem = page.locator(
    'a[data-app-navigation-item="true"]:visible[aria-current="page"]',
  );
  await expect(currentNavigationItem).toHaveCount(1);
  expect(
    await currentNavigationItem.evaluate((element) => getComputedStyle(element).outlineWidth),
  ).toBe("2px");

  await page.goto(`/en/teams/${TEAM_ID}/members`);
  await page.getByRole("button", { name: "Team Actions" }).first().click();
  const menuItem = page.getByRole("menuitem").first();
  await menuItem.hover();
  expect(await menuItem.evaluate((element) => getComputedStyle(element).outlineWidth)).toBe("2px");
});

test("browser-composited incident text remains above 4.5:1", async ({ page }) => {
  await login(page);

  const samples = [
    { path: `/en/teams/${TEAM_ID}/incidents`, selector: ".text-st-open" },
    { path: `/en/teams/${TEAM_ID}/incidents`, selector: ".text-sev-critical" },
    { path: `/en/teams/${TEAM_ID}/incidents`, selector: ".text-sev-low" },
    {
      path: `/en/teams/${TEAM_ID}/incidents?view=acknowledged`,
      selector: ".text-st-ack",
    },
  ];

  for (const sample of samples) {
    await page.goto(sample.path);
    const element = page.locator(sample.selector).first();
    await expect(element).toBeVisible();
    expect(
      await renderedContrast(element),
      `${sample.selector} rendered contrast`,
    ).toBeGreaterThanOrEqual(4.5);
  }

  const search = page.getByPlaceholder("Search by title, ID or assignee");
  expect(
    await renderedContrast(search, "::placeholder"),
    "search placeholder contrast",
  ).toBeGreaterThanOrEqual(4.5);
});
