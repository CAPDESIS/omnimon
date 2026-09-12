# OmniMon CVE Report

Date: 2026-09-11
Command: `cargo audit` (CI job in `.github/workflows/omnimon-ci.yml`) plus lockfile inspection
Workspace: `v4/`
Lockfile SoT: `v4/Cargo.lock`

## Executive Summary

- `CVE-2026-25727` / `RUSTSEC-2026-0009` in `time 0.3.45` is **mitigated in the current lockfile**.
- Installed `time` version: **0.3.54** (patched floor is `>= 0.3.47`).
- `RUSTSEC-2026-0258` in `h2 0.4.15` was present on 2026-09-11; lockfile bumped to **`h2 0.4.16`**.
- Informational `unmaintained` / `unsound` warnings may still appear from the GTK3/Linux desktop stack; they are not counted as confirmed CVEs.
- Re-run `cargo audit` in `v4/` before claiming a clean advisory set; this file tracks lockfile evidence plus the 2026-09-11 scan.

## Confirmed CVEs

| CVE | RustSec | Affected crate | Lockfile version | Patched version | Status |
| --- | --- | --- | --- | --- | --- |
| `CVE-2026-25727` | `RUSTSEC-2026-0009` | `time` | `0.3.54` | `>= 0.3.47` | Mitigated in lockfile |
| `RUSTSEC-2026-0258` | `RUSTSEC-2026-0258` | `h2` | `0.4.16` | `>= 0.4.16` | Mitigated in lockfile (was 0.4.15) |

## Vulnerability Detail

### `CVE-2026-25727` / `RUSTSEC-2026-0009`

- Crate: `time`
- Historical report (2026-03-08): `0.3.45` pending, blocked by an older `mac-notification-sys` pin
- Current lockfile: `0.3.54` (`v4/Cargo.lock` package `time`)
- Fixed version: `0.3.47` or newer
- Risk summary: specially crafted RFC 2822 date input can cause stack exhaustion
- Status: **mitigated** in this workspace lock. Do not `cargo update -p time` solely for this CVE unless `cargo audit` flags it again.

## Informational Warnings

These were reported by `cargo audit`, but they are not counted as confirmed CVEs:

- GTK3 bindings marked unmaintained: `atk`, `atk-sys`, `gdk`, `gdk-sys`, `gdkwayland-sys`, `gdkx11`, `gdkx11-sys`, `gtk`, `gtk-sys`, `gtk3-macros`
- Other unmaintained crates: `fxhash`, `paste`, `proc-macro-error`, `unic-char-property`, `unic-char-range`, `unic-common`, `unic-ucd-ident`, `unic-ucd-version`
- Unsoundness advisories: `glib 0.18.5`, `lru 0.12.5`

## Recommended Remediation Order

1. Patch `time` to `0.3.47+` and rerun `cargo audit`.
2. Review whether GTK3-linked crates are only present on Linux desktop paths and plan migration to maintained GTK4-era dependencies where feasible.
3. Review `lru` and `glib` transitive pins, especially for Linux desktop builds.
