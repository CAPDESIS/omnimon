import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { readFileSync } from "node:fs";

const host = process.env.TAURI_DEV_HOST;
const pkg = JSON.parse(readFileSync("./package.json", "utf-8"));

export default defineConfig(({ mode }) => ({
  plugins: [svelte()],
  clearScreen: false,
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
  },
  build: {
    rollupOptions: {
      output: {
        // Rolldown (Vite 8) no soporta `manualChunks` en forma de objeto;
        // se migra a `codeSplitting` con grupos equivalentes.
        codeSplitting: {
          groups: [
            { name: 'charts', test: /[\\/]node_modules[\\/]lightweight-charts[\\/]/ },
            { name: 'icons', test: /[\\/]node_modules[\\/]lucide-svelte[\\/]/ },
            { name: 'markdown', test: /[\\/]node_modules[\\/](marked|dompurify)[\\/]/ },
          ],
        },
      },
    },
  },
  esbuild: {
    drop: mode === 'production' ? ['console', 'debugger'] : [],
  },
}));
