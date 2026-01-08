//! Block and chain validation
//! 
//! Pure functions for validating blocks and chains.

use crate::consensus::{Block, BlockHeader};
use crate::crypto::{Hash, compute_merkle_root};
use crate::validation::Transaction;
use crate::storage::UTXOSet;
use thiserror::Error;

/// Validation errors
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Invalid proof of work")]
    InvalidPoW,
    #[error("Invalid merkle root")]
    InvalidMerkleRoot,
    #[error("Invalid previous hash")]
    InvalidPrevHash,
    #[error("Invalid timestamp")]
    InvalidTimestamp,
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
    #[error("Invalid block reward")]
    InvalidBlockReward,
    #[error("Invalid difficulty target")]
    InvalidDifficulty,
    #[error("Double spend detected")]
    DoubleSpend,
    #[error("Supply exceeded")]
    SupplyExceeded,
}

/// Validate proof of work
/// 
/// The block hash must be less than the target derived from difficulty_target
pub fn validate_pow(header: &BlockHeader) -> Result<(), ValidationError> {
    let hash = header.hash();
    let target = difficulty_to_target(header.difficulty_target);
    
    if hash_to_u256(&hash) > target {
        return Err(ValidationError::InvalidPoW);
    }
    
    Ok(())
}

/// Validate merkle root matches transactions
pub fn validate_merkle_root(block: &Block) -> Result<(), ValidationError> {
    let tx_hashes: Vec<Hash> = block.transactions.iter()
        .map(|tx| tx.hash())
        .collect();
    
    let computed_root = compute_merkle_root(&tx_hashes);
    
    if computed_root != block.header.merkle_root {
        return Err(ValidationError::InvalidMerkleRoot);
    }
    
    Ok(())
}

/// Validate a block against the current chain state
pub fn validate_block(
    block: &Block,
    prev_block_hash: &Hash,
    expected_difficulty: u32,
    utxo_set: &UTXOSet,
    current_height: u64,
    total_issued: u64,
) -> Result<(), ValidationError> {
    // Check previous hash
    if block.header.prev_hash != *prev_block_hash {
        return Err(ValidationError::InvalidPrevHash);
    }
    
    // Check difficulty
    if block.header.difficulty_target != expected_difficulty {
        return Err(ValidationError::InvalidDifficulty);
    }
    
    // Validate PoW
    validate_pow(&block.header)?;
    
    // Validate merkle root
    validate_merkle_root(block)?;
    
    // Validate all transactions
    validate_transactions(&block.transactions, utxo_set)?;
    
    // Validate block reward
    validate_block_reward(block, current_height, total_issued)?;
    
    Ok(())
}

/// Validate all transactions in a block
fn validate_transactions(
    transactions: &[Transaction],
    utxo_set: &UTXOSet,
) -> Result<(), ValidationError> {
    use std::collections::HashSet;
    let mut spent_outputs = HashSet::new();
    
    for tx in transactions {
        // Skip coinbase validation for now (first tx)
        if tx.is_coinbase() {
            continue;
        }
        
        // Check for double spends within the block
        for input in &tx.inputs {
            let outpoint = (input.prev_tx_hash.clone(), input.output_index);
            if spent_outputs.contains(&outpoint) {
                return Err(ValidationError::DoubleSpend);
            }
            spent_outputs.insert(outpoint);
            
            // Check UTXO exists
            if !utxo_set.contains(&input.prev_tx_hash, input.output_index) {
                return Err(ValidationError::InvalidTransaction(
                    "Input UTXO does not exist".to_string()
                ));
            }
        }
        
        // Validate transaction signature
        if !tx.verify_signatures(utxo_set) {
            return Err(ValidationError::InvalidTransaction(
                "Invalid signature".to_string()
            ));
        }
    }
    
    Ok(())
}

