# Final Infrastructure Remediation Plan (Hybrid CSP & CI Alignment)

This document tracks the final adjustments made to ensure stability and alignment prior to the release candidate.

## 1. Hybrid Content Security Policy (CSP)
**Adjustment:**
Restored `'unsafe-inline'` specifically for the `style-src` directive in `tauri.conf.json`.

**Rationale:**
The frontend relies heavily on dynamic, programmatic inline styles to drive components (e.g., RAM usage progress bars and severity color coding). A strict `style-src 'self'` prevents these inline mutations, degrading the visual user experience in production. By allowing `'unsafe-inline'` for styles but keeping `script-src` and `object-src` restricted to `'self'` (or strictly defined endpoints), we strike a practical balance. It mitigates the most critical threat vector (JavaScript XSS) while affording the UI team time to perform a deep refactoring of the design system to native CSS variables.

## 2. CI/CD Threshold Standardization
**Adjustment:**
Updated `.github/workflows/omnimon-ci.yml` to elevate the `cargo llvm-cov` line coverage gate from 80% to 85%.

**Rationale:**
This standardizes the threshold across all GitHub Actions workflows (`ci-cd.yml` already mandated 85%). This unified standard guarantees that PRs and main-branch commits adhere to the same rigorous test coverage bar, eliminating discrepancies between isolated CI runs and final release packaging pipelines.
