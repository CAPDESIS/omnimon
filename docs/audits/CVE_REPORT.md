# OmniMon CVE Report

Date: 2026-03-08
Command: `cargo audit --json`
Workspace: `v4/`

## Executive Summary

- Rust dependency vulnerabilities found: `1`
- Informational dependency warnings also present: `unmaintained` and `unsound` crates in the GTK3/Linux desktop stack and related transitive dependencies
- Highest confirmed actionable item in this audit: `CVE-2026-25727` in `time 0.3.45`

## Confirmed CVEs

| CVE | RustSec | CVSS | Affected crate | Current version | Patched version | Risk | Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `CVE-2026-25727` | `RUSTSEC-2026-0009` | High availability impact (`CVSS:4.0/AV:N/AC:H/AT:N/PR:L/UI:A/VC:N/VI:N/VA:H/SC:N/SI:N/SA:H`) | `time` | `0.3.45` | `>= 0.3.47` | RFC 2822 parsing can trigger stack exhaustion and denial of service with malicious input | Pending |

## Vulnerability Detail

### `CVE-2026-25727` / `RUSTSEC-2026-0009`

- Crate: `time`
- Installed version: `0.3.45`
- Fixed version: `0.3.47` or newer
- Risk summary: specially crafted RFC 2822 date input can cause stack exhaustion and crash the parsing process.
- Likely impact to OmniMon: low-to-moderate unless untrusted RFC 2822 date parsing is reachable in runtime paths; still should be patched because it is present in the shipped dependency graph.
- Proposed remediation:
  - run `cargo update -p time --precise 0.3.47` or newer,
  - rebuild and rerun `cargo audit`,
  - validate no transitive crate pins an older incompatible `time` version.
- Status: not patched in this branch.

## Informational Warnings

These were reported by `cargo audit`, but they are not counted as confirmed CVEs:

- GTK3 bindings marked unmaintained: `atk`, `atk-sys`, `gdk`, `gdk-sys`, `gdkwayland-sys`, `gdkx11`, `gdkx11-sys`, `gtk`, `gtk-sys`, `gtk3-macros`
- Other unmaintained crates: `fxhash`, `paste`, `proc-macro-error`, `unic-char-property`, `unic-char-range`, `unic-common`, `unic-ucd-ident`, `unic-ucd-version`
- Unsoundness advisories: `glib 0.18.5`, `lru 0.12.5`

## Recommended Remediation Order

1. Patch `time` to `0.3.47+` and rerun `cargo audit`.
2. Review whether GTK3-linked crates are only present on Linux desktop paths and plan migration to maintained GTK4-era dependencies where feasible.
3. Review `lru` and `glib` transitive pins, especially for Linux desktop builds.
