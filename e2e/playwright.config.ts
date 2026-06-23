import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright config for the Digital Menu Board servers.
 *
 * Point it at a running `dmbr-server-axum` (default http://localhost:8080).
 * Override with env vars — no secrets are hard-coded:
 *   BASE_URL        server URL (default http://localhost:8080)
 *   ADMIN_USER      admin username (default "admin")
 *   ADMIN_PASSWORD  admin password (default "admin")
 *
 * Run the server yourself, or let Playwright start it via `webServer` below
 * when START_SERVER=1 and DATABASE_URL are set.
 */
const BASE_URL = process.env.BASE_URL ?? 'http://localhost:8080';

export default defineConfig({
  testDir: './tests',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: [['list'], ['html', { open: 'never' }]],
  use: {
    baseURL: BASE_URL,
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },
  projects: [
    // The gallery/admin pages are driven at a desktop viewport; individual
    // screen pages declare their own native resolution in their markup.
    { name: 'chromium', use: { ...devices['Desktop Chrome'], viewport: { width: 1440, height: 900 } } },
  ],
  // Optionally boot the server. Requires DATABASE_URL; safe to leave off if you
  // run the server in another terminal.
  webServer: process.env.START_SERVER
    ? {
        command: 'cargo run -p dmbr-server-axum',
        cwd: '..',
        url: BASE_URL,
        timeout: 180_000,
        reuseExistingServer: true,
        env: {
          DATABASE_URL: process.env.DATABASE_URL ?? '',
          RESOURCES_DIR: process.env.RESOURCES_DIR ?? '../Resources',
          SESSION_SECRET: process.env.SESSION_SECRET ?? 'e2e-secret',
          PORT: '8080',
        },
      }
    : undefined,
});
