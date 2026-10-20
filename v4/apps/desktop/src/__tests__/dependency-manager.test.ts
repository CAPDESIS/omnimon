import { afterEach, describe, expect, it } from "vitest";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { dependencyManagerErrors, rustWorkspaceLockErrors } from "../../../../../scripts/dependency-manager";

const fixtures: string[] = [];
afterEach(() => {
  for (const fixture of fixtures.splice(0)) rmSync(fixture, { recursive: true, force: true });
});

describe("Rust workspace dependency graph", () => {
  it("uses the canonical workspace lock in this checkout", () => {
    expect(rustWorkspaceLockErrors(resolve(process.cwd(), "../.."))).toEqual([]);
  });

  it("rejects a child lock that would give auditors a second graph", () => {
    const workspace = fixture();
    writeFileSync(join(workspace, "Cargo.lock"), "# fixture");
    mkdirSync(join(workspace, "crates", "core"), { recursive: true });
    writeFileSync(join(workspace, "crates", "core", "Cargo.lock"), "# obsolete");
    expect(rustWorkspaceLockErrors(workspace)).toEqual([
      "Unexpected crates/core/Cargo.lock; core uses workspace Cargo.lock",
    ]);
  });

  it("requires the authoritative workspace lock", () => {
    expect(rustWorkspaceLockErrors(fixture())).toEqual(["Missing workspace Cargo.lock"]);
  });
});

function fixture(): string {
  const root = mkdtempSync(join(tmpdir(), "omnimon-lock-check-"));
  fixtures.push(root);
  const frontend = join(root, "desktop");
  mkdirSync(frontend, { mode: 0o700 });
  writeFileSync(join(frontend, "package.json"), "{}");
  writeFileSync(join(frontend, "bun.lock"), "{}");
  return frontend;
}

describe("desktop dependency graph", () => {
  it("uses only the authoritative Bun graph in this checkout", () => {
    expect(dependencyManagerErrors(resolve(process.cwd()))).toEqual([]);
  });

  it("accepts a complete Bun graph", () => {
    expect(dependencyManagerErrors(fixture())).toEqual([]);
  });

  it.each(["package-lock.json", "npm-shrinkwrap.json", "yarn.lock", "pnpm-lock.yaml"])(
    "rejects a competing %s even when Bun is present", (lockfile) => {
      const frontend = fixture();
      writeFileSync(join(frontend, lockfile), "{}");
      expect(dependencyManagerErrors(frontend)).toEqual([`Unexpected ${lockfile}; desktop uses bun.lock`]);
    },
  );

  it("fails when the authoritative lockfile is absent", () => {
    const frontend = fixture();
    rmSync(join(frontend, "bun.lock"));
    expect(dependencyManagerErrors(frontend)).toEqual(["Missing bun.lock"]);
  });
});
