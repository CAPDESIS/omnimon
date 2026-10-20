# Dependency audit validation

The desktop frontend uses `v4/apps/desktop/bun.lock`. Its existing CI installs
with `bun install --frozen-lockfile`. `package-lock.json` from commit
`7c9fd5f1473f04b76d9d06ec37e4afe0e327f043` described an older 6.5.0 dependency
graph, including wdio removed by #69; it was not installed by CI. It generated
the fleet's 50 reported HIGH findings on October 19. It is removed and ignored,
with the real checkout validated by `dependency-manager.test.ts` in the existing
Vitest suite. Reintroducing another manager's lock makes that test fail.

The Rust workspace uses `v4/Cargo.lock` for all four members. The obsolete
`v4/crates/core/Cargo.lock` is also removed and ignored. Running
`cargo metadata --no-deps --locked --offline --format-version=1 --manifest-path
v4/crates/core/Cargo.toml` confirms `workspace_root=v4`. The obsolete child
graph locked `crossbeam-epoch` 0.9.18 affected by
[RUSTSEC-2026-0204](https://rustsec.org/advisories/RUSTSEC-2026-0204.html), fixed
in 0.9.20; the unchanged active workspace lock already resolves 0.9.21.
No Rust dependency was upgraded or hidden from auditing. The existing Vitest
gate also requires the workspace lock and rejects the child lock.

`bash scripts/cve-report.sh [--json]` audits both the complete Bun graph and
`v4/Cargo.lock`, without installing tools or changing dependency resolutions.
Bun's report retains every returned advisory; HIGH/CRITICAL findings fail the
report. Every vulnerability reported by `cargo audit` fails it. No dev or
optional dependency groups are omitted.

| Exit | Consolidated status | Meaning |
| --- | --- | --- |
| 0 | `clean` | Both audits completed with no findings meeting this policy. |
| 1 | `findings` | At least one audit found a qualifying advisory. |
| 2 | `infra` | An audit is missing or failed, its output is invalid, or a required/competing lockfile invalidates the graph. |

`--json` emits one JSON object, with `bun_audit` and `rust_audit`; the obsolete
`npm_audit` field is retired. Reports are saved in `reports/` with unique
timestamps. Newly created report directories use mode 700 and report files
use mode 600. `--report-dir` selects another output directory; an existing
directory keeps its permissions, so choose a private directory before running
the command. Raw tool stderr is not exposed. Infrastructure errors retry at most
three attempts (`--attempts 1..3`); findings do not retry. A failed audit takes
precedence over a finding so a partially measured graph is never called clean.

Validation commands:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/test_cve_report.py -v
cd v4/apps/desktop
bun install --frozen-lockfile
bun run test -- src/__tests__/dependency-manager.test.ts
bun run build
bun audit --json
```

The report tests launch real fixture processes and check the CLI exit, saved
JSON and status for clean graphs, HIGH and moderate findings, missing tools or
locks, a competing lock, malformed/network responses and inconsistent exits.
The previous reporter returned 0 when neither auditor was installed; the new
reporter returns 2 with both audits marked `infra`. Restoring the obsolete npm
lock makes the checkout test fail; removing it makes the same test pass.

Measured on October 19, 2026 against main's unchanged Bun lock: `bun audit
--json` exited 0 with `{}`; OSV Scanner 2.6.0 returned `results: []` (270
packages). The new consolidated report ran actual Bun and cargo-audit and
returned 0 with both audits `clean`. These are local measurements; use current
CI and commit evidence to establish the state of main or a deployment.

The expanded lock guard measured 10 passing tests. Before removing the child
Cargo lock, the same suite had one failing test and nine passing tests. The
earlier full frontend suite measured 48 files and 722 passing tests; the Vite
build also passed. No Rust source, manifest or active Cargo lock changed in
this remediation, so Rust compilation results are not claimed here.
