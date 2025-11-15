// src/crypto/mod.rs
pub mod crypto_utils {
    use sodiumoxide::crypto::{pwhash, secretbox};
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum CryptoError {
        #[error("Key derivation failed")]
        KeyDerivationFailed,
        #[error("Encryption failed")]
        EncryptionFailed,
        #[error("Decryption failed (bad key or corrupted data)")]
        DecryptionFailed,
        #[error("Invalid salt length")]
        InvalidSalt,
        #[error("Invalid password")]
        InvalidPassword,
    }

    pub fn init() {
        sodiumoxide::init().expect("Failed to initialize sodiumoxide");
    }

    pub fn generate_salt() -> pwhash::argon2i13::Salt {
        pwhash::argon2i13::gen_salt()
    }

    pub fn derive_key(password: &str, salt: &pwhash::argon2i13::Salt) -> Result<secretbox::Key, CryptoError> {
        if password.is_empty() {
            return Err(CryptoError::InvalidPassword);
        }

        let mut key_bytes = [0u8; secretbox::KEYBYTES];
        pwhash::argon2i13::derive_key(
            &mut key_bytes,
            password.as_bytes(),
            salt,
            pwhash::argon2i13::OPSLIMIT_MODERATE,
            pwhash::argon2i13::MEMLIMIT_MODERATE,
        )
            .map_err(|_| CryptoError::KeyDerivationFailed)?;

        Ok(secretbox::Key(key_bytes))
    }

    pub fn encrypt(data: &[u8], key: &secretbox::Key) -> Result<Vec<u8>, CryptoError> {
        let nonce = secretbox::gen_nonce();
        let ciphertext = secretbox::seal(data, &nonce, key);

        let mut result = nonce.0.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    pub fn decrypt(encrypted_data: &[u8], key: &secretbox::Key) -> Result<Vec<u8>, CryptoError> {
        if encrypted_data.len() < secretbox::NONCEBYTES {
            return Err(CryptoError::DecryptionFailed);
        }

        let nonce = secretbox::Nonce::from_slice(&encrypted_data[..secretbox::NONCEBYTES])
            .ok_or(CryptoError::DecryptionFailed)?;

        let ciphertext = &encrypted_data[secretbox::NONCEBYTES..];

        secretbox::open(ciphertext, &nonce, key).map_err(|_| CryptoError::DecryptionFailed)
    }
}
