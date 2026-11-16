import { expect, test } from '@playwright/test';

test('starts the packaged app and applies an equation through the canvas UI', async ({page}) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('/');
  const status = page.getByRole('status');
  await expect(status).toHaveAttribute('data-state', 'ready');
  const canvas = page.getByLabel('Mathematical playground', {exact: true});
  await expect(canvas).toBeVisible();
  await canvas.focus();
  const editor = page.locator('#playground + input[type=text]');
  // egui exposes its canvas text editor through a sibling DOM input while focused.
  for (let index = 0; index < 24; index++) {
    if (await editor.evaluate(input => input === document.activeElement, undefined, {timeout: 3000})) break;
    await page.keyboard.press('Tab');
    await page.waitForFunction(() => new Promise(resolve => requestAnimationFrame(() => resolve(true))));
  }
  await expect(editor).toBeFocused();
  const request = 'y=x^2+7';
  await page.keyboard.press('Control+a');
  await page.keyboard.type(request);
  await page.keyboard.press('Enter');
  await expect.poll(() => page.evaluate(equation => {
    const raw = localStorage.getItem('math-playground.history.v1');
    return raw && JSON.parse(raw).entries.some(entry =>
      entry.request === equation && entry.outcome.status === 'success');
  }, request)).toBeTruthy();
  await expect(status).toHaveAttribute('data-state', 'ready');
  expect(errors).toEqual([]);
});

test('a failed Wasm download exposes a recoverable error', async ({page}) => {
  await page.route('**/*_bg.wasm', route => route.fulfill({status: 503}));
  await page.goto('/');
  const status = page.getByRole('status');
  await expect(status).toHaveAttribute('data-state', 'runtime-error');
  await expect(status).toContainText(/reload to retry/i);
});
