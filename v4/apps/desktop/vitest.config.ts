import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { readFileSync } from "node:fs";

const pkg = JSON.parse(readFileSync("./package.json", "utf-8"));

export default defineConfig({
  plugins: [svelte()],
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
  resolve: {
    conditions: ["browser"],
  },
  test: {
    globals: true,
    // The CAPDESIS self-hosted runner can be heavily loaded under multi-repo
    // CI bursts, making render/waitFor-based component tests (e.g. App.test.ts)
    // run far slower than locally and exceed Vitest's 5s default. Generous
    // timeouts keep these tests deterministic under runner load without
    // weakening any assertion.
    testTimeout: 20000,
    hookTimeout: 20000,
    // The persistent public runner is shared with other build jobs. Serial
    // file execution prevents Svelte module transforms from being torn down
    // while another test file is still importing the same component graph.
    fileParallelism: false,
    environment: "happy-dom",
    setupFiles: ["src/test-setup.ts"],
    include: ["src/**/__tests__/**/*.test.ts"],
    coverage: {
      provider: "v8",
      include: ["src/**/*.{ts,svelte}"],
      thresholds: {
        lines: 85,
        functions: 85,
        statements: 85,
        branches: 70,
      },
      exclude: [
        "src/**/__tests__/**",
        "src/test-setup.ts",
        "src/main.ts",
        "src/vite-env.d.ts",
        "src/lib/types.ts",
        "src/App.svelte",
      ],
    },
  },
});
