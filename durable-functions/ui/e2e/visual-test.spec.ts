import { test, expect } from '@playwright/test';

const BASE_URL = process.env.BASE_URL || 'http://localhost:5173';

test.describe('UI Visual Testing', () => {
  test('Login page displays correctly', async ({ page }) => {
    await page.goto(BASE_URL);
    await page.waitForLoadState('networkidle');

    // Check login page elements
    await expect(page.locator('text=Orchestration Studio')).toBeVisible();
    await expect(page.locator('text=Azure Durable Functions Demo')).toBeVisible();
    await expect(page.locator('text=Demo Admin')).toBeVisible();
    await expect(page.locator('text=Demo Viewer')).toBeVisible();

    await page.screenshot({ path: 'e2e/screenshots/login-page.png', fullPage: true });
    console.log('Login page loaded successfully');
  });

  test('Login as Demo Admin and check sidebar', async ({ page }) => {
    await page.goto(BASE_URL);
    await page.waitForLoadState('networkidle');

    // Click on Demo Admin to login
    await page.locator('text=Demo Admin').click();
    await page.waitForLoadState('networkidle');

    // Take screenshot after login
    await page.screenshot({ path: 'e2e/screenshots/after-login.png', fullPage: true });

    // Check if sidebar is now visible
    const sidebar = page.locator('nav, aside, [class*="sidebar"]').first();
    const sidebarVisible = await sidebar.isVisible().catch(() => false);
    console.log(`Sidebar visible after login: ${sidebarVisible}`);

    // Check page content
    const bodyText = await page.locator('body').innerText();
    console.log('Page content after login:');
    console.log(bodyText.substring(0, 500));
  });

  test('Navigate through all pages after login', async ({ page }) => {
    await page.goto(BASE_URL);
    await page.waitForLoadState('networkidle');

    // Login as Demo Admin
    await page.locator('text=Demo Admin').click();
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);

    // Take screenshot of dashboard/home after login
    await page.screenshot({ path: 'e2e/screenshots/dashboard.png', fullPage: true });
    console.log('Dashboard page captured');

    // Find and list all navigation links
    const allLinks = page.locator('a[href]');
    const linkCount = await allLinks.count();
    console.log(`Found ${linkCount} links on page`);

    for (let i = 0; i < linkCount; i++) {
      const href = await allLinks.nth(i).getAttribute('href');
      const text = await allLinks.nth(i).innerText().catch(() => '[no text]');
      console.log(`  Link ${i + 1}: ${href} - "${text.trim()}"`);
    }
  });

  test('Test Workflow Designer page', async ({ page }) => {
    await page.goto(BASE_URL);
    await page.waitForLoadState('networkidle');

    // Login
    await page.locator('text=Demo Admin').click();
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);

    // Navigate to designer
    await page.goto(`${BASE_URL}/designer`);
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);

    await page.screenshot({ path: 'e2e/screenshots/workflow-designer.png', fullPage: true });

    const bodyText = await page.locator('body').innerText();
    console.log('Workflow Designer content:');
    console.log(bodyText.substring(0, 500));
  });

  test('Test Start Workflow page', async ({ page }) => {
    await page.goto(BASE_URL);
    await page.waitForLoadState('networkidle');

    // Login
    await page.locator('text=Demo Admin').click();
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);

    // Navigate to start workflow
    await page.goto(`${BASE_URL}/workflows/start`);
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);

    await page.screenshot({ path: 'e2e/screenshots/start-workflow.png', fullPage: true });

    const bodyText = await page.locator('body').innerText();
    console.log('Start Workflow content:');
    console.log(bodyText.substring(0, 500));
  });

  test('Test Workflow Instances page', async ({ page }) => {
    await page.goto(BASE_URL);
    await page.waitForLoadState('networkidle');

    // Login
    await page.locator('text=Demo Admin').click();
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);

    // Navigate to instances
    await page.goto(`${BASE_URL}/workflows`);
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);

    await page.screenshot({ path: 'e2e/screenshots/instances.png', fullPage: true });

    const bodyText = await page.locator('body').innerText();
    console.log('Instances content:');
    console.log(bodyText.substring(0, 500));
  });

  test('Test Playground page', async ({ page }) => {
    await page.goto(BASE_URL);
    await page.waitForLoadState('networkidle');

    // Login
    await page.locator('text=Demo Admin').click();
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);

    // Navigate to playground
    await page.goto(`${BASE_URL}/playground`);
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);

    await page.screenshot({ path: 'e2e/screenshots/playground.png', fullPage: true });

    const bodyText = await page.locator('body').innerText();
    console.log('Playground content:');
    console.log(bodyText.substring(0, 500));
  });

  test('Check responsive design - mobile view', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 }); // iPhone SE size

    await page.goto(BASE_URL);
    await page.waitForLoadState('networkidle');

    await page.screenshot({ path: 'e2e/screenshots/mobile-login.png', fullPage: true });

    // Login
    await page.locator('text=Demo Admin').click();
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);

    await page.screenshot({ path: 'e2e/screenshots/mobile-dashboard.png', fullPage: true });
    console.log('Mobile views captured');
  });
});
