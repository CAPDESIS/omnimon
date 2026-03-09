//! Cryptographic utilities. Provides AES-256-GCM encryption, Ed25519 digital signatures,
//! SHA-256 hashing, HKDF key derivation, and secure payload handling for release integrity
//! verification. All key material is zeroized on drop to prevent memory-resident secrets.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use hkdf::Hkdf;
use rand::RngCore;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop};

const NONCE_LEN: usize = 12;

// ---------------------------------------------------------------------------
// HKDF Key Derivation (NIST SP 800-56C)
// ---------------------------------------------------------------------------

/// Domain separation contexts for HKDF key derivation.
pub mod kdf_context {
    pub const DATA_ENCRYPTION: &[u8] = b"omnimon-data-encryption";
    pub const API_KEY_STORAGE: &[u8] = b"omnimon-api-key-storage";
    pub const CONFIG_ENCRYPTION: &[u8] = b"omnimon-config-encryption";
}

/// A 256-bit key that is automatically zeroized when dropped.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct DerivedKey([u8; 32]);

impl DerivedKey {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Derives a 256-bit key from `master_key` using HKDF-SHA256 with the given `context`.
/// Different contexts produce cryptographically independent keys from the same master.
pub fn derive_key(master_key: &[u8], context: &[u8]) -> DerivedKey {
    let hk = Hkdf::<Sha256>::new(None, master_key);
    let mut okm = [0u8; 32];
    hk.expand(context, &mut okm)
        .expect("32 bytes is a valid length for HKDF-SHA256");
    DerivedKey(okm)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedPayload {
    pub algorithm: String,
    pub nonce_b64: String,
    pub ciphertext_b64: String,
}

pub fn encrypt_bytes(key: &[u8; 32], plaintext: &[u8]) -> Result<EncryptedPayload, String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("invalid key: {e}"))?;

    let mut nonce = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce);

    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext)
        .map_err(|_| "encryption failed".to_string())?;

    Ok(EncryptedPayload {
        algorithm: "AES-256-GCM".to_string(),
        nonce_b64: STANDARD.encode(nonce),
        ciphertext_b64: STANDARD.encode(ciphertext),
    })
}

/// Encrypts bytes using a [`DerivedKey`] (HKDF-derived, zeroized on drop).
pub fn encrypt_bytes_derived(
    key: &DerivedKey,
    plaintext: &[u8],
) -> Result<EncryptedPayload, String> {
    encrypt_bytes(key.as_bytes(), plaintext)
}

/// Decrypts bytes using a [`DerivedKey`].
pub fn decrypt_bytes_derived(
    key: &DerivedKey,
    encrypted: &EncryptedPayload,
) -> Result<Vec<u8>, String> {
    decrypt_bytes(key.as_bytes(), encrypted)
}

pub fn decrypt_bytes(key: &[u8; 32], encrypted: &EncryptedPayload) -> Result<Vec<u8>, String> {
    if encrypted.algorithm != "AES-256-GCM" {
        return Err(format!("unsupported algorithm: {}", encrypted.algorithm));
    }

    let nonce = STANDARD
        .decode(&encrypted.nonce_b64)
        .map_err(|e| format!("invalid nonce b64: {e}"))?;
    if nonce.len() != NONCE_LEN {
        return Err("invalid nonce length".to_string());
    }

    let ciphertext = STANDARD
        .decode(&encrypted.ciphertext_b64)
        .map_err(|e| format!("invalid ciphertext b64: {e}"))?;

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("invalid key: {e}"))?;
    cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| "decryption failed".to_string())
}

pub fn encrypt_json<T: Serialize>(key: &[u8; 32], value: &T) -> Result<EncryptedPayload, String> {
    let plaintext = serde_json::to_vec(value).map_err(|e| format!("serialize failed: {e}"))?;
    encrypt_bytes(key, &plaintext)
}

pub fn decrypt_json<T: DeserializeOwned>(
    key: &[u8; 32],
    encrypted: &EncryptedPayload,
) -> Result<T, String> {
    let plaintext = decrypt_bytes(key, encrypted)?;
    serde_json::from_slice(&plaintext).map_err(|e| format!("deserialize failed: {e}"))
}

// ---------------------------------------------------------------------------
// Ed25519 Digital Signatures (NIST FIPS 186-5)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseSignature {
    pub version: String,
    pub sha256: String,
    pub signature_b64: String,
    pub public_key_b64: String,
}

pub fn generate_ed25519_keypair() -> (SigningKey, VerifyingKey) {
    let mut csprng = rand::thread_rng();
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}

pub fn export_public_key(verifying_key: &VerifyingKey) -> String {
    STANDARD.encode(verifying_key.to_bytes())
}

pub fn import_public_key(b64: &str) -> Result<VerifyingKey, String> {
    let bytes = STANDARD
        .decode(b64)
        .map_err(|e| format!("invalid base64: {e}"))?;
    let key_bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "public key must be 32 bytes".to_string())?;
    VerifyingKey::from_bytes(&key_bytes).map_err(|e| format!("invalid Ed25519 public key: {e}"))
}

pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

pub fn sign_release(
    signing_key: &SigningKey,
    release_bytes: &[u8],
    version: &str,
) -> ReleaseSignature {
    let hash = sha256_hex(release_bytes);
    let signature = signing_key.sign(release_bytes);
    ReleaseSignature {
        version: version.to_string(),
        sha256: hash,
        signature_b64: STANDARD.encode(signature.to_bytes()),
        public_key_b64: export_public_key(&signing_key.verifying_key()),
    }
}

pub fn verify_release(
    release_bytes: &[u8],
    release_sig: &ReleaseSignature,
    trusted_public_key: &VerifyingKey,
) -> Result<(), String> {
    // 1. Verify SHA-256 hash
    let computed_hash = sha256_hex(release_bytes);
    if computed_hash != release_sig.sha256 {
        return Err(format!(
            "SHA-256 mismatch: expected {}, got {}",
            release_sig.sha256, computed_hash
        ));
    }

    // 2. Verify Ed25519 signature
    let sig_bytes = STANDARD
        .decode(&release_sig.signature_b64)
        .map_err(|e| format!("invalid signature base64: {e}"))?;
    let sig_array: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| "signature must be 64 bytes".to_string())?;
    let signature = Signature::from_bytes(&sig_array);

    trusted_public_key
        .verify(release_bytes, &signature)
        .map_err(|_| "Ed25519 signature verification failed".to_string())
}

// ---------------------------------------------------------------------------
// Update Manifest (served by update endpoint)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateManifest {
    pub version: String,
    pub sha256: String,
    pub signature_b64: String,
    pub download_url: String,
}

pub fn verify_update(
    downloaded_bytes: &[u8],
    manifest: &UpdateManifest,
    trusted_public_key: &VerifyingKey,
) -> Result<(), String> {
    let release_sig = ReleaseSignature {
        version: manifest.version.clone(),
        sha256: manifest.sha256.clone(),
        signature_b64: manifest.signature_b64.clone(),
        public_key_b64: export_public_key(trusted_public_key),
    };
    verify_release(downloaded_bytes, &release_sig, trusted_public_key)
}

// ---------------------------------------------------------------------------
// Key Rotation (NIST SC-12)
// ---------------------------------------------------------------------------

/// Generates a fresh 256-bit encryption key using the OS CSPRNG.
pub fn generate_encryption_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

