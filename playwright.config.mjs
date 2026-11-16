import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: 'tests/browser',
  timeout: 30_000,
  use: {
    baseURL: 'http://127.0.0.1:8080',
    viewport: {width: 1280, height: 900},
    trace: 'retain-on-failure',
    headless: !process.env.CI,
  },
  projects: [
    {name: 'chromium', use: {browserName: 'chromium', launchOptions: {args: ['--enable-unsafe-swiftshader']}}},
    {name: 'firefox', use: {browserName: 'firefox'}},
  ],
  webServer: {
    command: 'node scripts/serve-web.mjs',
    url: 'http://127.0.0.1:8080/',
    timeout: 10_000,
  },
});
