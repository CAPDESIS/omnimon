import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { existsSync } from "node:fs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, "..");
const workspaceDir = path.resolve(rootDir, "..", "..");

function tauriAppPath() {
  if (process.env.TAURI_E2E_APP_PATH) return process.env.TAURI_E2E_APP_PATH;

  if (process.platform === "darwin") {
    return path.join(workspaceDir, "target", "debug", "omnimon-desktop");
  }
  if (process.platform === "win32") {
    return path.join(workspaceDir, "target", "debug", "omnimon-desktop.exe");
  }
  return path.join(workspaceDir, "target", "debug", "omnimon-desktop");
}

let tauriDriverProcess;

export const config = {
  runner: "local",
  specs: [path.join(rootDir, "e2e", "specs", "**", "*.e2e.mjs")],
  maxInstances: 1,
  logLevel: "info",
  framework: "mocha",
  mochaOpts: {
    ui: "bdd",
    timeout: 120000,
  },
  reporters: ["spec"],
  hostname: "127.0.0.1",
  port: 4444,
  path: "/",
  capabilities: [
    {
      browserName: "tauri",
      "tauri:options": {
        application: tauriAppPath(),
      },
    },
  ],
  onPrepare: async () => {
    const app = tauriAppPath();
    if (!existsSync(app)) {
      throw new Error(`Tauri app binary not found at ${app}. Run npm run test:e2e:build first.`);
    }

    if (process.platform === "darwin") {
      const wkWebDriverInPath = process.env.PATH?.split(path.delimiter).some((p) =>
        existsSync(path.join(p, "wkwebdriver")),
      );
      if (!wkWebDriverInPath) {
        throw new Error(
          "wkwebdriver is required on macOS for tauri-driver. Install it and ensure it is in PATH.",
        );
      }
    }

    const tauriDriverBin = process.platform === "win32" ? "tauri-driver.cmd" : "tauri-driver";
    tauriDriverProcess = spawn(tauriDriverBin, ["--port", "4444"], {
      cwd: rootDir,
      stdio: "inherit",
      shell: process.platform === "win32",
    });

    await new Promise((resolve) => setTimeout(resolve, 2000));
  },
  onComplete: async () => {
    if (tauriDriverProcess && !tauriDriverProcess.killed) {
      tauriDriverProcess.kill("SIGTERM");
    }
  },
};
