import { defineConfig } from "@playwright/test";

// Port: OMNIMON_E2E_PORT (default 1420, the Vite/Tauri dev port). Reusing
// whatever already listens on the port would test a build that is not the one
// under test (another checkout, a running `tauri dev`) and pass on it, so
// reuse is opt-in (OMNIMON_E2E_REUSE=1), in and out of CI. By default
// Playwright fails loudly when the port is taken.
const rawPort = process.env.OMNIMON_E2E_PORT ?? "1420";
const PORT = Number(rawPort);
if (!/^\d+$/.test(rawPort) || !Number.isInteger(PORT) || PORT < 1024 || PORT > 65535) {
  throw new Error(
    `OMNIMON_E2E_PORT must be an integer in 1024-65535, got ${JSON.stringify(rawPort)}`,
  );
}

export default defineConfig({
  testDir: "./e2e",
  testMatch: ["*.spec.ts"],
  timeout: 30_000,
  expect: {
    timeout: 5_000,
  },
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: [["list"], ["html"]],
  use: {
    baseURL: `http://localhost:${PORT}`,
    trace: "on-first-retry",
  },
  projects: [
    {
      name: "chromium",
      use: {
        browserName: "chromium",
      },
    },
  ],
  webServer: {
    // --port/--strictPort override vite.config.ts (1420, strictPort) so the
    // server listens exactly where Playwright waits.
    command: `bun run dev --host 127.0.0.1 --port ${PORT} --strictPort`,
    port: PORT,
    reuseExistingServer: process.env.OMNIMON_E2E_REUSE === "1",
  },
});
