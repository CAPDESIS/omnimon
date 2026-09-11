# OmniMon CVE Report

Date: 2026-09-11
Command: lockfile inspection of `v4/Cargo.lock` plus CI `cargo audit`
Workspace: `v4/`

## Executive Summary

- `CVE-2026-25727` in `time 0.3.45` is **mitigated**. Lockfile has `time 0.3.54` (`>= 0.3.47`).
- `RUSTSEC-2026-0258` (`h2 0.4.15`) bumped to `h2 0.4.16` on 2026-09-11.
- GTK3/Linux `unmaintained`/`unsound` warnings may still appear; they are not confirmed CVEs.
- This copy mirrors the root `CVE_REPORT.md`. Re-run `cargo audit` in `v4/` for live advisories.

## Confirmed CVEs

| CVE | RustSec | Affected crate | Lockfile version | Patched version | Status |
| --- | --- | --- | --- | --- | --- |
| `CVE-2026-25727` | `RUSTSEC-2026-0009` | `time` | `0.3.54` | `>= 0.3.47` | Mitigated in lockfile |
| `RUSTSEC-2026-0258` | `RUSTSEC-2026-0258` | `h2` | `0.4.16` | `>= 0.4.16` | Mitigated in lockfile |

## Informational Warnings

These were reported by `cargo audit`, but they are not counted as confirmed CVEs:

- GTK3 bindings marked unmaintained: `atk`, `atk-sys`, `gdk`, `gdk-sys`, `gdkwayland-sys`, `gdkx11`, `gdkx11-sys`, `gtk`, `gtk-sys`, `gtk3-macros`
- Other unmaintained crates: `fxhash`, `paste`, `proc-macro-error`, `unic-char-property`, `unic-char-range`, `unic-common`, `unic-ucd-ident`, `unic-ucd-version`
- Unsoundness advisories: `glib 0.18.5`, `lru 0.12.5`

## Recommended Remediation Order

1. Patch `time` to `0.3.47+` and rerun `cargo audit`.
2. Review whether GTK3-linked crates are only present on Linux desktop paths and plan migration to maintained GTK4-era dependencies where feasible.
3. Review `lru` and `glib` transitive pins, especially for Linux desktop builds.
