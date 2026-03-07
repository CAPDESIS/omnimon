//! Cryptographic utilities. Provides AES-256-GCM encryption, Ed25519 digital signatures,
//! SHA-256 hashing, and secure payload handling for release integrity verification.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::RngCore;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const NONCE_LEN: usize = 12;

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
        assert!(result.unwrap_err().contains("signature verification failed"));
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
}
