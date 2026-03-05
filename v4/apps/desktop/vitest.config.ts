import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte({ hot: false })],
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
