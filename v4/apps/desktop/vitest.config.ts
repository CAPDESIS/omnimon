import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { readFileSync } from "node:fs";

const pkg = JSON.parse(readFileSync("./package.json", "utf-8"));

export default defineConfig({
  plugins: [svelte({ hot: false })],
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
  resolve: {
    conditions: ["browser"],
  },
  test: {
    globals: true,
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
        branches: 72,
      },
      exclude: [
        "src/**/__tests__/**",
        "src/test-setup.ts",
        "src/main.ts",
        "src/vite-env.d.ts",
        "src/lib/types.ts",
        "src/App.svelte",
        "src/components/StatusBar.svelte",
      ],
    },
  },
});