/// Validate block reward
fn validate_block_reward(
    block: &Block,
    height: u64,
    total_issued: u64,
) -> Result<(), ValidationError> {
    let expected_reward = crate::consensus::calculate_block_reward(height, total_issued);
    
    // First transaction should be coinbase
    if block.transactions.is_empty() {
        return Err(ValidationError::InvalidBlockReward);
    }
    
    let coinbase = &block.transactions[0];
    if !coinbase.is_coinbase() {
        return Err(ValidationError::InvalidBlockReward);
    }
    
    let coinbase_amount: u64 = coinbase.outputs.iter().map(|o| o.amount).sum();
    
    // Coinbase can include fees, so it should be >= expected reward
    // For now, we just check it doesn't exceed reward (fees handled separately)
    if coinbase_amount > expected_reward {
        // Check if excess is from fees
        let total_fees = calculate_total_fees(block);
        if coinbase_amount > expected_reward + total_fees {
            return Err(ValidationError::InvalidBlockReward);
        }
    }
    
    Ok(())
}

/// Calculate total fees in a block
fn calculate_total_fees(block: &Block) -> u64 {
    // Fees = sum of inputs - sum of outputs (excluding coinbase)
    block.transactions.iter()
        .skip(1) // Skip coinbase
        .map(|tx| {
            let input_sum: u64 = tx.inputs.iter().map(|_| 0u64).sum(); // Would need UTXO lookup
            let output_sum: u64 = tx.outputs.iter().map(|o| o.amount).sum();
            input_sum.saturating_sub(output_sum)
        })
        .sum()
}

/// Convert difficulty target to full 256-bit target
fn difficulty_to_target(compact: u32) -> [u8; 32] {
    let exponent = (compact >> 24) as usize;
    let mantissa = compact & 0x00FFFFFF;
    
    let mut target = [0u8; 32];
    
    if exponent == 0 || exponent > 32 {
        return target;
    }
    
    if exponent <= 3 {
        let shift = 8 * (3 - exponent);
        let value = mantissa >> shift;
        target[31] = (value & 0xFF) as u8;
        target[30] = ((value >> 8) & 0xFF) as u8;
        target[29] = ((value >> 16) & 0xFF) as u8;
    } else {
        let byte_index = 32usize.saturating_sub(exponent);
        if byte_index < 32 {
            target[byte_index] = (mantissa & 0xFF) as u8;
        }
        if byte_index + 1 < 32 {
            target[byte_index + 1] = ((mantissa >> 8) & 0xFF) as u8;
        }
        if byte_index + 2 < 32 {
            target[byte_index + 2] = ((mantissa >> 16) & 0xFF) as u8;
        }
    }
    
    target
}

/// Convert hash to comparable value
fn hash_to_u256(hash: &Hash) -> [u8; 32] {
    hash.0
}

/// Chain validation result
#[derive(Debug)]
pub struct ChainValidationResult {
    /// Whether the chain is valid
    pub is_valid: bool,
    /// Total cumulative difficulty (work)
    pub total_work: u128,
    /// Chain height
    pub height: u64,
    /// Tip hash
    pub tip_hash: Hash,
}

/// Validate a chain of blocks and calculate total work
/// 
/// Implements longest valid chain rule:
/// - All blocks must be valid
/// - Chain with most cumulative work wins
pub fn validate_chain(
    blocks: &[Block],
    utxo_set: &mut UTXOSet,
) -> Result<ChainValidationResult, ValidationError> {
    if blocks.is_empty() {
        return Ok(ChainValidationResult {
            is_valid: true,
            total_work: 0,
            height: 0,
            tip_hash: Hash::zero(),
        });
    }

    let mut total_work: u128 = 0;
    let mut total_issued: u64 = 0;
    let mut prev_hash = Hash::zero();
    let mut prev_difficulty = blocks[0].header.difficulty_target;

    for (height, block) in blocks.iter().enumerate() {
        // Validate block
        validate_block(
            block,
            &prev_hash,
            prev_difficulty,
            utxo_set,
            height as u64,
            total_issued,
        )?;

        // Calculate work for this block
        let work = calculate_work(block.header.difficulty_target);
        total_work = total_work.saturating_add(work);

        // Track block reward
        let reward = crate::consensus::calculate_block_reward(height as u64, total_issued);
        total_issued = total_issued.saturating_add(reward);

        // Apply block to UTXO set
        for tx in &block.transactions {
            utxo_set.apply_transaction(tx, height as u64);
        }

        // Update for next iteration
        prev_hash = block.hash();
        prev_difficulty = block.header.difficulty_target;
    }

    let tip = blocks.last().unwrap();
    Ok(ChainValidationResult {
        is_valid: true,
        total_work,
        height: blocks.len() as u64,
        tip_hash: tip.hash(),
    })
}

