const { test, expect } = require('@playwright/test');

test('Web bundle loads', async ({ page }) => {
  await page.goto('http://localhost:8999/', { waitUntil: 'domcontentloaded', timeout: 60000 });
  const canvas = page.locator('#firecraft-canvas');
  await expect(canvas).toBeAttached();
  await page.waitForTimeout(8000);
});

test('version.json is accessible', async ({ request }) => {
  const resp = await request.get('http://localhost:8999/version.json');
  expect(resp.status()).toBe(200);
  const json = await resp.json();
  expect(json.version).toBeTruthy();
});

test('_redirects file is accessible', async ({ request }) => {
  const resp = await request.get('http://localhost:8999/_redirects');
  expect(resp.status()).toBe(200);
  const text = await resp.text();
  expect(text).toContain('index.html');
});
