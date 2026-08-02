import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/e2e",
  outputDir: "../artifacts/playwright",
  use: {
    baseURL: process.env.PARADE_BASE_URL || "http://127.0.0.1:8008",
    trace: "retain-on-failure",
  },
  webServer: {
    command: "../scripts/run-e2e-hub.sh",
    url: "http://127.0.0.1:8008",
    reuseExistingServer: true,
    timeout: 120_000,
  },
  projects: [
    {
      name: "desktop",
      use: { ...devices["Desktop Chrome"], colorScheme: "dark" },
    },
    {
      name: "mobile",
      use: { viewport: { width: 390, height: 844 }, colorScheme: "light" },
    },
  ],
});
