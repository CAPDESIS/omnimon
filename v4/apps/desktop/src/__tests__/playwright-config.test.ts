import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Contract for the e2e webServer port and reuse.
//
// `reuseExistingServer: !process.env.CI` made a local run test whatever
// already listened on 1420 (another checkout, a running `tauri dev`) and pass
// on a build that was not the one under test. Each case re-imports the REAL
// playwright.config.ts with its own env; only `defineConfig` is replaced by an
// identity function so the resolved object can be inspected.
vi.mock("@playwright/test", () => ({ defineConfig: (config: unknown) => config }));

const OWN_VARS = ["CI", "OMNIMON_E2E_PORT", "OMNIMON_E2E_REUSE"];

type Resolved = {
  use: { baseURL: string };
  webServer: { command: string; port: number; reuseExistingServer: boolean };
};

async function loadConfig(env: Record<string, string> = {}): Promise<Resolved> {
  vi.resetModules();
  for (const name of OWN_VARS) vi.stubEnv(name, undefined);
  for (const [name, value] of Object.entries(env)) vi.stubEnv(name, value);
  return (await import("../../playwright.config")).default as unknown as Resolved;
}

describe("playwright e2e webServer", () => {
  beforeEach(() => {
    vi.resetModules();
  });
  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it("defaults to 1420 and does not reuse a foreign server", async () => {
    const config = await loadConfig();
    expect(config.use.baseURL).toBe("http://localhost:1420");
    expect(config.webServer.port).toBe(1420);
    expect(config.webServer.command).toBe(
      "bun run dev --host 127.0.0.1 --port 1420 --strictPort",
    );
    expect(config.webServer.reuseExistingServer).toBe(false);
  });

  it("never reuses a server in CI either", async () => {
    const config = await loadConfig({ CI: "true" });
    expect(config.webServer.reuseExistingServer).toBe(false);
  });

  it("follows OMNIMON_E2E_PORT in baseURL, wait port and server command", async () => {
    const config = await loadConfig({ OMNIMON_E2E_PORT: "50123" });
    expect(config.use.baseURL).toBe("http://localhost:50123");
    expect(config.webServer.port).toBe(50123);
    expect(config.webServer.command).toBe(
      "bun run dev --host 127.0.0.1 --port 50123 --strictPort",
    );
    expect(config.webServer.reuseExistingServer).toBe(false);
  });

  it("reuses an existing server only when OMNIMON_E2E_REUSE=1, in or out of CI", async () => {
    for (const ci of [{}, { CI: "true" }]) {
      const on = await loadConfig({ ...ci, OMNIMON_E2E_REUSE: "1" });
      expect(on.webServer.reuseExistingServer).toBe(true);
      for (const flag of ["0", "true", "yes", ""]) {
        const off = await loadConfig({ ...ci, OMNIMON_E2E_REUSE: flag });
        expect(off.webServer.reuseExistingServer).toBe(false);
      }
    }
  });

  it("rejects a port that is not a valid unprivileged integer", async () => {
    for (const bad of ["abc", "80", "70000", "1420x", ""]) {
      await expect(loadConfig({ OMNIMON_E2E_PORT: bad })).rejects.toThrow("OMNIMON_E2E_PORT");
    }
  });
});