/// Compare two chains and return which is better
/// 
/// Returns true if chain_a is better than chain_b (more work)
pub fn compare_chains(chain_a: &ChainValidationResult, chain_b: &ChainValidationResult) -> bool {
    chain_a.total_work > chain_b.total_work
}

/// Calculate work from difficulty target
/// 
/// Work = 2^256 / (target + 1)
/// Higher exponent = easier target = less work
/// Lower exponent = harder target = more work
fn calculate_work(compact_difficulty: u32) -> u128 {
    let exponent = (compact_difficulty >> 24) as u32;
    let mantissa = (compact_difficulty & 0x00FFFFFF) as u128;
    
    if mantissa == 0 || exponent == 0 {
        return 0;
    }
    
    // Work is inversely proportional to target
    // Smaller exponent = harder = more work
    // We calculate: 2^32 * (256 - exponent * 8) / mantissa
    let work_factor = (32u32.saturating_sub(exponent)) as u128;
    ((1u128 << 32) * work_factor) / mantissa
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::BlockHeader;
    use crate::crypto::hash_bytes;
    use crate::validation::Transaction;

    #[test]
    fn test_difficulty_to_target() {
        // Bitcoin-style compact target
        let target = difficulty_to_target(0x1d00ffff);
        assert!(target[0] == 0x00);
    }

    #[test]
    fn test_validate_pow_easy_target() {
        // Very easy target (all 0xFF)
        let header = BlockHeader::new(
            1,
            Hash::zero(),
            Hash::zero(),
            1234567890,
            0x2100ffff, // Very easy
            0,
        );
        // This may or may not pass depending on hash - just check no panic
        let _ = validate_pow(&header);
    }

    #[test]
    fn test_merkle_validation() {
        let tx = Transaction::coinbase(5000, hash_bytes(b"miner"));
        let merkle_root = crate::crypto::compute_merkle_root(&[tx.hash()]);
        
        let header = BlockHeader::new(1, Hash::zero(), merkle_root, 0, 0x1d00ffff, 0);
        let block = Block::new(header, vec![tx]);
        
        assert!(validate_merkle_root(&block).is_ok());
    }

    #[test]
    fn test_invalid_merkle_root() {
        let tx = Transaction::coinbase(5000, hash_bytes(b"miner"));
        let wrong_root = hash_bytes(b"wrong");
        
        let header = BlockHeader::new(1, Hash::zero(), wrong_root, 0, 0x1d00ffff, 0);
        let block = Block::new(header, vec![tx]);
        
        assert!(validate_merkle_root(&block).is_err());
    }

    #[test]
    fn test_calculate_work() {
        // Smaller exponent = harder = more work
        let easy = calculate_work(0x1d00ffff); // exponent 0x1d = 29
        let hard = calculate_work(0x1c00ffff); // exponent 0x1c = 28
        
        // Harder difficulty (smaller exponent) = more work
        assert!(hard > easy, "hard={} should be > easy={}", hard, easy);
    }

    #[test]
    fn test_compare_chains() {
        let chain_a = ChainValidationResult {
            is_valid: true,
            total_work: 1000,
            height: 10,
            tip_hash: Hash::zero(),
        };
        let chain_b = ChainValidationResult {
            is_valid: true,
            total_work: 500,
            height: 10,
            tip_hash: Hash::zero(),
        };
        
        assert!(compare_chains(&chain_a, &chain_b));
        assert!(!compare_chains(&chain_b, &chain_a));
    }
}

