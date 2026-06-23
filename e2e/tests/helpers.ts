import { Page, expect, APIRequestContext } from '@playwright/test';

export const ADMIN_USER = process.env.ADMIN_USER ?? 'admin';
export const ADMIN_PASSWORD = process.env.ADMIN_PASSWORD ?? 'admin';

/** The six wall configs shipped in Resources/configs. */
export const CONFIGS = ['solo', 'duo', 'wall', 'tower', 'twins', 'totem'] as const;

/** The day-states shipped in Resources/states. */
export const STATES = [
  'weekday-morning',
  'weekday-lunch-rush',
  'weekend-evening',
] as const;

/** Logs into the admin UI through the login form; asserts it lands on /admin/stores. */
export async function adminLogin(page: Page): Promise<void> {
  await page.goto('/admin/login');
  await page.fill('input[name="username"]', ADMIN_USER);
  await page.fill('input[name="password"]', ADMIN_PASSWORD);
  await page.click('button[type="submit"]');
  await expect(page).toHaveURL(/\/admin\/stores$/);
}

/** Asserts a rendered screen page is self-contained and well-formed. */
export function expectScreenIsRenderable(html: string): void {
  expect(html.startsWith('<!DOCTYPE html>')).toBeTruthy();
  expect(html).not.toContain('<script');
  // The board chrome and at least one menu item must be present.
  expect(html).toContain('class="board"');
  expect(html).toContain('class="menu-item"');
}

/** Fetches text via the request context (shares cookies with the page). */
export async function getText(req: APIRequestContext, url: string): Promise<string> {
  const res = await req.get(url);
  expect(res.ok()).toBeTruthy();
  return res.text();
}
