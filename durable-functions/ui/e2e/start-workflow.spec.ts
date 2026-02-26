import { test, expect } from '@playwright/test';

const BASE_URL = process.env.BASE_URL || 'http://localhost:5173';

test.describe('Start Workflow Test', () => {
  test('Try to start a workflow from UI', async ({ page }) => {
    const errors: string[] = [];

    // Capture console messages
    page.on('console', msg => {
      if (msg.type() === 'error') {
        errors.push(`[console.error] ${msg.text()}`);
      }
    });

    page.on('pageerror', error => {
      errors.push(`[pageerror] ${error.message}`);
    });

    // Login
    await page.goto(BASE_URL);
    await page.waitForLoadState('networkidle');
    await page.locator('text=Demo Admin').click();
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);

    console.log('Logged in as Demo Admin');

    // Click "Start New Workflow" button on dashboard
    const startButton = page.locator('text=Start New Workflow').first();
    if (await startButton.isVisible()) {
      console.log('Found "Start New Workflow" button, clicking...');
      await startButton.click();
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(1000);

      await page.screenshot({ path: 'e2e/screenshots/start-workflow-page.png', fullPage: true });

      const bodyText = await page.locator('body').innerText();
      console.log('Start Workflow page content:');
      console.log(bodyText.substring(0, 800));
    }

    // Try navigating to /workflows and clicking Start Workflow there
    console.log('\n--- Trying from Workflows page ---');
    await page.goto(`${BASE_URL}/workflows`);
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);

    const startWorkflowBtn = page.locator('button:has-text("Start Workflow"), a:has-text("Start Workflow")').first();
    if (await startWorkflowBtn.isVisible()) {
      console.log('Found "Start Workflow" button on workflows page, clicking...');
      await startWorkflowBtn.click();
      await page.waitForLoadState('networkidle');
      await page.waitForTimeout(1000);

      await page.screenshot({ path: 'e2e/screenshots/start-workflow-modal.png', fullPage: true });

      const bodyText = await page.locator('body').innerText();
      console.log('After clicking Start Workflow:');
      console.log(bodyText.substring(0, 800));
    }

    // Try using the Playground to execute a workflow
    console.log('\n--- Trying from Playground ---');
    await page.goto(`${BASE_URL}/playground`);
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);

    // Look for definition selector
    const definitionSelect = page.locator('select, [role="combobox"]').first();
    if (await definitionSelect.isVisible()) {
      console.log('Found definition selector');
      await definitionSelect.click();
      await page.waitForTimeout(500);
      await page.screenshot({ path: 'e2e/screenshots/playground-select-open.png', fullPage: true });

      // Try to select an option
      const options = page.locator('option, [role="option"]');
      const optionCount = await options.count();
      console.log(`Found ${optionCount} options`);

      if (optionCount > 1) {
        // Select first real option (not the placeholder)
        await options.nth(1).click();
        await page.waitForTimeout(500);
      }
    }

    // Look for Execute button
    const executeBtn = page.locator('button:has-text("Execute"), button:has-text("Run")').first();
    if (await executeBtn.isVisible()) {
      console.log('Found Execute button');
      await executeBtn.click();
      await page.waitForTimeout(2000);
      await page.screenshot({ path: 'e2e/screenshots/playground-executed.png', fullPage: true });
    }

    // Report errors
    if (errors.length > 0) {
      console.log('\n--- ERRORS FOUND ---');
      errors.forEach(e => console.log(e));
    } else {
      console.log('\n--- No errors found ---');
    }
  });
});
