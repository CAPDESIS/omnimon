use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rand::RngCore;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

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
}
