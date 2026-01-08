//! Block structure for the RH blockchain
//! 
//! Defines the immutable block and block header structures.

use serde::{Deserialize, Serialize};
use crate::crypto::Hash;
use crate::validation::Transaction;

/// Block header containing all metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockHeader {
    /// Protocol version
    pub version: u32,
    /// Hash of the previous block
    pub prev_hash: Hash,
    /// Merkle root of all transactions
    pub merkle_root: Hash,
    /// Block timestamp (seconds since Unix epoch)
    pub timestamp: u64,
    /// Difficulty target (compact representation)
    pub difficulty_target: u32,
    /// Nonce used for PoW
    pub nonce: u64,
}

impl BlockHeader {
    /// Create a new block header
    pub fn new(
        version: u32,
        prev_hash: Hash,
        merkle_root: Hash,
        timestamp: u64,
        difficulty_target: u32,
        nonce: u64,
    ) -> Self {
        Self {
            version,
            prev_hash,
            merkle_root,
            timestamp,
            difficulty_target,
            nonce,
        }
    }

    /// Serialize the header for hashing
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&self.prev_hash.0);
        bytes.extend_from_slice(&self.merkle_root.0);
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes.extend_from_slice(&self.difficulty_target.to_le_bytes());
        bytes.extend_from_slice(&self.nonce.to_le_bytes());
        bytes
    }

    /// Calculate the hash of this header
    pub fn hash(&self) -> Hash {
        crate::crypto::hash_bytes(&self.to_bytes())
    }
}

/// A complete block containing header and transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Block header
    pub header: BlockHeader,
    /// List of transactions in this block
    pub transactions: Vec<Transaction>,
}

impl Block {
    /// Create a new block
    pub fn new(header: BlockHeader, transactions: Vec<Transaction>) -> Self {
        Self { header, transactions }
    }

    /// Get the block hash
    pub fn hash(&self) -> Hash {
        self.header.hash()
    }

    /// Get the block height (must be tracked externally)
    pub fn prev_hash(&self) -> &Hash {
        &self.header.prev_hash
    }

    /// Check if this is the genesis block
    pub fn is_genesis(&self) -> bool {
        self.header.prev_hash == Hash::zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_header_serialization() {
        let header = BlockHeader::new(
            1,
            Hash::zero(),
            Hash::zero(),
            1234567890,
            0x1d00ffff,
            0,
        );
        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), 4 + 32 + 32 + 8 + 4 + 8); // 88 bytes
    }

    #[test]
    fn test_genesis_block_detection() {
        let header = BlockHeader::new(
            1,
            Hash::zero(),
            Hash::zero(),
            1234567890,
            0x1d00ffff,
            0,
        );
        let block = Block::new(header, vec![]);
        assert!(block.is_genesis());
    }
}
