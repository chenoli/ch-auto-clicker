import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "ui/tests/visual",
  snapshotPathTemplate: "{testDir}/__screenshots__/{arg}{ext}",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  reporter: process.env.CI ? "github" : "list",
  expect: { toHaveScreenshot: { maxDiffPixelRatio: 0.001, animations: "disabled" } },
  use: { baseURL: "http://localhost:1420", deviceScaleFactor: 1 },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"], channel: "chromium" } }],
  webServer: { command: "pnpm dev", url: "http://localhost:1420/gallery.html", reuseExistingServer: !process.env.CI },
});
