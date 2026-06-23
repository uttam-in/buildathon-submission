import { test, expect } from '@playwright/test';
import { CONFIGS, STATES, expectScreenIsRenderable, getText } from './helpers';

/**
 * The public renderer: picker → board gallery → individual screen, for every
 * wall config and day-state. These need no database (they read Resources/).
 */

test('home picker lists every wall config', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: /Menu Boards/i })).toBeVisible();
  for (const cfg of CONFIGS) {
    await expect(page.locator(`a[href="/config/${cfg}"]`)).toHaveCount(1);
  }
});

for (const cfg of CONFIGS) {
  test(`board gallery renders for config "${cfg}" at lunch`, async ({ page }) => {
    await page.goto(`/board/${cfg}/weekday-lunch-rush`);
    await expect(page.getByText(/render_hash/i)).toBeVisible();
    const screenCards = page.locator('a[href^="/screen/"]');
    await expect(screenCards.first()).toBeVisible();
  });
}

test('a single screen page is self-contained and shows menu items', async ({ page, request }) => {
  await page.goto('/board/wall/weekday-lunch-rush');
  const href = await page.locator('a[href^="/screen/"]').first().getAttribute('href');
  expect(href).toBeTruthy();
  const html = await getText(request, href!);
  expectScreenIsRenderable(html);
});

for (const state of STATES) {
  test(`wall renders renderable screens for state "${state}"`, async ({ page, request }) => {
    await page.goto(`/board/wall/${state}`);
    const links = await page.locator('a[href^="/screen/"]').evaluateAll((els) =>
      els.map((e) => (e as HTMLAnchorElement).getAttribute('href')!),
    );
    expect(links.length).toBeGreaterThan(0);
    for (const href of links) {
      const html = await getText(request, href);
      expectScreenIsRenderable(html);
    }
  });
}

test('every available item appears exactly once across a single-screen wall', async ({ request }) => {
  // The solo wall holds the whole menu on one screen (cycling across pages).
  const galleryHtml = await getText(request, '/board/solo/weekday-lunch-rush');
  const screenHref = galleryHtml.match(/href="(\/screen\/[^"]+)"/)?.[1];
  expect(screenHref).toBeTruthy();
  const html = await getText(request, screenHref!);

  const itemRows = html.match(/class="menu-item"/g)?.length ?? 0;
  expect(itemRows).toBeGreaterThan(200); // the lunch menu is dense

  // Item names must be unique (no duplication across cycle pages).
  const names = [...html.matchAll(/class="item-name">([^<]+)</g)].map((m) => m[1]);
  const unique = new Set(names);
  expect(unique.size).toBe(names.length);
});

test('render is deterministic: two fetches are byte-identical', async ({ request }) => {
  const galleryHtml = await getText(request, '/board/wall/weekday-lunch-rush');
  const screenHref = galleryHtml.match(/href="(\/screen\/[^"]+)"/)?.[1];
  expect(screenHref).toBeTruthy();
  const a = await getText(request, screenHref!);
  const b = await getText(request, screenHref!);
  expect(a).toBe(b);
});
