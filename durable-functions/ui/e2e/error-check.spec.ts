import { test, expect } from '@playwright/test';

const BASE_URL = process.env.BASE_URL || 'http://localhost:5173';

test.describe('UI Error Check', () => {
  test('Check for console errors on all pages', async ({ page }) => {
    const errors: string[] = [];
    const warnings: string[] = [];

    // Capture console messages
    page.on('console', msg => {
      if (msg.type() === 'error') {
        errors.push(`[${msg.type()}] ${msg.text()}`);
      }
      if (msg.type() === 'warning') {
        warnings.push(`[${msg.type()}] ${msg.text()}`);
      }
    });

    // Capture page errors
    page.on('pageerror', error => {
      errors.push(`[pageerror] ${error.message}`);
    });

    // Check login page
    console.log('\n=== Checking Login Page ===');
    await page.goto(BASE_URL);
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);

    // Login
    console.log('\n=== Logging in ===');
    await page.locator('text=Demo Admin').click();
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);

    // Check Dashboard
    console.log('\n=== Checking Dashboard ===');
    // Already on dashboard after login

    // Check Playground
    console.log('\n=== Checking Playground ===');
    await page.goto(`${BASE_URL}/playground`);
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);

    // Check Workflows
    console.log('\n=== Checking Workflows ===');
    await page.goto(`${BASE_URL}/workflows`);
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);

    // Check Designer
    console.log('\n=== Checking Designer ===');
    await page.goto(`${BASE_URL}/designer`);
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);

    // Check Definitions
    console.log('\n=== Checking Definitions ===');
    await page.goto(`${BASE_URL}/definitions`);
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);

    // Check Activities
    console.log('\n=== Checking Activities ===');
    await page.goto(`${BASE_URL}/activities`);
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(1000);

    // Report findings
    console.log('\n========== ERROR REPORT ==========');
    console.log(`Total Errors: ${errors.length}`);
    console.log(`Total Warnings: ${warnings.length}`);

    if (errors.length > 0) {
      console.log('\n--- ERRORS ---');
      errors.forEach((e, i) => console.log(`${i + 1}. ${e}`));
    }

    if (warnings.length > 0) {
      console.log('\n--- WARNINGS ---');
      warnings.forEach((w, i) => console.log(`${i + 1}. ${w}`));
    }

    console.log('\n==================================');
  });
});
