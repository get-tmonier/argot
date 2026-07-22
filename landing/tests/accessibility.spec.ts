import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '@playwright/test';

const routes = ['/', '/#audit', '/#engine', '/benchmarks/', '/#proof', '/docs/', '/privacy/'];

for (const route of routes) {
  test(`${route} has no serious or critical desktop axe findings`, async ({ page, isMobile }) => {
    test.skip(
      isMobile,
      'mobile layouts are covered by the recorded responsive matrix and navigation smoke',
    );
    await page.goto(route);
    const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(
      results.violations.filter((violation) =>
        ['serious', 'critical'].includes(violation.impact ?? ''),
      ),
    ).toEqual([]);
  });
}

test('mobile navigation opens, closes with Escape, and restores focus', async ({
  page,
  isMobile,
}) => {
  test.skip(!isMobile, 'mobile-only behavior');
  await page.goto('/');
  const toggle = page.getByRole('button', { name: 'Open navigation' });
  await toggle.click();
  await expect(
    page.locator('#mobile-navigation').getByRole('link', { name: 'Audit' }),
  ).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(toggle).toBeFocused();
  await expect(toggle).toHaveAccessibleName('Open navigation');
});
