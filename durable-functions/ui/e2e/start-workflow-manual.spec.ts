import { test, expect } from '@playwright/test';

const BASE_URL = process.env.BASE_URL || 'http://localhost:5173';

test.describe('Manual Start Workflow Test', () => {
  test('Fill form and start a new workflow', async ({ page }) => {
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

    console.log('Logged in as Demo Admin');

    // Navigate to Start Workflow page
    await page.goto(`${BASE_URL}/workflows/new`);
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);

    await page.screenshot({ path: 'e2e/screenshots/start-workflow-form.png', fullPage: true });
    console.log('On Start Workflow page');

    // Fill in the Entity ID field (required)
    const entityIdInput = page.locator('input[placeholder*="device"], input[name*="entity"], input').first();
    await entityIdInput.fill('test-device-' + Date.now());
    console.log('Filled Entity ID');

    await page.screenshot({ path: 'e2e/screenshots/start-workflow-filled.png', fullPage: true });

    // Click Start Workflow button
    const startButton = page.locator('button:has-text("Start Workflow")');
    console.log('Clicking Start Workflow button...');
    await startButton.click();

    // Wait for response
    await page.waitForTimeout(3000);

    await page.screenshot({ path: 'e2e/screenshots/start-workflow-result.png', fullPage: true });

    // Check what happened
    const bodyText = await page.locator('body').innerText();
    console.log('\nPage content after clicking Start:');
    console.log(bodyText.substring(0, 1000));

    // Check URL - did we navigate away?
    const currentUrl = page.url();
    console.log(`\nCurrent URL: ${currentUrl}`);

    // Report errors
    console.log('\n--- Console Errors ---');
    if (errors.length === 0) {
      console.log('None');
    } else {
      errors.forEach(e => console.log(e));
    }

    console.log('\n--- Network Errors ---');
    if (networkErrors.length === 0) {
      console.log('None');
    } else {
      networkErrors.forEach(e => console.log(e));
    }
  });
});
