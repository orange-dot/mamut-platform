# UI Testing Guide

This guide covers end-to-end (E2E) testing for the Orchestration Studio UI using Playwright.

## Overview

The UI uses [Playwright](https://playwright.dev/) for automated browser testing. Tests can run in:
- **Headed mode** (visible browser) - for development and debugging
- **Headless mode** - for CI/CD pipelines

## Prerequisites

```bash
cd ui
npm install
npx playwright install chromium
```

## Test Structure

Tests are located in `ui/e2e/`:

```
ui/e2e/
├── visual-test.spec.ts      # Basic page visibility tests
├── error-check.spec.ts      # Console error detection
├── workflow-detail.spec.ts  # Workflow detail page tests
├── start-workflow-manual.spec.ts  # Start workflow flow
├── quick-full-test.spec.ts  # Complete UI walkthrough
└── screenshots/             # Test screenshots
```

## Running Tests

### Headed Mode (Visible Browser)

For development and debugging - opens a real browser window:

```bash
cd ui

# Run all tests
npx playwright test --headed

# Run specific test file
npx playwright test e2e/quick-full-test.spec.ts --headed

# Run with extended timeout
npx playwright test --headed --timeout=60000
```

### Headless Mode (CI/CD)

For automated pipelines - no browser window:

```bash
cd ui

# Run all tests
npx playwright test

# Run with CI reporter
npx playwright test --reporter=github
```

### Debug Mode

```bash
# Step through tests with Playwright Inspector
npx playwright test --debug

# Run with trace recording
npx playwright test --trace on
```

## Configuration

Playwright configuration is in `ui/playwright.config.ts`:

```typescript
import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: process.env.CI ? 'github' : 'list',
  use: {
    baseURL: 'http://localhost:5175',
    trace: 'on-first-retry',
    screenshot: 'on',
    video: 'on-first-retry',
    headless: !!process.env.CI,
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
});
```

## Writing Tests

### Basic Test Structure

```typescript
import { test, expect } from '@playwright/test';

const BASE_URL = 'http://localhost:5175';

test.describe('Feature Name', () => {
  test('should do something', async ({ page }) => {
    // Navigate
    await page.goto(BASE_URL);
    await page.waitForLoadState('networkidle');

    // Interact
    await page.locator('text=Demo Admin').click();

    // Assert
    await expect(page.locator('h1')).toContainText('Dashboard');

    // Screenshot
    await page.screenshot({ path: 'e2e/screenshots/test.png' });
  });
});
```

### Testing with Authentication

The demo app uses session-based auth. Login before each test:

```typescript
test('authenticated test', async ({ page }) => {
  // Login
  await page.goto(BASE_URL);
  await page.locator('text=Demo Admin').click();
  await page.waitForLoadState('networkidle');

  // Now test authenticated pages
  await page.goto(`${BASE_URL}/workflows`);
  // ...
});
```

### Capturing Errors

```typescript
test('check for errors', async ({ page }) => {
  const errors: string[] = [];

  page.on('console', msg => {
    if (msg.type() === 'error') {
      errors.push(msg.text());
    }
  });

  page.on('response', response => {
    if (response.status() >= 400) {
      errors.push(`${response.status()} ${response.url()}`);
    }
  });

  // Run test...

  // Assert no errors
  expect(errors).toHaveLength(0);
});
```

## Test Scenarios

### Quick Full Test

Tests all major pages in sequence:

1. Login page loads
2. Login as Demo Admin
3. Start a new workflow
4. View workflows list
5. Visit Designer
6. Visit Playground
7. Visit Definitions
8. Visit Activities
9. Logout

### Visual Tests

Captures screenshots of all pages for visual regression testing.

### Error Check

Navigates through all pages checking for:
- JavaScript console errors
- Network request failures (4xx, 5xx)
- React warnings

## Screenshots

Tests automatically capture screenshots to `ui/e2e/screenshots/`. These can be used for:
- Visual regression testing
- Documentation
- Debugging test failures

## CI/CD Integration

See `.github/workflows/ui-tests.yml` for GitHub Actions integration.

### Required Environment

For CI, the tests need:
1. UI dev server running on port 5175
2. Backend API available on port 7071 (or mocked)
3. Chromium browser installed

### Running in CI

```yaml
- name: Install Playwright
  run: npx playwright install chromium --with-deps

- name: Run E2E Tests
  run: npx playwright test
  env:
    CI: true
```

## Troubleshooting

### Browser not found

```bash
npx playwright install chromium
```

### Tests timing out

Increase timeout:
```bash
npx playwright test --timeout=120000
```

### Selector issues

Use Playwright Inspector to find correct selectors:
```bash
npx playwright test --debug
```

### Port already in use

The dev server auto-selects available port. Update `BASE_URL` in tests if needed.
