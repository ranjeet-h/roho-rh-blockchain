//! Schnorr signature implementation
//! 
//! Uses the secp256k1 curve with Schnorr signatures for transaction signing.

use k256::schnorr::{SigningKey, VerifyingKey, Signature};
use k256::schnorr::signature::{Signer, Verifier};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::Hash;

/// Signature errors
#[derive(Debug, Error)]
pub enum SignatureError {
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Invalid public key")]
    InvalidPublicKey,
    #[error("Invalid private key")]
    InvalidPrivateKey,
    #[error("Signing failed: {0}")]
    SigningFailed(String),
}

/// 32-byte private key
#[derive(Clone)]
pub struct PrivateKey(SigningKey);

impl std::fmt::Debug for PrivateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PrivateKey([REDACTED])")
    }
}

/// 32-byte public key (x-only for Schnorr)
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicKey(#[serde(with = "pubkey_serde")] pub [u8; 32]);

/// 64-byte Schnorr signature
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchnorrSignature(#[serde(with = "sig_serde")] pub [u8; 64]);

mod pubkey_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(bytes)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
        if bytes.len() != 32 {
            return Err(serde::de::Error::custom("Invalid public key length"));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(arr)
    }
}

mod sig_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(bytes)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
        if bytes.len() != 64 {
            return Err(serde::de::Error::custom("Invalid signature length"));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Ok(arr)
    }
}

impl PrivateKey {
    /// Generate a new random private key
    pub fn generate() -> Self {
        let signing_key = SigningKey::random(&mut OsRng);
        PrivateKey(signing_key)
    }

    /// Create from 32 bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, SignatureError> {
        SigningKey::from_bytes(bytes)
            .map(PrivateKey)
            .map_err(|_| SignatureError::InvalidPrivateKey)
    }

    /// Get the corresponding public key
    pub fn public_key(&self) -> PublicKey {
        let verifying_key = self.0.verifying_key();
        let bytes = verifying_key.to_bytes();
        PublicKey(bytes.into())
    }

    /// Sign a message hash
    pub fn sign(&self, message: &Hash) -> Result<SchnorrSignature, SignatureError> {
        let signature: Signature = self.0.sign(&message.0);
        Ok(SchnorrSignature(signature.to_bytes()))
    }

    /// Export to bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes().into()
    }
}

impl PublicKey {
    /// Create from 32 bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, SignatureError> {
        // Validate by trying to create a verifying key
        VerifyingKey::from_bytes(bytes)
            .map_err(|_| SignatureError::InvalidPublicKey)?;
        Ok(PublicKey(*bytes))
    }

    /// Verify a signature
    pub fn verify(&self, message: &Hash, signature: &SchnorrSignature) -> bool {
        let verifying_key = match VerifyingKey::from_bytes(&self.0) {
            Ok(vk) => vk,
            Err(_) => return false,
        };

        let sig = match Signature::try_from(signature.0.as_slice()) {
            Ok(s) => s,
            Err(_) => return false,
        };

        verifying_key.verify(&message.0, &sig).is_ok()
    }

    /// Convert to address with checksum
    pub fn to_address(&self) -> String {
        // Address = "RH" + Base58Check(BLAKE3(pubkey)[0:20])
        let hash = super::hash_bytes(&self.0);
        let addr_bytes = &hash.0[0..20];
        
        // Add checksum (4 bytes of double hash)
        let checksum = super::double_hash(addr_bytes);
        
        let mut with_checksum = Vec::with_capacity(24);
        with_checksum.extend_from_slice(addr_bytes);
        with_checksum.extend_from_slice(&checksum.0[0..4]);
        
        format!("RH{}", bs58::encode(&with_checksum).into_string())
    }

    /// Export to bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
}

impl SchnorrSignature {
    /// Create from 64 bytes
    pub fn from_bytes(bytes: &[u8; 64]) -> Self {
        SchnorrSignature(*bytes)
    }

    /// Export to bytes
    pub fn to_bytes(&self) -> [u8; 64] {
        self.0
    }
}

impl std::fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PublicKey({})", hex::encode(self.0))
    }
}

impl std::fmt::Debug for SchnorrSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Signature({})", hex::encode(self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let private = PrivateKey::generate();
        let public = private.public_key();
        assert_eq!(public.0.len(), 32);
    }

    #[test]
    fn test_sign_verify() {
        let private = PrivateKey::generate();
        let public = private.public_key();
        
        let message = super::super::hash_bytes(b"test message");
        let signature = private.sign(&message).unwrap();
        
        assert!(public.verify(&message, &signature));
    }

    #[test]
    fn test_wrong_key_fails() {
        let private1 = PrivateKey::generate();
        let private2 = PrivateKey::generate();
        let public2 = private2.public_key();
        
        let message = super::super::hash_bytes(b"test message");
        let signature = private1.sign(&message).unwrap();
        
        assert!(!public2.verify(&message, &signature));
    }

    #[test]
    fn test_wrong_message_fails() {
        let private = PrivateKey::generate();
        let public = private.public_key();
        
        let message1 = super::super::hash_bytes(b"message 1");
        let message2 = super::super::hash_bytes(b"message 2");
        let signature = private.sign(&message1).unwrap();
        
        assert!(!public.verify(&message2, &signature));
    }

    #[test]
    fn test_address_generation() {
        let private = PrivateKey::generate();
        let public = private.public_key();
        let address = public.to_address();
        
        assert!(address.starts_with("RH"));
        assert!(address.len() > 10);
    }

    #[test]
    fn test_key_serialization() {
        let private = PrivateKey::generate();
        let bytes = private.to_bytes();
        let recovered = PrivateKey::from_bytes(&bytes).unwrap();
        
        assert_eq!(private.public_key().0, recovered.public_key().0);
    }
}
