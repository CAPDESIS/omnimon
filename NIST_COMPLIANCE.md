# OmniMon NIST SP 800-53 Compliance Matrix

Date: 2026-03-08
Assessment basis: source review of `v4/` plus current configuration and `cargo audit` output.

## Summary

- OmniMon already implements meaningful controls around encryption, secure credential storage, telemetry, rate limiting, and audit logging.
- Coverage is strongest in system monitoring and application-layer safeguards.
- Main gaps are policy/process controls: formal access control, retention policy, centralized logging, SBOM/change-management evidence, and automated version consistency checks.

## Control Matrix

| Control | Status | Evidence | Notes |
| --- | --- | --- | --- |
| `AU-2` Event Logging | Partial | `v4/crates/core/src/audit_trail.rs` | Security-relevant events can be recorded, but logging coverage is not yet universal across all commands |
| `AU-3` Content of Audit Records | Partial | `MitreAlertRecord` stores timestamp, pid, process name, confidence, technique IDs | Good record structure; missing operator identity/session context |
| `AU-9` Protection of Audit Information | Implemented | Audit trail records are encrypted with AES-256-GCM in `v4/crates/core/src/crypto.rs` and `v4/crates/core/src/audit_trail.rs` | Tamper resistance is good locally, but external integrity/centralization is not documented |
| `AU-12` Audit Generation | Partial | Audit trail append helpers exist | Needs broader integration with CLI, IPC, and admin actions |
| `CA-7` Continuous Monitoring | Partial | Background watcher, process telemetry, network telemetry, automation engine | Strong local monitoring; no documented central continuous-monitoring workflow |
| `CM-8` System Component Inventory | Partial | `get_metrics`, watcher cache, plugin registry, browser tab inventory | Practical runtime inventory exists, but no formal asset inventory export policy |
| `IA-5` Authenticator Management | Partial | AI keys and cloud keys stored in native keyring (`keyring`) | No documented rotation/expiration enforcement |
| `IR-4` Incident Handling | Partial | Security scan, alerts, automations, encrypted report generation | Incident workflow exists technically but lacks documented runbooks |
| `SC-7` Boundary Protection | Partial | Tauri CSP in `tauri.conf.json`; IPC rate limiting in `v4/crates/core/src/rate_limit.rs` | Helps constrain frontend and IPC abuse; not a full network boundary control |
| `SC-12` Cryptographic Key Establishment and Management | Partial | Ed25519/AES helpers in `v4/crates/core/src/crypto.rs`; keyring-backed secrets | Strong primitives; formal key lifecycle management still missing |
| `SC-13` Cryptographic Protection | Implemented | AES-256-GCM, Ed25519, SHA-256 | Clear code evidence for crypto usage |
| `SC-28` Protection of Information at Rest | Implemented | Encrypted audit logs and encrypted security heartbeat persistence | Local at-rest protection is present |
| `SI-3` Malicious Code Protection | Partial | CVE scan support, MITRE mappings, process threat labeling | Useful detections, but no malware-signature or quarantine workflow |
| `SI-4` System Monitoring | Implemented | watcher, network data, process metrics, dynamic rule alerts | Core product capability |
| `SI-10` Information Input Validation | Partial | Runtime IPC validation in `v4/apps/desktop/src/lib/ipc.ts`, prompt-injection checks in `v4/crates/core/src/ai.rs` | Good app-level validation; broader schema enforcement could improve coverage |

## Implemented Controls

- Secure credential storage through native OS keyrings.
- IPC rate limiting for destructive, browser, AI, and configuration actions.
- Encrypted audit trail and encrypted security heartbeat persistence.
- Cryptographic integrity helpers using AES-256-GCM, Ed25519, and SHA-256.
- Runtime validation for Tauri IPC responses in the frontend.
- Content Security Policy in Tauri configuration.
- Continuous local monitoring of processes, tabs, and network activity.

## Missing or Weak Controls

- No documented retention policy or centralized collection for audit records.
- No formal RBAC or operator identity model for privileged actions.
- No documented SBOM, signed provenance, or release attestation process in this audit pass.
- No automated CI guard that enforces version consistency across Cargo, Tauri, package, and visible UI labels.
- Incident response and vulnerability remediation are not yet captured as operational runbooks.

## Recommended Next Steps

1. Add a release/compliance CI job for version parity, `cargo audit`, and artifact metadata verification.
2. Extend audit logging to CLI and IPC admin/destructive actions with actor/context metadata.
3. Publish a lightweight security operations guide covering key rotation, report retention, and incident handling.
4. Generate an SBOM for desktop releases and track dependency exceptions explicitly.
