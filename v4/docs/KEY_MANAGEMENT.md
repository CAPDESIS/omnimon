# OmniMon Key Management Policy (NIST SC-12)

This document describes how OmniMon manages cryptographic key lifecycle
in compliance with NIST SP 800-57 / SC-12 controls.

## Key Lifecycle

### 1. Generation
- All symmetric keys are generated using the OS CSPRNG via `rand::thread_rng()` (backed by `getrandom`).
- Ed25519 signing keys use `SigningKey::generate()` with the same CSPRNG.
- Minimum key size: 256-bit (AES-256-GCM).

### 2. Derivation
- Application-level keys are derived from a master key using **HKDF-SHA256** (RFC 5869).
- Domain separation via context strings prevents key reuse across different purposes:
  - `omnimon-data-encryption` — local data at rest
  - `omnimon-api-key-storage` — API key wrapping
  - `omnimon-config-encryption` — configuration encryption
- Implementation: `core::crypto::derive_key()`

### 3. Storage
- All secret key material is stored in the **OS native keyring**:
  - macOS: Keychain (via `keyring` crate with `apple-native` feature)
  - Windows: Credential Manager (`windows-native`)
  - Linux: Secret Service / libsecret (`linux-native`)
- Keys are never written to disk in plaintext.
- Each service has a unique keyring entry (e.g., `omnimon_openai`, `omnimon_security`).

### 4. Usage
- Keys are loaded from the keyring into memory only when needed.
- In-memory keys are wrapped in `DerivedKey` (implements `ZeroizeOnDrop`), ensuring
  automatic zeroization when the key goes out of scope.
- API keys retrieved from the keyring are used transiently in HTTP request headers.

### 5. Rotation
- Key rotation is available via `omnimon config rotate-key`.
- Rotation process:
  1. Generate a new 256-bit master key via CSPRNG.
  2. Re-derive encryption keys using HKDF with new master.
  3. Store the new master key in the OS keyring.
  4. The old key is zeroized automatically (ZeroizeOnDrop).
- Recommended rotation interval: when a key compromise is suspected, or periodically
  per organizational policy.

### 6. Destruction
- All key types implement `Zeroize` and `ZeroizeOnDrop` from the `zeroize` crate.
- Keys are overwritten with zeros before deallocation.
- OS keyring entries can be deleted via `keyring::Entry::delete_credential()`.

## Algorithms

| Purpose              | Algorithm        | Key Size | Standard           |
|----------------------|------------------|----------|--------------------|
| Data encryption      | AES-256-GCM      | 256-bit  | NIST SP 800-38D    |
| Key derivation       | HKDF-SHA256      | 256-bit  | RFC 5869 / SP 800-56C |
| Integrity/signing    | Ed25519          | 256-bit  | FIPS 186-5         |
| Hashing              | SHA-256          | 256-bit  | FIPS 180-4         |
| Random generation    | OS CSPRNG        | —        | SP 800-90A         |
