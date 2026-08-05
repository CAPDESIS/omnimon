# Final Infrastructure Remediation Plan (Hybrid CSP & CI Alignment)

This document tracks the final adjustments made to ensure stability and alignment prior to the release candidate.

## 1. Hybrid Content Security Policy (CSP)
**Adjustment:**
Restored `'unsafe-inline'` specifically for the `style-src` directive in `tauri.conf.json`.

**Rationale:**
The frontend relies heavily on dynamic, programmatic inline styles to drive components (e.g., RAM usage progress bars and severity color coding). A strict `style-src 'self'` prevents these inline mutations, degrading the visual user experience in production. By allowing `'unsafe-inline'` for styles but keeping `script-src` and `object-src` restricted to `'self'` (or strictly defined endpoints), we strike a practical balance. It mitigates the most critical threat vector (JavaScript XSS) while affording the UI team time to perform a deep refactoring of the design system to native CSS variables.

## 2. CI/CD Threshold Standardization
**Current status (2026-06-29):**
The current `.github/workflows/omnimon-ci.yml` gates frontend line coverage at
75% and Rust line coverage at 70%. Linux is blocking; macOS and Windows are
non-blocking experimental jobs.

The older 85% statement in this document is superseded by the workflow source of
truth. Raising either threshold should be done in the workflow and then
revalidated locally with `bun run test:coverage` and `cargo llvm-cov`.
