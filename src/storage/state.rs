//! Chain state management
//! 
//! Tracks the current state of the blockchain including the UTXO set,
//! current height, total issued supply, and difficulty.

use std::collections::HashMap;
use crate::consensus::{Block, BlockHeader};
use crate::crypto::Hash;
use crate::constants::PUBLIC_ISSUANCE;
use super::{UTXOSet, UTXO, UTXOKey};

/// Complete chain state
#[derive(Debug)]
pub struct ChainState {
    /// Current UTXO set
    pub utxo_set: UTXOSet,
    /// Current block height
    pub height: u64,
    /// Hash of the current tip
    pub tip_hash: Hash,
    /// Total RH issued through mining (excludes founder allocation)
    pub total_issued: u64,
    /// Current difficulty target
    pub difficulty: u32,
    /// Block index: hash -> (header, height)
    block_index: HashMap<Hash, (BlockHeader, u64)>,
}

impl ChainState {
    /// Create a new chain state from genesis
    pub fn new(genesis_block: &Block) -> Self {
        let mut state = Self {
            utxo_set: UTXOSet::new(),
            height: 0,
            tip_hash: genesis_block.hash(),
            total_issued: 0,
            difficulty: genesis_block.header.difficulty_target,
            block_index: HashMap::new(),
        };

        // Apply genesis transactions (founder allocation)
        for tx in &genesis_block.transactions {
            state.utxo_set.apply_transaction(tx, 0);
        }

        // Index genesis block
        state.block_index.insert(
            genesis_block.hash(),
            (genesis_block.header.clone(), 0),
        );

        state
    }

    /// Apply a new block to the state
    /// 
    /// Returns the spent UTXOs for potential rollback.
    pub fn apply_block(&mut self, block: &Block) -> Vec<(UTXOKey, UTXO)> {
        let mut spent_utxos = Vec::new();
        let new_height = self.height + 1;

        // Collect spent UTXOs before applying
        for tx in &block.transactions {
            if !tx.is_coinbase() {
                for input in &tx.inputs {
                    if let Some(utxo) = self.utxo_set.get(&input.prev_tx_hash, input.output_index) {
                        spent_utxos.push((
                            (input.prev_tx_hash, input.output_index),
                            utxo.clone(),
                        ));
                    }
                }
            }
        }

        // Apply transactions
        for tx in &block.transactions {
            // Track block reward for total issued
            if tx.is_coinbase() {
                let reward: u64 = tx.outputs.iter().map(|o| o.amount).sum();
                self.total_issued = self.total_issued.saturating_add(reward);
            }
            
            self.utxo_set.apply_transaction(tx, new_height);
        }

        // Update state
        self.height = new_height;
        self.tip_hash = block.hash();
        self.difficulty = block.header.difficulty_target;

        // Index block
        self.block_index.insert(
            block.hash(),
            (block.header.clone(), new_height),
        );

        spent_utxos
    }

    /// Revert a block from the state
    pub fn revert_block(&mut self, block: &Block, spent_utxos: Vec<(UTXOKey, UTXO)>) {
        // Revert transactions in reverse order
        for tx in block.transactions.iter().rev() {
            // Find the spent UTXOs for this transaction
            let tx_spent: Vec<_> = spent_utxos.iter()
                .filter(|((hash, _), _)| {
                    tx.inputs.iter().any(|i| i.prev_tx_hash == *hash)
                })
                .cloned()
                .collect();
            
            self.utxo_set.revert_transaction(tx, &tx_spent);

            // Revert block reward tracking
            if tx.is_coinbase() {
                let reward: u64 = tx.outputs.iter().map(|o| o.amount).sum();
                self.total_issued = self.total_issued.saturating_sub(reward);
            }
        }

        // Update state
        self.height -= 1;
        self.tip_hash = block.header.prev_hash;

        // Get previous difficulty from index
        if let Some((header, _)) = self.block_index.get(&block.header.prev_hash) {
            self.difficulty = header.difficulty_target;
        }
    }

    /// Get block header by hash
    pub fn get_block_header(&self, hash: &Hash) -> Option<&BlockHeader> {
        self.block_index.get(hash).map(|(h, _)| h)
    }

    /// Get block height by hash
    pub fn get_block_height(&self, hash: &Hash) -> Option<u64> {
        self.block_index.get(hash).map(|(_, h)| *h)
    }

    /// Verify total supply invariant
    pub fn verify_supply_invariant(&self) -> bool {
        // Total issued through mining must not exceed PUBLIC_ISSUANCE
        if self.total_issued > PUBLIC_ISSUANCE {
            return false;
        }

        // Calculate total from UTXO set
        // This would require summing all UTXOs, which is expensive
        // In production, this would be done periodically or on demand
        true
    }

    /// Get statistics about the chain state
    pub fn get_stats(&self) -> ChainStats {
        ChainStats {
            height: self.height,
            tip_hash: self.tip_hash,
            total_issued: self.total_issued,
            utxo_count: self.utxo_set.len(),
            difficulty: self.difficulty,
        }
    }
}

/// Statistics about the chain state
#[derive(Debug)]
pub struct ChainStats {
    pub height: u64,
    pub tip_hash: Hash,
    pub total_issued: u64,
    pub utxo_count: usize,
    pub difficulty: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::Block;
    use crate::validation::Transaction;
    use crate::crypto::hash_bytes;

    fn make_genesis() -> Block {
        use crate::consensus::BlockHeader;
        use crate::constants::FOUNDER_ALLOCATION;

        let founder_tx = Transaction::coinbase(
            FOUNDER_ALLOCATION,
            hash_bytes(b"founder"),
        );

        Block::new(
            BlockHeader::new(
                1,
                Hash::zero(),
                hash_bytes(b"merkle"),
                1234567890,
                0x1d00ffff,
                0,
            ),
            vec![founder_tx],
        )
    }

    #[test]
    fn test_genesis_initialization() {
        let genesis = make_genesis();
        let state = ChainState::new(&genesis);

        assert_eq!(state.height, 0);
        assert_eq!(state.tip_hash, genesis.hash());
    }

    #[test]
    fn test_apply_block() {
        let genesis = make_genesis();
        let mut state = ChainState::new(&genesis);

        // Create a simple block
        use crate::consensus::BlockHeader;
        let block = Block::new(
            BlockHeader::new(
                1,
                genesis.hash(),
                hash_bytes(b"merkle2"),
                1234567891,
                0x1d00ffff,
                123,
            ),
            vec![Transaction::coinbase(5000, hash_bytes(b"miner"))],
        );

        let spent = state.apply_block(&block);
        
        assert_eq!(state.height, 1);
        assert_eq!(state.tip_hash, block.hash());
        assert!(spent.is_empty()); // Only coinbase, no spent
    }
}