/// Rotates an encryption key: generates a new key, re-encrypts `existing_payload`
/// from `old_key` to the new key, and returns `(new_key, re_encrypted_payload)`.
/// The caller is responsible for persisting the new key to the keyring.
pub fn rotate_key(
    old_key: &[u8; 32],
    existing_payload: &EncryptedPayload,
) -> Result<([u8; 32], EncryptedPayload), String> {
    let plaintext = decrypt_bytes(old_key, existing_payload)?;
    let new_key = generate_encryption_key();
    let new_payload = encrypt_bytes(&new_key, &plaintext)?;
    Ok((new_key, new_payload))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn round_trip_json_encryption() {
        let key = [7u8; 32];
        let input = json!({"profile": "balanced", "refresh_ms": 1500});

        let encrypted = encrypt_json(&key, &input).expect("encrypt");
        let decrypted: serde_json::Value = decrypt_json(&key, &encrypted).expect("decrypt");

        assert_eq!(decrypted, input);
        assert_eq!(encrypted.algorithm, "AES-256-GCM");
    }

    #[test]
    fn wrong_key_fails_decryption() {
        let key_ok = [1u8; 32];
        let key_bad = [2u8; 32];
        let encrypted = encrypt_bytes(&key_ok, b"secret-data").expect("encrypt");

        let decrypted = decrypt_bytes(&key_bad, &encrypted);
        assert!(decrypted.is_err());
    }

    // --- Ed25519 Tests ---

    #[test]
    fn ed25519_keypair_generation_and_export_import() {
        let (_, verifying_key) = generate_ed25519_keypair();
        let exported = export_public_key(&verifying_key);
        let imported = import_public_key(&exported).expect("import");
        assert_eq!(verifying_key.to_bytes(), imported.to_bytes());
    }

    #[test]
    fn import_invalid_public_key_fails() {
        assert!(import_public_key("not-valid-base64!!!").is_err());
        assert!(import_public_key(&STANDARD.encode([0u8; 16])).is_err());
    }

    #[test]
    fn sha256_produces_correct_hex() {
        let hash = sha256_hex(b"hello world");
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn sign_and_verify_release() {
        let (signing_key, verifying_key) = generate_ed25519_keypair();
        let payload = b"OmniMon v4.2.0 release binary content";

        let sig = sign_release(&signing_key, payload, "v4.2.0");
        assert_eq!(sig.version, "v4.2.0");

        let result = verify_release(payload, &sig, &verifying_key);
        assert!(result.is_ok());
    }

    #[test]
    fn tampered_payload_fails_verification() {
        let (signing_key, verifying_key) = generate_ed25519_keypair();
        let payload = b"legitimate binary";

        let sig = sign_release(&signing_key, payload, "v4.2.0");

        let tampered = b"malicious binary";
        let result = verify_release(tampered, &sig, &verifying_key);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("SHA-256 mismatch"));
    }

    #[test]
    fn wrong_key_fails_signature_verification() {
        let (signing_key, _) = generate_ed25519_keypair();
        let (_, wrong_verifying_key) = generate_ed25519_keypair();
        let payload = b"release binary";

        let sig = sign_release(&signing_key, payload, "v4.2.0");

        let result = verify_release(payload, &sig, &wrong_verifying_key);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("signature verification failed"));
    }

    #[test]
    fn verify_update_manifest() {
        let (signing_key, verifying_key) = generate_ed25519_keypair();
        let payload = b"update payload";
        let sig = sign_release(&signing_key, payload, "v4.3.0");

        let manifest = UpdateManifest {
            version: sig.version,
            sha256: sig.sha256,
            signature_b64: sig.signature_b64,
            download_url: "https://github.com/omnimon/releases/v4.3.0".to_string(),
        };

        assert!(verify_update(payload, &manifest, &verifying_key).is_ok());
    }

    // --- HKDF Key Derivation Tests ---

    #[test]
    fn derive_key_is_deterministic() {
        let master = [42u8; 32];
        let k1 = derive_key(&master, kdf_context::DATA_ENCRYPTION);
        let k2 = derive_key(&master, kdf_context::DATA_ENCRYPTION);
        assert_eq!(k1.as_bytes(), k2.as_bytes());
    }

    #[test]
    fn derive_key_different_contexts_produce_different_keys() {
        let master = [42u8; 32];
        let k_data = derive_key(&master, kdf_context::DATA_ENCRYPTION);
        let k_api = derive_key(&master, kdf_context::API_KEY_STORAGE);
        let k_cfg = derive_key(&master, kdf_context::CONFIG_ENCRYPTION);
        assert_ne!(k_data.as_bytes(), k_api.as_bytes());
        assert_ne!(k_data.as_bytes(), k_cfg.as_bytes());
        assert_ne!(k_api.as_bytes(), k_cfg.as_bytes());
    }

    #[test]
    fn derive_key_output_is_32_bytes() {
        let master = [1u8; 16];
        let derived = derive_key(&master, b"test-context");
        assert_eq!(derived.as_bytes().len(), 32);
    }

    #[test]
    fn derived_key_encrypt_decrypt_round_trip() {
        let master = [99u8; 32];
        let key = derive_key(&master, kdf_context::DATA_ENCRYPTION);
        let plaintext = b"sensitive data for derived key test";
        let encrypted = encrypt_bytes_derived(&key, plaintext).expect("encrypt");
        let decrypted = decrypt_bytes_derived(&key, &encrypted).expect("decrypt");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn derived_key_wrong_context_fails_decryption() {
        let master = [99u8; 32];
        let key_enc = derive_key(&master, kdf_context::DATA_ENCRYPTION);
        let key_dec = derive_key(&master, kdf_context::CONFIG_ENCRYPTION);
        let encrypted = encrypt_bytes_derived(&key_enc, b"secret").expect("encrypt");
        assert!(decrypt_bytes_derived(&key_dec, &encrypted).is_err());
    }

    #[test]
    fn derived_key_compiles_with_zeroize() {
        let master = [0u8; 32];
        let mut key = derive_key(&master, b"zeroize-test");
        // Verify manual zeroize works
        key.zeroize();
        assert_eq!(key.as_bytes(), &[0u8; 32]);
    }

    // --- Key Rotation Tests (NIST SC-12) ---

    #[test]
    fn rotate_key_preserves_plaintext() {
        let old_key = [1u8; 32];
        let plaintext = b"data to survive rotation";
        let encrypted = encrypt_bytes(&old_key, plaintext).expect("encrypt");

        let (new_key, new_encrypted) = rotate_key(&old_key, &encrypted).expect("rotate");
        assert_ne!(old_key, new_key);

        let decrypted = decrypt_bytes(&new_key, &new_encrypted).expect("decrypt");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn rotate_key_old_key_cannot_decrypt_new_payload() {
        let old_key = [1u8; 32];
        let encrypted = encrypt_bytes(&old_key, b"secret").expect("encrypt");

        let (_, new_encrypted) = rotate_key(&old_key, &encrypted).expect("rotate");
        assert!(decrypt_bytes(&old_key, &new_encrypted).is_err());
    }

    #[test]
    fn generate_encryption_key_is_random() {
        let k1 = generate_encryption_key();
        let k2 = generate_encryption_key();
        assert_ne!(k1, k2);
    }
}
