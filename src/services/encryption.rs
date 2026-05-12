use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key,
};
use sha2::{Digest, Sha256};

pub struct EncryptionService {
    key: [u8; 32],
}

impl EncryptionService {
    pub fn new(passphrase: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(passphrase.as_bytes());
        let key_bytes: [u8; 32] = hasher.finalize().into();
        Self { key: key_bytes }
    }

    pub fn encrypt(&self, plaintext: &str) -> anyhow::Result<String> {
        let key = Key::<Aes256Gcm>::from_slice(&self.key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        // Format: base64(nonce || ciphertext)
        let mut combined = Vec::with_capacity(nonce.len() + ciphertext.len());
        combined.extend_from_slice(&nonce);
        combined.extend_from_slice(&ciphertext);

        Ok(base64::encode(combined))
    }

    pub fn decrypt(&self, encrypted: &str) -> anyhow::Result<String> {
        let combined = base64::decode(encrypted)
            .map_err(|e| anyhow::anyhow!("Base64 decode failed: {}", e))?;

        if combined.len() < 12 {
            return Err(anyhow::anyhow!("Invalid encrypted data"));
        }

        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = aes_gcm::Nonce::from_slice(nonce_bytes);

        let key = Key::<Aes256Gcm>::from_slice(&self.key);
        let cipher = Aes256Gcm::new(key);

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

        String::from_utf8(plaintext)
            .map_err(|e| anyhow::anyhow!("UTF-8 decode failed: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let svc = EncryptionService::new("test-key");
        let plaintext = "secret-password-123";
        let encrypted = svc.encrypt(plaintext).unwrap();
        let decrypted = svc.decrypt(&encrypted).unwrap();
        assert_eq!(plaintext, decrypted);
    }
}
