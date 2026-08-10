import { chromium } from '@playwright/test';

(async () => {
  const browser = await chromium.launch();
  const context = await browser.newContext({ viewport: { width: 768, height: 900 } });
  const page = await context.newPage();
  
  // Login logic as in the tests
  await page.goto('http://localhost:8081/en/login');
  await page.fill('input[name="email"]', 'manager@opswarden.local');
  await page.fill('input[name="password"]', 'sudo');
  await page.click('button[type="submit"]');
  await page.waitForURL('http://localhost:8081/en/teams');
  
  // Go to incidents queue
  await page.goto('http://localhost:8081/en/teams/1/incidents');
  
  const nav = page.getByRole('navigation', { name: 'Breadcrumb' });
  const box = await nav.boundingBox();
  console.log("Nav BoundingBox:", box);
  const isVisible = await nav.isVisible();
  console.log("Is Visible:", isVisible);

  await browser.close();
})();
