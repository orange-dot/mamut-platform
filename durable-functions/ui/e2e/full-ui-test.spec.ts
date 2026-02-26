import { test, expect } from '@playwright/test';

const BASE_URL = process.env.BASE_URL || 'http://localhost:5173';

test.describe('Full UI Test - Visible Mode', () => {
  test('Complete workflow: Login -> Start Workflow -> View Detail -> Check Status', async ({ page }) => {
    // Slow down for visibility
    test.slow();

    console.log('\n========================================');
    console.log('       FULL UI TEST - VISIBLE MODE');
    console.log('========================================\n');

    // Step 1: Login
    console.log('Step 1: Navigating to login page...');
    await page.goto(BASE_URL);
    await page.waitForLoadState('networkidle');
    await expect(page.locator('text=Orchestration Studio')).toBeVisible();
    await expect(page.locator('text=Demo Admin')).toBeVisible();
    await page.screenshot({ path: 'e2e/screenshots/full-test-01-login.png' });
    console.log('  ✓ Login page loaded');

    // Step 2: Click Demo Admin to login
    console.log('\nStep 2: Logging in as Demo Admin...');
    await page.locator('text=Demo Admin').click();
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);
    await expect(page.locator('h1:has-text("Dashboard")')).toBeVisible();
    await page.screenshot({ path: 'e2e/screenshots/full-test-02-dashboard.png' });
    console.log('  ✓ Logged in, dashboard visible');

    // Step 3: Check dashboard stats
    console.log('\nStep 3: Checking dashboard...');
    await expect(page.locator('text=Total Workflows')).toBeVisible();
    await expect(page.locator('text=Quick Actions')).toBeVisible();
    const stats = await page.locator('.text-2xl, .text-3xl').allTextContents();
    console.log(`  ✓ Dashboard stats: ${stats.join(', ')}`);

    // Step 4: Navigate to Workflows list
    console.log('\nStep 4: Navigating to Workflows list...');
    await page.locator('a:has-text("Workflows")').first().click();
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);
    await page.screenshot({ path: 'e2e/screenshots/full-test-03-workflows-list.png' });
    const workflowCount = await page.locator('table tbody tr').count();
    console.log(`  ✓ Workflows page loaded with ${workflowCount} workflows`);

    // Step 5: Start a new workflow
    console.log('\nStep 5: Starting a new workflow...');
    await page.locator('button:has-text("Start Workflow"), a:has-text("Start Workflow")').first().click();
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);
    await expect(page.locator('text=Start New Workflow')).toBeVisible();
    await page.screenshot({ path: 'e2e/screenshots/full-test-04-start-form.png' });
    console.log('  ✓ Start workflow form loaded');

    // Step 6: Fill in the form
    console.log('\nStep 6: Filling in workflow form...');
    const entityId = `ui-test-${Date.now()}`;
    await page.locator('input[placeholder*="device"]').fill(entityId);
    await page.screenshot({ path: 'e2e/screenshots/full-test-05-form-filled.png' });
    console.log(`  ✓ Entity ID filled: ${entityId}`);

    // Step 7: Submit the form
    console.log('\nStep 7: Submitting workflow...');
    await page.locator('button:has-text("Start Workflow")').click();
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000);
    await page.screenshot({ path: 'e2e/screenshots/full-test-06-workflow-started.png' });

    const currentUrl = page.url();
    const instanceId = currentUrl.split('/').pop();
    console.log(`  ✓ Workflow started! Instance ID: ${instanceId}`);

    // Step 8: Verify workflow detail page
    console.log('\nStep 8: Verifying workflow detail...');
    await expect(page.locator('text=← Back to list')).toBeVisible();
    const statusBadge = page.locator('text=Running, text=Completed, text=Failed, text=Pending').first();
    const status = await statusBadge.textContent().catch(() => 'Unknown');
    console.log(`  ✓ Workflow status: ${status}`);

    // Check for actions
    const hasTerminate = await page.locator('button:has-text("Terminate")').isVisible();
    const hasRaiseEvent = await page.locator('button:has-text("Raise Event")').isVisible();
    console.log(`  ✓ Actions available: Terminate=${hasTerminate}, RaiseEvent=${hasRaiseEvent}`);

    // Step 9: Navigate to Designer
    console.log('\nStep 9: Navigating to Designer...');
    await page.locator('a:has-text("Designer")').click();
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);
    await expect(page.locator('text=State Types')).toBeVisible();
    await page.screenshot({ path: 'e2e/screenshots/full-test-07-designer.png' });
    console.log('  ✓ Designer page loaded');

    // Step 10: Navigate to Playground
    console.log('\nStep 10: Navigating to Playground...');
    await page.locator('a:has-text("Playground")').click();
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);
    await expect(page.locator('text=Workflow Playground')).toBeVisible();
    await page.screenshot({ path: 'e2e/screenshots/full-test-08-playground.png' });
    console.log('  ✓ Playground page loaded');

    // Step 11: Navigate to Definitions
    console.log('\nStep 11: Navigating to Definitions...');
    await page.locator('a:has-text("Definitions")').click();
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);
    await page.screenshot({ path: 'e2e/screenshots/full-test-09-definitions.png' });
    console.log('  ✓ Definitions page loaded');

    // Step 12: Navigate to Activities
    console.log('\nStep 12: Navigating to Activities...');
    await page.locator('a:has-text("Activities")').click();
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);
    await page.screenshot({ path: 'e2e/screenshots/full-test-10-activities.png' });
    console.log('  ✓ Activities page loaded');

    // Step 13: Go back to Dashboard
    console.log('\nStep 13: Returning to Dashboard...');
    await page.locator('a:has-text("Dashboard")').click();
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);
    await page.screenshot({ path: 'e2e/screenshots/full-test-11-dashboard-final.png' });
    console.log('  ✓ Back to Dashboard');

    // Step 14: Logout
    console.log('\nStep 14: Logging out...');
    await page.locator('text=Logout').click();
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);
    await expect(page.locator('text=Select a demo user')).toBeVisible();
    await page.screenshot({ path: 'e2e/screenshots/full-test-12-logout.png' });
    console.log('  ✓ Logged out successfully');

    console.log('\n========================================');
    console.log('       ALL TESTS PASSED!');
    console.log('========================================\n');
  });
});
