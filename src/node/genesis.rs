//! Genesis block generation for ROHO (RH) blockchain
//! 
//! Creates the immutable genesis block with founder allocation.
//! RH is the short form used in addresses and logos.

use crate::consensus::{Block, BlockHeader};
use crate::crypto::{Hash, compute_merkle_root, hash_bytes};
use crate::validation::Transaction;
use crate::constants::{FOUNDER_ALLOCATION, GENESIS_TIMESTAMP, FOUNDER_ADDRESS, CONSTITUTION_HASH};

/// Initial difficulty target (easy for genesis)
const GENESIS_DIFFICULTY: u32 = 0x1e00ffff;

/// Genesis block version
const GENESIS_VERSION: u32 = 1;

/// Create the genesis block
/// 
/// This function produces a reproducible, byte-for-byte identical genesis block.
/// It MUST be called exactly once at chain initialization.
pub fn create_genesis_block() -> Block {
    // Create founder allocation transaction
    let founder_pubkey_hash = hash_bytes(FOUNDER_ADDRESS.as_bytes());
    
    let founder_tx = Transaction {
        version: 1,
        inputs: vec![crate::validation::TxInput {
            prev_tx_hash: Hash::zero(),
            output_index: 0xFFFFFFFF,
            signature: crate::crypto::SchnorrSignature([0u8; 64]),
            public_key: crate::crypto::PublicKey([0u8; 32]),
        }],
        outputs: vec![crate::validation::TxOutput {
            amount: FOUNDER_ALLOCATION,
            pubkey_hash: founder_pubkey_hash,
        }],
        lock_time: 0,
        nonce: 0,
    };

    // Embed constitution hash in a second (empty value) output
    let constitution_hash = Hash::from_hex(CONSTITUTION_HASH)
        .unwrap_or_else(|_| Hash::zero());
    
    let constitution_tx = Transaction {
        version: 1,
        inputs: vec![crate::validation::TxInput {
            prev_tx_hash: Hash::zero(),
            output_index: 0xFFFFFFFE, // Marker for constitution
            signature: crate::crypto::SchnorrSignature([0u8; 64]),
            public_key: crate::crypto::PublicKey([0u8; 32]),
        }],
        outputs: vec![crate::validation::TxOutput {
            amount: 0, // No value, just embedding data
            pubkey_hash: constitution_hash,
        }],
        lock_time: 0,
        nonce: 0,
    };

    let transactions = vec![founder_tx, constitution_tx];

    // Calculate merkle root
    let tx_hashes: Vec<Hash> = transactions.iter()
        .map(|tx| tx.hash())
        .collect();
    let merkle_root = compute_merkle_root(&tx_hashes);

    // Create genesis header
    let header = BlockHeader::new(
        GENESIS_VERSION,
        crate::constants::CHAIN_ID, // chain_id for replay protection
        Hash::zero(), // No previous block
        merkle_root,
        GENESIS_TIMESTAMP,
        GENESIS_DIFFICULTY,
        0, // Nonce (genesis doesn't need mining in most chains)
    );

    Block::new(header, transactions)
}

/// Verify genesis block matches expected hash
pub fn verify_genesis_hash(block: &Block, expected_hash: &Hash) -> bool {
    block.hash() == *expected_hash
}

/// Get genesis block hash (computed once)
pub fn genesis_hash() -> Hash {
    create_genesis_block().hash()
}

/// Genesis block statistics
#[derive(Debug)]
pub struct GenesisInfo {
    pub hash: Hash,
    pub merkle_root: Hash,
    pub timestamp: u64,
    pub difficulty: u32,
    pub founder_allocation: u64,
}

impl GenesisInfo {
    pub fn new() -> Self {
        let genesis = create_genesis_block();
        Self {
            hash: genesis.hash(),
            merkle_root: genesis.header.merkle_root,
            timestamp: genesis.header.timestamp,
            difficulty: genesis.header.difficulty_target,
            founder_allocation: FOUNDER_ALLOCATION,
        }
    }
}

impl Default for GenesisInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_is_deterministic() {
        let genesis1 = create_genesis_block();
        let genesis2 = create_genesis_block();

        assert_eq!(genesis1.hash(), genesis2.hash());
    }

    #[test]
    fn test_genesis_has_founder_allocation() {
        let genesis = create_genesis_block();
        
        assert!(!genesis.transactions.is_empty());
        
        let total: u64 = genesis.transactions[0].outputs.iter()
            .map(|o| o.amount)
            .sum();
        
        assert_eq!(total, FOUNDER_ALLOCATION);
    }

    #[test]
    fn test_genesis_is_genesis() {
        let genesis = create_genesis_block();
        assert!(genesis.is_genesis());
    }

    #[test]
    fn test_genesis_hash_stable() {
        let hash1 = genesis_hash();
        let hash2 = genesis_hash();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_genesis_info() {
        let info = GenesisInfo::new();
        assert_eq!(info.founder_allocation, FOUNDER_ALLOCATION);
        assert_eq!(info.timestamp, GENESIS_TIMESTAMP);
    }
}
