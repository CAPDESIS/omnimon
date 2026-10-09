import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

// Regression pin for the Fleet CVE watch (2026-10-08): bun never floats a
// locked transitive while its pin satisfies parent ranges, so stale HIGH
// pins linger until a refresh. This test fails if bun.lock drops back to a
// known-vulnerable pin. Floors are the OSV fixed versions per major line
// present in the tree; the fleet cve-watch remains the gate for new lines.
// Excepted advisories (no compatible fix) are NOT pinned here:
// extract-zip (no fixed release), serialize-javascript GHSA-5c6j (mocha
// pins 6.x including v12), basic-ftp GHSA-c475 (fix only in 6.x, get-uri
// pins 5.x including v8). They live in ci-nodes cve-exceptions.tsv.

type Triple = [number, number, number];

const FLOORS: Record<string, Record<number, Triple>> = {
  "basic-ftp": { 5: [5, 3, 1] },
  "brace-expansion": { 1: [1, 1, 20], 2: [2, 1, 6], 5: [5, 0, 11] },
  "deepmerge-ts": { 8: [8, 0, 0] },
  "fast-xml-builder": { 1: [1, 1, 7] },
  "fast-xml-parser": { 5: [5, 5, 6] },
  "ip-address": { 10: [10, 3, 1] },
  "js-yaml": { 4: [4, 3, 2] },
  "lodash": { 4: [4, 18, 0] },
  nanoid: { 3: [3, 3, 18] },
  picomatch: { 2: [2, 3, 2], 4: [4, 0, 4] },
  postcss: { 8: [8, 5, 18] },
  "source-map-js": { 1: [1, 2, 2] },
  undici: { 6: [6, 28, 1], 7: [7, 29, 1] },
};

function parseVersion(raw: string): Triple | null {
  const core = raw.split("-")[0].split("+")[0];
  const parts = core.split(".");
  if (parts.length !== 3) return null;
  const nums = parts.map((p) => Number(p));
  if (nums.some((n) => !Number.isInteger(n) || n < 0)) return null;
  return nums as Triple;
}

function atLeast(version: Triple, floor: Triple): boolean {
  for (let i = 0; i < 3; i++) {
    if (version[i] !== floor[i]) return version[i] > floor[i];
  }
  return true;
}

function lockedCopies(): Array<{ name: string; version: string }> {
  const here = dirname(fileURLToPath(import.meta.url));
  const lockPath = resolve(here, "..", "..", "bun.lock");
  const text = readFileSync(lockPath, "utf-8").replace(/,(\s*[}\]])/g, "$1");
  const lock = JSON.parse(text) as {
    packages: Record<string, [string, ...unknown[]]>;
  };
  const out: Array<{ name: string; version: string }> = [];
  for (const entry of Object.values(lock.packages)) {
    const resolved = entry[0];
    const at = resolved.lastIndexOf("@");
    // Resolved ids look like "name@x.y.z" (scoped: "@scope/name@x.y.z").
    const name = resolved.slice(0, at);
    const version = resolved.slice(at + 1);
    out.push({ name, version });
  }
  return out;
}

describe("bun.lock CVE floors", () => {
  it("keeps every pinned package at or past its fixed floor", () => {
    const failures: string[] = [];
    for (const { name, version } of lockedCopies()) {
      const lines = FLOORS[name];
      if (!lines) continue;
      const parsed = parseVersion(version);
      if (!parsed) {
        failures.push(`${name}@${version}: unparseable version`);
        continue;
      }
      const floor = lines[parsed[0]];
      if (!floor) continue; // Unknown major line: cve-watch owns it.
      if (!atLeast(parsed, floor)) {
        failures.push(
          `${name}@${version} is below floor ${floor.join(".")} (regressed pin)`,
        );
      }
    }
    expect(failures).toEqual([]);
  });
});
