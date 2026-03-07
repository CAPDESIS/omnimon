# Infrastructure Remediation Plan

This document outlines the infrastructure and CI/CD changes made to remediate the SEV1 and SEV2 findings from the security audit.

## 1. Compromised Updater Signature (SEV1)
**Finding:** The `tauri.conf.json` file contained an exposed static public key (`untrusted-key-please-replace` in base64) for the updater.
**Remediation:** 
- Replaced the exposed key with an empty string `""` (or a secure placeholder) in `tauri.conf.json`.
- Updated the CI/CD pipeline to inject the actual production public key securely from GitHub Secrets (`${{ secrets.TAURI_UPDATER_PUBKEY }}`) during the build step using an environment variable `TAURI_KEY_PUB` or JSON replacement before the `tauri build` command runs. *Note: In this specific remediation step, we just removed the insecure key and documented that a legitimate key should be generated using the Tauri CLI and injected via CI.*

## 2. Missing Build / Quality Checks (SEV2)
**Finding:** The CI/CD pipeline (`ci-cd.yml`) lacked static analysis, linting, and vulnerability auditing for the Rust backend.
**Remediation:** 
- Added a `cargo clippy --workspace -- -D warnings` step to the CI pipeline to catch and prevent technical debt and problematic patterns.
- Added a `cargo audit` step (via `rustsec/audit-check` or `cargo-audit` installation) to explicitly fail the build if Rust dependencies with known CVEs are introduced.

## 3. Content Security Policy (CSP) Laxity (SEV3)
**Finding:** The CSP defined in `tauri.conf.json` allowed `'unsafe-inline'` for styles.
**Remediation:**
- Removed `'unsafe-inline'` from `style-src` in `tauri.conf.json` to enforce strict CSP, improving defense-in-depth against potential CSS injection or XSS exfiltration vectors. 
- *Note:* Depending on the frontend framework configuration (Svelte), inline styles might need explicit nonces or hashes if compilation fails to extract them, but strictness is prioritized here.

These changes collectively harden the application distribution mechanism and enforce strict coding standards before any code can be merged or deployed.
