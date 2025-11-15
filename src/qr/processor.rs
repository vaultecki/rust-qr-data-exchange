// src/qr/processor.rs
use crate::crypto::crypto_utils;
use base64::{engine::general_purpose, Engine};
use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::pwhash;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum QrProcessorError {
    #[error("Crypto error: {0}")]
    Crypto(#[from] crypto_utils::CryptoError),
    #[error("Compression error: {0}")]
    Compression(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Base64 error: {0}")]
    Base64(#[from] base64::DecodeError),
}

#[derive(Serialize, Deserialize)]
struct QrData {
    salt: Vec<u8>,
    encrypted: Vec<u8>,
}

pub struct QrDataProcessor;

impl QrDataProcessor {
    pub fn serialize(raw_data: &[u8], password: &str) -> Result<String, QrProcessorError> {
        crypto_utils::init();

        let salt = crypto_utils::generate_salt();
        let key = crypto_utils::derive_key(password, &salt)?;

        let compressed = zstd::encode_all(raw_data, 16)
            .map_err(|e| QrProcessorError::Compression(e.to_string()))?;

        let encrypted = crypto_utils::encrypt(&compressed, &key)?;

        let qr_data = QrData {
            salt: salt.0.to_vec(),
            encrypted,
        };

        let packed = rmp_serde::to_vec(&qr_data)
            .map_err(|e| QrProcessorError::Serialization(e.to_string()))?;

        Ok(general_purpose::STANDARD.encode(packed))
    }

    pub fn deserialize(input_string: &str, password: &str) -> Result<Vec<u8>, QrProcessorError> {
        crypto_utils::init();

        let packed = general_purpose::STANDARD.decode(input_string)?;

        let qr_data: QrData = rmp_serde::from_slice(&packed)
            .map_err(|e| QrProcessorError::Serialization(e.to_string()))?;

        let salt = pwhash::argon2i13::Salt::from_slice(&qr_data.salt)
            .ok_or(crypto_utils::CryptoError::InvalidSalt)?;

        let key = crypto_utils::derive_key(password, &salt)?;

        let decrypted = crypto_utils::decrypt(&qr_data.encrypted, &key)?;

        let decompressed = zstd::decode_all(&decrypted[..])
            .map_err(|e| QrProcessorError::Compression(e.to_string()))?;

        Ok(decompressed)
    }
}
