import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { existsSync } from "node:fs";
import { waitTauriDriverReady } from "@crabnebula/tauri-driver";
import { waitTestRunnerBackendReady } from "@crabnebula/test-runner-backend";

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
let testRunnerBackendProcess;
let killedTauriDriver = false;
let killedTestRunnerBackend = false;

function closeDrivers() {
  killedTauriDriver = true;
  killedTestRunnerBackend = true;
  if (tauriDriverProcess && !tauriDriverProcess.killed) tauriDriverProcess.kill("SIGTERM");
  if (testRunnerBackendProcess && !testRunnerBackendProcess.killed)
    testRunnerBackendProcess.kill("SIGTERM");
}

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
      if (!process.env.CN_API_KEY) {
        throw new Error(
          "CN_API_KEY is required on macOS. Export it to use CrabNebula Webdriver backend.",
        );
      }

      const backendBin = process.platform === "win32" ? "test-runner-backend.cmd" : "test-runner-backend";
      testRunnerBackendProcess = spawn(backendBin, [], {
        cwd: rootDir,
        stdio: "inherit",
        shell: process.platform === "win32",
      });

      testRunnerBackendProcess.on("exit", (code) => {
        if (!killedTestRunnerBackend) {
          console.error(`test-runner-backend exited unexpectedly with code ${code}`);
          process.exit(1);
        }
      });

      await waitTestRunnerBackendReady();
      process.env.REMOTE_WEBDRIVER_URL = "http://127.0.0.1:3000";
    }
  },
  beforeSession: async () => {
    const tauriDriverBin = process.platform === "win32" ? "tauri-driver.cmd" : "tauri-driver";
    tauriDriverProcess = spawn(tauriDriverBin, ["--port", "4444"], {
      cwd: rootDir,
      stdio: "inherit",
      shell: process.platform === "win32",
    });

    tauriDriverProcess.on("exit", (code) => {
      if (!killedTauriDriver) {
        console.error(`tauri-driver exited unexpectedly with code ${code}`);
        process.exit(1);
      }
    });

    await waitTauriDriverReady();
  },
  afterSession: async () => {
    closeDrivers();
  },
  onComplete: async () => {
    closeDrivers();
  },
};
