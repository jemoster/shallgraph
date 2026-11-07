import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./visual-tests",
  timeout: 30_000,
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI ? "github" : "list",
  use: {
    browserName: "chromium",
    locale: "en-US",
    timezoneId: "UTC",
    colorScheme: "light",
    reducedMotion: "reduce",
    deviceScaleFactor: 1,
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
  projects: [
    { name: "desktop", use: { ...devices["Desktop Chrome"], viewport: { width: 1280, height: 900 } } },
    { name: "mobile", use: { ...devices["iPhone 13"], viewport: { width: 390, height: 844 } } },
  ],
  expect: {
    toHaveScreenshot: {
      // Allow minor platform antialiasing differences while still failing meaningful layout changes.
      maxDiffPixels: 500,
    },
  },
  snapshotPathTemplate: "{testDir}/snapshots/{projectName}/{arg}{ext}",
});
