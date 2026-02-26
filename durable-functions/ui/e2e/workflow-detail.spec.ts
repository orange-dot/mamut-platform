import { test, expect } from '@playwright/test';

const BASE_URL = process.env.BASE_URL || 'http://localhost:5173';

test.describe('Workflow Detail Test', () => {
  test('Navigate to workflow detail page', async ({ page }) => {
    const errors: string[] = [];
    const networkErrors: string[] = [];

    page.on('console', msg => {
      if (msg.type() === 'error') {
        errors.push(`[console] ${msg.text()}`);
      }
    });

    page.on('response', response => {
      if (response.status() >= 400) {
        networkErrors.push(`[${response.status()}] ${response.url()}`);
      }
    });

    // Login
    await page.goto(BASE_URL);
    await page.waitForLoadState('networkidle');
    await page.locator('text=Demo Admin').click();
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);

    // Go to workflows page
    await page.goto(`${BASE_URL}/workflows`);
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);

    await page.screenshot({ path: 'e2e/screenshots/workflows-list.png', fullPage: true });

    // Find workflow instance links (exclude /new)
    const workflowLinks = page.locator('table a[href*="/workflows/"]');
    const count = await workflowLinks.count();
    console.log(`Found ${count} workflow links in table`);

    if (count > 0) {
      const href = await workflowLinks.first().getAttribute('href');
      console.log(`Clicking workflow link: ${href}`);
      await workflowLinks.first().click();
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(1000);

      await page.screenshot({ path: 'e2e/screenshots/workflow-detail.png', fullPage: true });

      const bodyText = await page.locator('body').innerText();
      console.log('Workflow Detail page content:');
      console.log(bodyText.substring(0, 1500));
    } else {
      console.log('No workflow links found in table');
    }

    // Report
    console.log('\n--- Console Errors ---');
    errors.forEach(e => console.log(e));

    console.log('\n--- Network Errors ---');
    networkErrors.forEach(e => console.log(e));
  });
});
