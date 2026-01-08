//! Block miner implementation
//! 
//! Assembles candidate blocks and performs PoW.

use crate::consensus::{Block, BlockHeader, calculate_block_reward};
use crate::crypto::{Hash, compute_merkle_root};
use crate::validation::Transaction;
use crate::storage::ChainState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Mining result
#[derive(Debug)]
pub enum MiningResult {
    /// Successfully mined a block
    Success(Block),
    /// Mining was interrupted
    Interrupted,
    /// No transactions to mine
    NoWork,
}

/// Block miner
#[derive(Clone)]
pub struct Miner {
    /// Miner's public key hash (for coinbase)
    miner_pubkey_hash: Hash,
    /// Stop signal
    stop_signal: Arc<AtomicBool>,
}

impl Miner {
    /// Create a new miner
    pub fn new(miner_pubkey_hash: Hash) -> Self {
        Self {
            miner_pubkey_hash,
            stop_signal: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get a stop signal handle
    pub fn stop_signal(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.stop_signal)
    }

    /// Stop mining
    pub fn stop(&self) {
        self.stop_signal.store(true, Ordering::SeqCst);
    }

    /// Reset stop signal
    pub fn reset(&self) {
        self.stop_signal.store(false, Ordering::SeqCst);
    }

    /// Assemble a candidate block
    pub fn assemble_block(
        &self,
        chain_state: &ChainState,
        transactions: Vec<Transaction>,
    ) -> Block {
        let height = chain_state.height + 1;
        let reward = calculate_block_reward(height, chain_state.total_issued);

        // Create coinbase transaction
        let coinbase = Transaction::coinbase(reward, self.miner_pubkey_hash);

        // Combine coinbase with other transactions
        let mut all_txs = vec![coinbase];
        all_txs.extend(transactions);

        // Calculate merkle root
        let tx_hashes: Vec<Hash> = all_txs.iter().map(|tx| tx.hash()).collect();
        let merkle_root = compute_merkle_root(&tx_hashes);

        // Get current timestamp
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create header
        let header = BlockHeader::new(
            1, // version
            chain_state.tip_hash,
            merkle_root,
            timestamp,
            chain_state.difficulty,
            0, // nonce starts at 0
        );

        Block::new(header, all_txs)
    }

    /// Mine a block (find valid nonce)
    /// 
    /// This performs the PoW loop, incrementing the nonce until
    /// a valid hash is found or mining is interrupted.
    pub fn mine_block(&self, mut block: Block) -> MiningResult {
        let target = difficulty_to_target(block.header.difficulty_target);

        loop {
            // Check stop signal
            if self.stop_signal.load(Ordering::SeqCst) {
                return MiningResult::Interrupted;
            }

            // Calculate hash
            let hash = block.header.hash();

            // Check if valid
            if compare_to_target(&hash, &target) {
                return MiningResult::Success(block);
            }

            // Increment nonce
            block.header.nonce = block.header.nonce.wrapping_add(1);

            // If nonce wrapped, update timestamp
            if block.header.nonce == 0 {
                block.header.timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
            }
        }
    }

    /// Mine with progress callback
    pub fn mine_with_progress<F>(
        &self,
        mut block: Block,
        progress_interval: u64,
        mut callback: F,
    ) -> MiningResult
    where
        F: FnMut(u64), // nonce count
    {
        let target = difficulty_to_target(block.header.difficulty_target);
        let mut iterations = 0u64;

        loop {
            if self.stop_signal.load(Ordering::SeqCst) {
                return MiningResult::Interrupted;
            }

            let hash = block.header.hash();

            if compare_to_target(&hash, &target) {
                return MiningResult::Success(block);
            }

            block.header.nonce = block.header.nonce.wrapping_add(1);
            iterations += 1;

            if iterations % progress_interval == 0 {
                callback(iterations);
            }

            if block.header.nonce == 0 {
                block.header.timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
            }
        }
    }
}

/// Convert compact difficulty to 256-bit target
fn difficulty_to_target(compact: u32) -> [u8; 32] {
    let exponent = (compact >> 24) as usize;
    let mantissa = compact & 0x007FFFFF;

    let mut target = [0u8; 32];

    if exponent == 0 || exponent > 32 {
        return target;
    }

    let negative = (compact & 0x00800000) != 0;
    if negative {
        return target;
    }

    if exponent <= 3 {
        let value = mantissa >> (8 * (3 - exponent));
        target[31] = (value & 0xFF) as u8;
        if exponent >= 2 {
            target[30] = ((value >> 8) & 0xFF) as u8;
        }
        if exponent >= 3 {
            target[29] = ((value >> 16) & 0xFF) as u8;
        }
    } else {
        let start = 32 - exponent;
        target[start] = ((mantissa >> 16) & 0xFF) as u8;
        if start + 1 < 32 {
            target[start + 1] = ((mantissa >> 8) & 0xFF) as u8;
        }
        if start + 2 < 32 {
            target[start + 2] = (mantissa & 0xFF) as u8;
        }
    }

    target
}

/// Compare hash to target (hash <= target)
fn compare_to_target(hash: &Hash, target: &[u8; 32]) -> bool {
    for i in 0..32 {
        if hash.0[i] < target[i] {
            return true;
        }
        if hash.0[i] > target[i] {
            return false;
        }
    }
    true // Equal
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::hash_bytes;

    #[test]
    fn test_compare_to_target() {
        let easy_target = [0xFF; 32]; // Very easy
        let hash = hash_bytes(b"test");
        assert!(compare_to_target(&hash, &easy_target));

        let hard_target = [0x00; 32]; // Impossible
        assert!(!compare_to_target(&hash, &hard_target));
    }

    #[test]
    fn test_miner_stop_signal() {
        let miner = Miner::new(Hash::zero());
        let signal = miner.stop_signal();

        assert!(!signal.load(Ordering::SeqCst));
        
        miner.stop();
        assert!(signal.load(Ordering::SeqCst));
        
        miner.reset();
        assert!(!signal.load(Ordering::SeqCst));
    }
}
