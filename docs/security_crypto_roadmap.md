# OmniMon Security & Cryptography Roadmap

## Architecture Overview

### 1. Release Integrity (NIST SP 800-186 / FIPS 186-5)

**Ed25519 Digital Signatures:**
- Key pair generation: `Ed25519` via `ed25519-dalek` crate (RFC 8032)
- The CI/CD pipeline signs each release artifact with the **private key** (stored as `ED25519_SIGNING_KEY` GitHub Secret)
- The **public key** is embedded in the application binary for offline verification
- Signature format: Base64-encoded 64-byte Ed25519 signature

**SHA-256 Integrity Hashes:**
- Every release artifact produces a `.sha256` checksum file
- SHA-256 is computed using the `sha2` crate (NIST FIPS 180-4)
- Checksums are published alongside release binaries

**Update Verification Flow:**
```
1. Client downloads UpdateManifest (version, sha256, signature_b64, download_url)
2. Client downloads the artifact from download_url
3. verify_update() performs:
   a. SHA-256(downloaded_bytes) == manifest.sha256  (integrity)
   b. Ed25519.verify(downloaded_bytes, signature, EMBEDDED_PUBLIC_KEY)  (authenticity)
4. BOTH checks MUST pass before the update is applied
```

### 2. MITRE ATT&CK Alignment

| Detection | MITRE ID | Tactic | Implementation |
|-----------|----------|--------|----------------|
| DLL Injection | T1055.001 | Defense Evasion | `security.rs::map_behavior_to_mitre` |
| Remote Thread Injection | T1055.003 | Defense Evasion | Process behavior analysis |
| Process Hollowing | T1055.012 | Defense Evasion | Memory pattern detection |
| Credential Dumping | T1003 | Credential Access | Suspicious memory read monitoring |
| Hijack Execution Flow | T1574 | Persistence | Unsigned module load detection |
| C2 Communication | T1043, T1571 | Command & Control | Network policy enforcement |
| Command Injection | T1059 | Execution | Native FFI (no shell commands) |
| Unsecured Credentials | T1552 | Credential Access | OS Keychain integration |
| UAC Bypass | T1548.002 | Privilege Escalation | User-space only operation |

### 3. NIST Cybersecurity Framework (CSF) Mapping

| Function | Category | OmniMon Control |
|----------|----------|-----------------|
| **Identify** | Asset Management | Process inventory, network connection tracking |
| **Protect** | Data Security | AES-256-GCM encrypted audit trails, OS keychain |
| **Detect** | Anomalies & Events | Real-time behavioral analysis, MITRE mapping |
| **Respond** | Analysis | Threat labeling with confidence scores |
| **Recover** | Recovery Planning | Encrypted audit trail for forensics |

### 4. CI/CD Hardening

**Coverage Requirements:**
- Rust backend: >= 85% line coverage (enforced via `cargo-llvm-cov --fail-under-lines 85`)
- Frontend: >= 85% statement coverage (enforced via Vitest `--coverage`)
- All three platforms: macOS, Linux, Windows

**CVE Scanning Pipeline:**
- `cargo audit` for Rust dependency CVEs (RustSec Advisory DB)
- `npm audit --audit-level=high` for Node.js dependency CVEs
- TruffleHog for leaked secrets/credentials
- `scripts/cve-report.sh` generates consolidated vulnerability reports

**Release Signing Pipeline:**
1. CI runs all tests + security audit
2. `tauri-action` builds platform-specific artifacts
3. Post-build step computes SHA-256 and signs with Ed25519
4. Signature manifest (`.sig.json`) uploaded as release asset
5. Rename step applies user-friendly names

**CrabNebula Distribution:**
- `CN_API_KEY` required for CrabNebula Cloud distribution
- If unavailable or invalid, fallback to GitHub Releases (free tier)
- Both paths use the same Ed25519 signing infrastructure

### 5. Code Quality Auditing

**Rust:**
- `cargo fmt --check` - formatting consistency
- `cargo clippy --workspace -- -D warnings` - lint enforcement
- `cargo audit` - dependency vulnerability scanning

**Frontend (Svelte/TypeScript):**
- Vitest with coverage thresholds
- `vite build` must succeed without errors

### 6. Implementation Files

| File | Purpose |
|------|---------|
| `v4/crates/core/src/crypto.rs` | Ed25519 signatures, SHA-256, AES-256-GCM |
| `v4/crates/core/src/security.rs` | MITRE ATT&CK mapping, threat labeling |
| `v4/crates/core/src/audit_trail.rs` | Encrypted forensic audit trail |
| `.github/workflows/omnimon-ci.yml` | CI/CD with signing, coverage, CVE scan |
| `scripts/sign-release.sh` | Release signing utility |
| `scripts/cve-report.sh` | CVE vulnerability report generator |
