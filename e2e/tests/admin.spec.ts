import { test, expect } from '@playwright/test';
import { adminLogin } from './helpers';

/**
 * Admin app: auth guard, store + monitor CRUD, menu editor, and the
 * admin-controlled featured flag. These require the DB-backed Axum server.
 *
 * Tests create their own throwaway records (unique slugs) and clean them up,
 * so they don't disturb seeded data.
 */

const uniq = () => Math.random().toString(36).slice(2, 8);

test('unauthenticated admin access redirects to login', async ({ page }) => {
  await page.goto('/admin/stores');
  await expect(page).toHaveURL(/\/admin\/login$/);
  await expect(page.getByRole('heading', { name: /sign in/i })).toBeVisible();
});

test('wrong credentials are rejected', async ({ page }) => {
  await page.goto('/admin/login');
  await page.fill('input[name="username"]', 'admin');
  await page.fill('input[name="password"]', 'definitely-wrong');
  await page.click('button[type="submit"]');
  await expect(page.getByText(/invalid username or password/i)).toBeVisible();
});

test('admin can create, view and delete a store with a monitor', async ({ page }) => {
  await adminLogin(page);
  const slug = `e2e-store-${uniq()}`;

  await page.goto('/admin/stores');
  await page.fill('form[action="/admin/stores"] input[name="name"]', 'E2E Test Store');
  await page.fill('form[action="/admin/stores"] input[name="slug"]', slug);
  await page.click('form[action="/admin/stores"] button[type="submit"]');
  await expect(page.getByText('E2E Test Store')).toBeVisible();

  await page.getByRole('row', { name: /E2E Test Store/ }).getByRole('link', { name: 'Manage' }).click();
  await expect(page).toHaveURL(/\/admin\/stores\/[0-9a-f-]{36}$/);
  await page.fill('form[action$="/screens"] input[name="label"]', 'Counter Left');
  await page.fill('form[action$="/screens"] input[name="width_px"]', '1920');
  await page.fill('form[action$="/screens"] input[name="height_px"]', '1080');
  await page.click('form[action$="/screens"] button[type="submit"]');
  await expect(page.locator('input[value="Counter Left"]')).toBeVisible();

  // The store's wall renders from that monitor.
  const wall = await page.request.get(`/store/${slug}`);
  expect(wall.ok()).toBeTruthy();

  // Clean up: delete the store (cascades to the monitor).
  await page.goto('/admin/stores');
  page.once('dialog', (d) => d.accept());
  await page.getByRole('row', { name: /E2E Test Store/ }).getByRole('button', { name: 'Delete' }).click();
  await expect(page.getByText('E2E Test Store')).toHaveCount(0);
});

test('admin menu editor lists categories and supports item CRUD with featured flag', async ({ page }) => {
  await adminLogin(page);
  await page.goto('/admin/menu');
  await expect(page.getByRole('heading', { name: 'Menu' })).toBeVisible();

  const slug = `e2e-cat-${uniq()}`;
  await page.fill('form[action="/admin/menu"] input[name="name"]', 'E2E Specials');
  await page.fill('form[action="/admin/menu"] input[name="slug"]', slug);
  await page.click('form[action="/admin/menu"] button[type="submit"]');
  await expect(page.getByText('E2E Specials')).toBeVisible();

  await page.getByRole('row', { name: /E2E Specials/ }).getByRole('link', { name: 'Edit' }).click();
  await expect(page).toHaveURL(/\/admin\/menu\/[0-9a-f-]{36}$/);
  const addItem = page.locator('form[action$="/items"]');
  await addItem.locator('input[name="name"]').fill('E2E Dish');
  await addItem.locator('input[name="slug"]').fill(`e2e-dish-${uniq()}`);
  await addItem.locator('input[name="price_min"]').fill('9.99');
  await addItem.locator('input[name="image"]').fill('https://example.com/photo.jpg');
  await addItem.locator('input[name="featured"]').check();
  await addItem.locator('button[type="submit"]').click();
  await expect(page.locator('input[value="E2E Dish"]')).toBeVisible();

  // The item's Feature checkbox should be checked (admin-controlled special).
  const itemForm = page.locator('form[action$="/update"]', {
    has: page.locator('input[value="E2E Dish"]'),
  });
  await expect(itemForm.locator('input[name="featured"]')).toBeChecked();

  // Clean up: delete the category (cascades to the item).
  await page.goto('/admin/menu');
  page.once('dialog', (d) => d.accept());
  await page.getByRole('row', { name: /E2E Specials/ }).getByRole('button', { name: 'Delete' }).click();
  await expect(page.getByText('E2E Specials')).toHaveCount(0);
});
