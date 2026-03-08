# Optimization & Infrastructure Hardening

## Overview
This document outlines the tactical DevSecOps hardening (P0/P1) patches applied to OmniMon v4 to enhance IPC security, prevent secret leaks, and optimize binary compilation.

## 1. Tauri IPC Scopes Hardening
**File:** `v4/apps/desktop/src-tauri/capabilities/default.json`
- Granular scopes were introduced to prevent arbitrary execution or file system access in the event of an XSS attack.
- Restricted `shell:allow-open` to only permit URLs matching `https://github.com/chochy2001/omnimon/*`.
- Restricted `store:allow-*` (`get`, `set`, `save`, `load`) to restrict read/write access strictly to the `$APPDATA/omnimon/*` directory.
- **Dependency Cleanup:** Removed `@tauri-apps/plugin-shell` from `v4/apps/desktop/package.json` as it was not imported in the frontend codebase.

## 2. Secret Leak Prevention
**Files:** `.gitignore`, `.github/workflows/omnimon-ci.yml`
- Appended global exclusion patterns for common secrets (`.env`, `.env.local`, `*.pem`, `*.p12`, `*.key`, `secrets.json`) to `.gitignore`.
- Integrated `trufflesecurity/trufflehog@main` into the CI/CD pipeline (`omnimon-ci.yml`) under the `security` job to actively scan for unverified secrets on every commit and Pull Request.

## 3. Binary Compaction
**File:** `v4/Cargo.toml`
- Updated the release profile `opt-level` from `"s"` to `"z"`. 
- This instructs `rustc` and LLVM to optimize the binary for the absolute smallest file size possible, which concurrently aids in hardening the binary against reverse-engineering by compressing the compiled output further.
