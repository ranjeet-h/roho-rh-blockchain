//! BLAKE3 hashing implementation
//! 
//! All hashing in RH uses BLAKE3 for its speed and security.

use serde::{Deserialize, Serialize};
use std::fmt;

/// 32-byte hash output
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    /// Create a zero hash (used for genesis previous hash)
    pub const fn zero() -> Self {
        Hash([0u8; 32])
    }

    /// Create hash from bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Hash(bytes)
    }

    /// Create hash from hex string
    pub fn from_hex(hex: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(hex)?;
        if bytes.len() != 32 {
            return Err(hex::FromHexError::InvalidStringLength);
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Hash(arr))
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Get as bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash({})", self.to_hex())
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl Default for Hash {
    fn default() -> Self {
        Self::zero()
    }
}

/// Hash arbitrary bytes using BLAKE3
pub fn hash_bytes(data: &[u8]) -> Hash {
    let hash = blake3::hash(data);
    Hash(*hash.as_bytes())
}

/// Hash two hashes together (for Merkle tree)
pub fn hash_pair(left: &Hash, right: &Hash) -> Hash {
    let mut data = Vec::with_capacity(64);
    data.extend_from_slice(&left.0);
    data.extend_from_slice(&right.0);
    hash_bytes(&data)
}

/// Double hash (hash of hash) for extra security
pub fn double_hash(data: &[u8]) -> Hash {
    let first = hash_bytes(data);
    hash_bytes(&first.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_deterministic() {
        let data = b"hello world";
        let hash1 = hash_bytes(data);
        let hash2 = hash_bytes(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_different_inputs() {
        let hash1 = hash_bytes(b"hello");
        let hash2 = hash_bytes(b"world");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_zero_hash() {
        let zero = Hash::zero();
        assert_eq!(zero.0, [0u8; 32]);
    }

    #[test]
    fn test_hex_roundtrip() {
        let hash = hash_bytes(b"test");
        let hex = hash.to_hex();
        let recovered = Hash::from_hex(&hex).unwrap();
        assert_eq!(hash, recovered);
    }

    #[test]
    fn test_hash_pair() {
        let left = hash_bytes(b"left");
        let right = hash_bytes(b"right");
        let combined = hash_pair(&left, &right);
        
        // Should be deterministic
        let combined2 = hash_pair(&left, &right);
        assert_eq!(combined, combined2);
        
        // Order matters
        let reversed = hash_pair(&right, &left);
        assert_ne!(combined, reversed);
    }
}
