const { test, expect } = require('@playwright/test');

test('Web bundle loads', async ({ page }) => {
  test.setTimeout(150_000);
  const pageErrors: string[] = [];
  const consoleErrors: string[] = [];
  page.on('pageerror', error => pageErrors.push(error.message));
  page.on('console', message => {
    if (message.type() === 'error') consoleErrors.push(message.text());
  });

  for (const viewport of [{ width: 1440, height: 900 }, { width: 960, height: 640 }]) {
    await page.setViewportSize(viewport);
    await page.goto('http://localhost:8999/', { waitUntil: 'commit', timeout: 90_000 });
    await page.waitForFunction(
      () => Reflect.get(window, 'wasmBindings') !== undefined,
      undefined,
      { timeout: 90_000 },
    );
    const canvas = page.locator('#firecraft-canvas');
    await expect(canvas).toBeVisible({ timeout: 30_000 });
    await expect.poll(() => canvas.evaluate(element => {
      const canvasElement = element as HTMLCanvasElement;
      return canvasElement.clientWidth > 0 && canvasElement.clientHeight > 0;
    }), { timeout: 30_000 }).toBe(true);
    await page.screenshot({
      path: `dist/windows-review/wasm-${viewport.width}x${viewport.height}.png`,
      fullPage: true,
    });
  }

  expect(pageErrors, 'uncaught page errors').toEqual([]);
  expect(consoleErrors, 'console errors').toEqual([]);
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
