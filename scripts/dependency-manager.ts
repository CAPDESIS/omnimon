import { existsSync } from "node:fs";
import { join } from "node:path";

/** Validate the graph used by frozen installs and vulnerability reports. */
export function dependencyManagerErrors(frontend: string): string[] {
  const errors: string[] = [];
  for (const required of ["package.json", "bun.lock"]) {
    if (!existsSync(join(frontend, required))) errors.push(`Missing ${required}`);
  }
  for (const obsolete of ["package-lock.json", "npm-shrinkwrap.json", "yarn.lock", "pnpm-lock.yaml"]) {
    if (existsSync(join(frontend, obsolete))) {
      errors.push(`Unexpected ${obsolete}; desktop uses bun.lock`);
    }
  }
  return errors;
}

/** Cargo members resolve from the root workspace lock, including core. */
export function rustWorkspaceLockErrors(workspace: string): string[] {
  const errors: string[] = [];
  if (!existsSync(join(workspace, "Cargo.lock"))) errors.push("Missing workspace Cargo.lock");
  if (existsSync(join(workspace, "crates", "core", "Cargo.lock"))) {
    errors.push("Unexpected crates/core/Cargo.lock; core uses workspace Cargo.lock");
  }
  return errors;
}
