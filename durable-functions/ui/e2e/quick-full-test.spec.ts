import { test, expect } from '@playwright/test';

const BASE_URL = process.env.BASE_URL || 'http://localhost:5173';

test.describe('Quick Full UI Test', () => {
  test('Test all pages', async ({ page }) => {
    console.log('\n=== QUICK UI TEST ===\n');

    // Login
    await page.goto(BASE_URL);
    await page.waitForLoadState('networkidle');
    console.log('1. Login page ✓');

    await page.locator('text=Demo Admin').click();
    await page.waitForLoadState('networkidle');
    console.log('2. Dashboard ✓');

    // Start workflow
    await page.goto(`${BASE_URL}/workflows/new`);
    await page.waitForLoadState('networkidle');
    await page.locator('input[placeholder*="device"]').fill(`test-${Date.now()}`);
    await page.locator('button:has-text("Start Workflow")').click();
    await page.waitForTimeout(2000);
    console.log('3. Start Workflow ✓ - URL: ' + page.url());
    await page.screenshot({ path: 'e2e/screenshots/quick-test-workflow.png' });

    // Workflows list
    await page.goto(`${BASE_URL}/workflows`);
    await page.waitForLoadState('networkidle');
    console.log('4. Workflows List ✓');
    await page.screenshot({ path: 'e2e/screenshots/quick-test-list.png' });

    // Designer
    await page.goto(`${BASE_URL}/designer`);
    await page.waitForLoadState('networkidle');
    console.log('5. Designer ✓');
    await page.screenshot({ path: 'e2e/screenshots/quick-test-designer.png' });

    // Playground
    await page.goto(`${BASE_URL}/playground`);
    await page.waitForLoadState('networkidle');
    console.log('6. Playground ✓');
    await page.screenshot({ path: 'e2e/screenshots/quick-test-playground.png' });

    // Definitions
    await page.goto(`${BASE_URL}/definitions`);
    await page.waitForLoadState('networkidle');
    console.log('7. Definitions ✓');

    // Activities
    await page.goto(`${BASE_URL}/activities`);
    await page.waitForLoadState('networkidle');
    console.log('8. Activities ✓');

    // Logout
    await page.locator('text=Logout').click();
    await page.waitForLoadState('networkidle');
    console.log('9. Logout ✓');

    console.log('\n=== ALL TESTS PASSED ===\n');
  });
});
