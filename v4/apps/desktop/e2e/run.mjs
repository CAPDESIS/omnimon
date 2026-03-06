import { spawnSync } from "node:child_process";

function run(cmd, args) {
  const result = spawnSync(cmd, args, { stdio: "inherit", shell: process.platform === "win32" });
  return result.status ?? 1;
}

const strict = process.env.E2E_STRICT === "1";

if (process.platform === "darwin" && !process.env.CN_API_KEY) {
  const msg = [
    "[e2e] Skipping on macOS: CN_API_KEY is not set.",
    "[e2e] To run full Tauri E2E on macOS:",
    "       export CN_API_KEY=\"<your-crabnebula-key>\"",
    "       npm run test:e2e",
  ].join("\n");

  if (strict) {
    console.error(msg);
    process.exit(1);
  }

  console.log(msg);
  process.exit(0);
}

let code = run("npm", ["run", "test:e2e:build"]);
if (code !== 0) process.exit(code);

code = run("npx", ["wdio", "run", "./e2e/wdio.conf.mjs"]);
process.exit(code);
