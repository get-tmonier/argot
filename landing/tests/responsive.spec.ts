import { expect, test } from '@playwright/test';

const routes = ['/', '/#audit', '/#engine', '/benchmarks/', '/#proof', '/docs/', '/privacy/'];
const widths = [320, 375, 768, 1440];

for (const width of widths) {
  test(`primary navigation does not overflow at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    for (const route of routes) {
      await page.goto(route);
      const hasNoOverflow = await page.locator('header').evaluate(
        (element: HTMLElement) => element.scrollWidth <= element.clientWidth,
      );
      expect(hasNoOverflow).toBe(true);
    }
  });
}

test('home remains operable with reduced motion', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');
  await expect(page.getByRole('main')).toBeVisible();
  await expect(page.getByRole('link', { name: 'Skip to main content' })).toBeVisible();
});
