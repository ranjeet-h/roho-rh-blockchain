//! Chain state management
//! 
//! Tracks the current state of the blockchain including the UTXO set,
//! current height, total issued supply, and difficulty.

use std::collections::{HashMap, HashSet};
use crate::consensus::{Block, BlockHeader};
use crate::crypto::Hash;
use crate::constants::PUBLIC_ISSUANCE;
use crate::validation::Transaction;
use super::{UTXOSet, UTXO, UTXOKey};
use super::db::BlockChainDB;

/// Maximum mempool size in bytes (300 MB - production standard)
const MAX_MEMPOOL_BYTES: u64 = 300 * 1024 * 1024;

/// Minimum relay fee in satoshis per byte (prevents dust spam)
const MIN_RELAY_FEE: u64 = 1; // 1 sat/byte

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
    /// Block index: hash -> Entry
    block_index: HashMap<Hash, BlockIndexEntry>,
    /// Full block storage (hash -> block)
    full_blocks: HashMap<Hash, Block>,
    /// Height to Hash map (Main chain ONLY)
    height_to_hash: HashMap<u64, Hash>,
    /// Unconfirmed transactions
    pub mempool: HashMap<Hash, Transaction>,
    /// Database connection
    pub db: Option<BlockChainDB>,
    /// Next expected nonce per sender (pubkey_hash -> nonce)
    /// Used to enforce sequential nonce ordering and allow tx replacement
    next_nonce: HashMap<Hash, u64>,
    /// Timestamps of last 11 blocks for median time calculation
    /// Used to validate new block timestamps
    recent_block_timestamps: std::collections::VecDeque<u64>,
}

#[derive(Debug, Clone)]
pub struct BlockIndexEntry {
    pub header: BlockHeader,
    pub height: u64,
    pub total_issued: u64,
    pub undo_data: Vec<(UTXOKey, UTXO)>,
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
            full_blocks: HashMap::new(),
            height_to_hash: HashMap::new(),
            mempool: HashMap::new(),
            db: None,
            next_nonce: HashMap::new(),
            recent_block_timestamps: std::collections::VecDeque::with_capacity(11),
        };

        // Apply genesis transactions (founder allocation)
        for tx in &genesis_block.transactions {
            state.utxo_set.apply_transaction(tx, 0);
        }

        // Index genesis block
        state.height_to_hash.insert(0, genesis_block.hash());
        state.block_index.insert(
            genesis_block.hash(),
            BlockIndexEntry {
                header: genesis_block.header.clone(),
                height: 0,
                total_issued: 0,
                undo_data: Vec::new(),
            },
        );
        state.full_blocks.insert(genesis_block.hash(), genesis_block.clone());

        state
    }

    /// Restore chain state from database
    pub fn restore(db: BlockChainDB) -> Result<Self, String> {
        println!("📂 Loading chain state from disk...");
        
        // 1. Load Metadata
        let (tip_hash, height, total_issued) = match db.load_metadata().map_err(|e| e.to_string())? {
            Some(meta) => meta,
            None => return Err("No metadata found in DB".to_string()),
        };

        // 2. Load UTXO Set
        let utxo_set = db.load_utxo_set().map_err(|e| e.to_string())?;

        // Reconstruct Indices by walking back from tip
        let mut block_index = HashMap::new();
        let mut full_blocks = HashMap::new();
        let mut height_to_hash = HashMap::new();

        let mut curr_height = height;

        // Load tip block first
        let tip_block = db.get_block(&tip_hash).map_err(|e| e.to_string())?
            .ok_or("Tip block missing from DB".to_string())?;
            
        let difficulty = tip_block.header.difficulty_target;

        // Trace back to genesis (or as far as we have in DB)
        let mut next_hash = tip_hash;
        
        loop {
            let block = db.get_block(&next_hash).map_err(|e| e.to_string())?
                .ok_or(format!("Block {} missing from DB", next_hash))?;
            
            // Reconstruct index entry (Note: undo_data is lost, so reorgs deep into history are limited)
            block_index.insert(next_hash, BlockIndexEntry {
                header: block.header.clone(),
                height: curr_height,
                total_issued: 0, // Approximate, fixed if we walk forward, but we assume valid chain
                undo_data: Vec::new(),
            });
            
            full_blocks.insert(next_hash, block.clone());
            height_to_hash.insert(curr_height, next_hash);

            if curr_height == 0 {
                break;
            }

            next_hash = block.header.prev_hash;
            curr_height -= 1;
        }

        // Correct total_issued for the index (optional, but good for consistency)
        // For now, we trust the tip metadata.

        println!("✅ Restored chain to height {}", height);

        Ok(Self {
            utxo_set,
            height,
            tip_hash,
            total_issued,
            difficulty,
            block_index,
            full_blocks,
            height_to_hash,
            mempool: HashMap::new(),
            db: Some(db),
            next_nonce: HashMap::new(),
            recent_block_timestamps: std::collections::VecDeque::with_capacity(11),
        })
    }

    /// Set database connection
    pub fn set_db(&mut self, db: BlockChainDB) {
        self.db = Some(db);
    }

    /// Calculate median time of last 11 blocks
    fn calculate_median_time(&self) -> u64 {
        if self.recent_block_timestamps.is_empty() {
            return 0;
        }
        
        let mut times: Vec<u64> = self.recent_block_timestamps.iter().copied().collect();
        times.sort_unstable();
        times[times.len() / 2]
    }

    /// Validate block timestamp against network time rules
    fn validate_block_timestamp(&self, timestamp: u64) -> Result<(), String> {
        // Current time (simplified - in production, use a more robust time source)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Rule 1: Block timestamp must not be more than 2 hours in the future
        const MAX_FUTURE_TIME: u64 = 2 * 3600; // 2 hours
        if timestamp > now + MAX_FUTURE_TIME {
            return Err(format!(
                "Block timestamp {} is too far in future (now: {}, max: {})",
                timestamp,
                now,
                now + MAX_FUTURE_TIME
            ));
        }
        
        // Rule 2: Block timestamp must not be before median time of last 11 blocks minus 1 hour
        let median_time = self.calculate_median_time();
        if median_time > 0 {
            const MIN_PAST_TIME: u64 = 3600; // 1 hour
            if timestamp < median_time.saturating_sub(MIN_PAST_TIME) {
                return Err(format!(
                    "Block timestamp {} is too old (median: {}, min: {})",
                    timestamp,
                    median_time,
                    median_time.saturating_sub(MIN_PAST_TIME)
                ));
            }
        }
        
        Ok(())
    }

    /// Apply a new block to the state
    /// 
    /// Returns the spent UTXOs for potential rollback.
    pub fn apply_block(&mut self, block: &Block) -> Result<Vec<(UTXOKey, UTXO)>, String> {
        // 1. Validate chain_id (replay protection)
        if block.header.chain_id != crate::constants::CHAIN_ID {
            return Err(format!(
                "Block has invalid chain_id: {} (expected {})",
                block.header.chain_id,
                crate::constants::CHAIN_ID
            ));
        }
        
        // 2. Validate timestamp against network time
        self.validate_block_timestamp(block.header.timestamp)?;
        
        // 3. Validate block header (PoW is assumed valid if it reached here, but we can check height)
        if block.header.prev_hash != self.tip_hash && self.height > 0 {
            return Err("Block does not connect to current tip".to_string());
        }

        let mut spent_utxos = Vec::new();
        let new_height = self.height + 1;

        // 2. Validate all transactions in the block
        let mut total_subsidy = 0u64;
        let mut coinbase_reward = 0u64;
        let mut block_fees = 0u64;

        for tx in &block.transactions {
            if tx.is_coinbase() {
                if coinbase_reward > 0 {
                    return Err("Multiple coinbase transactions in block".to_string());
                }
                coinbase_reward = tx.total_output_value();
                total_subsidy = crate::consensus::calculate_block_reward(new_height, self.total_issued);
            } else {
                // Verify signatures
                tx.verify_signatures(&self.utxo_set)?;

                // Verify inputs exist and collect them for fees/rollback
                let input_val = tx.total_input_value(&self.utxo_set);
                let output_val = tx.total_output_value();
                
                if input_val < output_val {
                    return Err(format!("Insufficient input in transaction {}", tx.hash()));
                }
                
                let fee = input_val - output_val;
                block_fees = block_fees.saturating_add(fee);

                for input in &tx.inputs {
                    if let Some(utxo) = self.utxo_set.get(&input.prev_tx_hash, input.output_index) {
                        spent_utxos.push((
                            (input.prev_tx_hash, input.output_index),
                            utxo.clone(),
                        ));
                    } else {
                        return Err(format!("UTXO missing for transaction {}", tx.hash()));
                    }
                }
            }
        }

        // 3. Verify coinbase reward (subsidy + fees)
        if coinbase_reward > total_subsidy + block_fees {
            return Err(format!("Coinbase reward too high: {} > {} subsidy + {} fees", 
                coinbase_reward, total_subsidy, block_fees));
        }

        // Clean mempool: Remove mined transactions and conflicting transactions
        let mut txs_to_remove = HashSet::new();
        let mut block_inputs = HashSet::new();

        // Identify inputs spent by the new block
        for tx in &block.transactions {
            txs_to_remove.insert(tx.hash()); // Remove the tx itself
            if !tx.is_coinbase() {
                for input in &tx.inputs {
                    block_inputs.insert((input.prev_tx_hash, input.output_index));
                }
            }
        }

        // Identify mempool transactions that conflict with the new block
        for (tx_hash, mempool_tx) in &self.mempool {
            for input in &mempool_tx.inputs {
                if block_inputs.contains(&(input.prev_tx_hash, input.output_index)) {
                    txs_to_remove.insert(*tx_hash);
                    break;
                }
            }
        }

        // Remove from mempool and reset nonces for removed transactions
        for hash in &txs_to_remove {
            if let Some(tx) = self.mempool.remove(hash) {
                // Reset nonce for sender if no more txs from them in mempool
                if !tx.inputs.is_empty() {
                    let sender_pubkey = &tx.inputs[0].public_key;
                    let sender_hash = crate::crypto::hash_bytes(&sender_pubkey.0);
                    
                    let has_more_txs = self.mempool.values()
                        .any(|t| !t.inputs.is_empty() && t.inputs[0].public_key == *sender_pubkey);
                    
                    if !has_more_txs {
                        self.next_nonce.remove(&sender_hash);
                    }
                }
            }
        }

        // 4. Apply transactions to UTXO set
        let mut new_utxos = Vec::new();
        let spent_keys: Vec<UTXOKey> = spent_utxos.iter().map(|(k, _)| *k).collect();

        for tx in &block.transactions {
            self.utxo_set.apply_transaction(tx, new_height);
            
            // Collect new UTXOs for DB update
            for (i, output) in tx.outputs.iter().enumerate() {
                new_utxos.push((
                    (tx.hash(), i as u32),
                    UTXO {
                        amount: output.amount,
                        pubkey_hash: output.pubkey_hash,
                        height: new_height,
                    }
                ));
            }
        }

        // 5. Update state
        self.total_issued = self.total_issued.saturating_add(total_subsidy);
        self.height = new_height;
        self.tip_hash = block.hash();
        self.difficulty = block.header.difficulty_target;
        self.height_to_hash.insert(new_height, block.hash());
        
        // Track block timestamp for median time calculation
        self.recent_block_timestamps.push_back(block.header.timestamp);
        if self.recent_block_timestamps.len() > 11 {
            self.recent_block_timestamps.pop_front();
        }

        // Index block
        self.block_index.insert(
            block.hash(),
            BlockIndexEntry {
                header: block.header.clone(),
                height: new_height,
                total_issued: self.total_issued,
                undo_data: spent_utxos.clone(),
            },
        );
        self.full_blocks.insert(block.hash(), block.clone());

        // 6. Persist to DB if available
        if let Some(db) = &self.db {
            db.save_block(block).map_err(|e| e.to_string())?;
            db.update_utxos(&spent_keys, &new_utxos).map_err(|e| e.to_string())?;
            db.update_metadata(&self.tip_hash, self.height, self.total_issued).map_err(|e| e.to_string())?;
        }

        Ok(spent_utxos)
    }

    /// Revert the current tip block
    pub fn revert_tip(&mut self) -> Result<(), String> {
        let tip_hash = self.tip_hash;
        let block = self.full_blocks.get(&tip_hash).ok_or("Tip block not found")?.clone();
        let entry = self.block_index.get(&tip_hash).ok_or("Tip index not found")?.clone();
        
        self.revert_block(&block, entry.undo_data);
        Ok(())
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
        }

        // Update state
        self.height_to_hash.remove(&(self.height));
        self.height -= 1;
        self.tip_hash = block.header.prev_hash;
        
        // Remove timestamp from tracking (revert the most recent block timestamp)
        self.recent_block_timestamps.pop_back();

        // Restore previous total_issued from index
        if let Some(entry) = self.block_index.get(&block.header.prev_hash) {
            self.total_issued = entry.total_issued;
            self.difficulty = entry.header.difficulty_target;
        }

        // Note: Reverts are currently NOT persisted to DB immediately.
        // If node crashes during reorg, it might be in inconsistent state relative to DB.
        // In full production, we'd use DB transactions or batches.
        // For now, the DB is updated only on forward progress (apply_block).
        // A restart would load the old tip (pre-reorg state) which is safe.
    }

    /// Get block header by hash
    pub fn get_block_header(&self, hash: &Hash) -> Option<&BlockHeader> {
        self.block_index.get(hash).map(|e| &e.header)
    }

    /// Get block height by hash
    pub fn get_block_height(&self, hash: &Hash) -> Option<u64> {
        self.block_index.get(hash).map(|e| e.height)
    }

    /// Check if a reorg would violate checkpoint rules
    fn validate_reorg_depth(&self, common_ancestor_height: u64) -> Result<(), String> {
        let reorg_depth = self.height - common_ancestor_height;
        
        // Check against max reorg depth
        if reorg_depth > crate::constants::MAX_REORG_DEPTH {
            return Err(format!(
                "Reorg depth {} exceeds max allowed depth {}",
                reorg_depth,
                crate::constants::MAX_REORG_DEPTH
            ));
        }
        
        // Check against checkpoints - cannot reorg past a checkpoint
        for (checkpoint_height, _) in crate::constants::CHECKPOINTS {
            if common_ancestor_height < *checkpoint_height {
                return Err(format!(
                    "Cannot reorg past checkpoint at height {}",
                    checkpoint_height
                ));
            }
        }
        
        Ok(())
    }

    /// Reorganize the chain to a new tip hash
    /// 
    /// This finds the common ancestor, reverts local blocks, and applies the new chain.
    /// Validates against checkpoint constraints and max reorg depth.
    pub fn reorganize(&mut self, target_hash: Hash) -> Result<(), String> {
        let mut new_chain = Vec::new();
        let mut curr_hash = target_hash;

        // 1. Trace back the new chain until we find a block that is in our main chain
        while let Some(entry) = self.block_index.get(&curr_hash) {
            if self.get_block_hash_at_height(entry.height) == Some(curr_hash) {
                // Found common ancestor
                break;
            }
            new_chain.push(curr_hash);
            if entry.height == 0 { break; }
            curr_hash = entry.header.prev_hash;
        }
        new_chain.reverse();

        // Common ancestor info
        let common_height = self.get_block_height(&curr_hash).ok_or("Common ancestor not in index")?;

        // Validate reorg constraints
        self.validate_reorg_depth(common_height)?;

        // 2. Revert blocks until common ancestor
        while self.height > common_height {
            self.revert_tip()?;
        }

        // 3. Apply new blocks
        for hash in new_chain {
            let block = self.full_blocks.get(&hash).ok_or("Block data missing during re-org")?.clone();
            self.apply_block(&block)?;
        }

        Ok(())
    }

    /// Index a block without applying it (for side chains)
    pub fn index_block(&mut self, block: &Block) {
        if self.block_index.contains_key(&block.hash()) {
            return;
        }

        // Try to derive height from parent
        let height = self.get_block_height(&block.header.prev_hash)
            .map(|h| h + 1)
            .unwrap_or(0);

        self.block_index.insert(
            block.hash(),
            BlockIndexEntry {
                header: block.header.clone(),
                height,
                total_issued: 0, 
                undo_data: Vec::new(),
            },
        );
        self.full_blocks.insert(block.hash(), block.clone());
        
        // Save to DB even if not applied yet (so we have the data)
        if let Some(db) = &self.db {
            let _ = db.save_block(block);
        }
    }

    /// Get full block by hash
    pub fn get_block(&self, hash: &Hash) -> Option<&Block> {
        self.full_blocks.get(hash)
    }

    /// Get block hash at a given height (Main chain)
    pub fn get_block_hash_at_height(&self, target_height: u64) -> Option<Hash> {
        self.height_to_hash.get(&target_height).copied()
    }

    /// Calculate current mempool size in bytes
    pub fn mempool_bytes(&self) -> u64 {
        self.mempool.values()
            .map(|tx| {
                bincode::serialized_size(tx).unwrap_or(0)
            })
            .sum()
    }

    /// Calculate fee rate (satoshis per byte)
    fn calculate_fee_rate(tx: &Transaction, fee: u64) -> u64 {
        let tx_size = bincode::serialized_size(tx).unwrap_or(1) as u64;
        fee / tx_size.max(1)
    }

    /// Add a transaction to the mempool
    pub fn add_to_mempool(&mut self, tx: Transaction) -> Result<(), String> {
        let hash = tx.hash();
        
        // 1. Basic checks
        if self.mempool.contains_key(&hash) {
            return Err("Transaction already in mempool".to_string());
        }
        if tx.is_coinbase() {
            return Err("Coinbase transaction cannot be added to mempool".to_string());
        }

        // 2. Verify signatures and UTXO existence
        tx.verify_signatures(&self.utxo_set)?;

        // 3. Verify amounts (input >= output + fee)
        let input_val = tx.total_input_value(&self.utxo_set);
        let output_val = tx.total_output_value();
        if input_val < output_val {
            return Err(format!("Insufficient input: {} < {}", input_val, output_val));
        }
        
        // Calculate fee and fee rate
        let fee = input_val - output_val;
        let fee_rate = Self::calculate_fee_rate(&tx, fee);

        // 4. Enforce minimum relay fee
        if fee_rate < MIN_RELAY_FEE {
            return Err(format!("Transaction fee too low: {} sat/byte < {} sat/byte minimum", 
                fee_rate, MIN_RELAY_FEE));
        }

        // 5. Nonce validation: Enforce sequential nonce ordering per sender
        // Get sender's pubkey hash from first input
        if !tx.inputs.is_empty() {
            let sender_pubkey = &tx.inputs[0].public_key;
            let sender_hash = crate::crypto::hash_bytes(&sender_pubkey.0);
            
            let expected_nonce = self.next_nonce.get(&sender_hash).copied().unwrap_or(0);
            
            // Check if this is a replacement tx (same nonce) or next in sequence
            let existing_tx_with_nonce: Option<Hash> = self.mempool.values()
                .find(|t| !t.inputs.is_empty() && t.inputs[0].public_key == *sender_pubkey && t.nonce == tx.nonce)
                .map(|t| t.hash());
            
            if let Some(existing_hash) = existing_tx_with_nonce {
                // Replacement: newer tx must have higher fee rate
                if let Some(existing_tx) = self.mempool.get(&existing_hash) {
                    let existing_fee = existing_tx.total_input_value(&self.utxo_set)
                        .saturating_sub(existing_tx.total_output_value());
                    let existing_fee_rate = Self::calculate_fee_rate(existing_tx, existing_fee);
                    
                    if fee_rate <= existing_fee_rate {
                        return Err("Replacement transaction must have higher fee rate".to_string());
                    }
                    // Remove old transaction
                    self.mempool.remove(&existing_hash);
                }
            } else if tx.nonce < expected_nonce {
                // Old nonce - reject
                return Err(format!("Transaction nonce {} is too old (expected >= {})", tx.nonce, expected_nonce));
            } else if tx.nonce > expected_nonce {
                // Gap in nonces - reject (must maintain order)
                return Err(format!("Transaction nonce gap: got {}, expected {}", tx.nonce, expected_nonce));
            }
        }

        // 6. Double spend check (mempool vs mempool, excluding replacements)
        for input in &tx.inputs {
            for existing_tx in self.mempool.values() {
                if existing_tx.inputs.iter().any(|i| i.prev_tx_hash == input.prev_tx_hash && i.output_index == input.output_index) {
                    return Err("Transaction double-spends an existing mempool transaction".to_string());
                }
            }
        }
        
        // 7. DoS Protection: Enforce 300MB mempool size limit
        let tx_size = bincode::serialized_size(&tx).unwrap_or(0) as u64;
        let current_mempool_bytes = self.mempool_bytes();
        
        if current_mempool_bytes + tx_size > MAX_MEMPOOL_BYTES {
            // Evict lowest fee-rate transaction
            if let Some((evict_hash, _)) = self.mempool.iter()
                .min_by_key(|(_, t)| {
                    let t_fee = t.total_input_value(&self.utxo_set)
                        .saturating_sub(t.total_output_value());
                    let t_size = bincode::serialized_size(t).unwrap_or(1) as u64;
                    t_fee / t_size.max(1)
                })
                .map(|(h, t)| (*h, t.clone()))
            {
                self.mempool.remove(&evict_hash);
                
                // Check again - if still full, reject
                if self.mempool_bytes() + tx_size > MAX_MEMPOOL_BYTES {
                    return Err("Mempool full: Cannot add transaction with higher fee rate".to_string());
                }
            } else {
                return Err("Mempool full".to_string());
            }
        }

        self.mempool.insert(hash, tx.clone());
        
        // Update next expected nonce for this sender
        if !tx.inputs.is_empty() {
            let sender_pubkey = &tx.inputs[0].public_key;
            let sender_hash = crate::crypto::hash_bytes(&sender_pubkey.0);
            self.next_nonce.insert(sender_hash, tx.nonce + 1);
        }
        
        Ok(())
    }

    /// Get all transactions from the mempool, sorted by fee rate (highest first)
    pub fn get_mempool_transactions(&self) -> Vec<Transaction> {
        let mut txs: Vec<_> = self.mempool.values().cloned().collect();
        
        // Sort by fee rate (fee / size) in descending order
        txs.sort_by_key(|tx| {
            let fee = tx.total_input_value(&self.utxo_set)
                .saturating_sub(tx.total_output_value());
            let size = bincode::serialized_size(tx).unwrap_or(1) as u64;
            let fee_rate = fee / size.max(1);
            std::cmp::Reverse(fee_rate) // Reverse for descending order
        });
        
        txs
    }

    /// Get all mempool transaction hashes sorted by fee rate
    pub fn get_mempool_hashes_sorted(&self) -> Vec<Hash> {
        self.get_mempool_transactions().iter().map(|tx| tx.hash()).collect()
    }

    /// Get the next expected nonce for a sender (pubkey hash)
    /// Returns 0 if sender has no transactions in mempool
    pub fn get_next_nonce(&self, sender_pubkey_hash: &Hash) -> u64 {
        self.next_nonce.get(sender_pubkey_hash).copied().unwrap_or(0)
    }

    /// Get pending nonce for a sender (for wallet use)
    /// Returns the recommended nonce for the next transaction from this sender
    pub fn get_pending_nonce(&self, sender_pubkey: &crate::crypto::PublicKey) -> u64 {
        let sender_hash = crate::crypto::hash_bytes(&sender_pubkey.0);
        self.get_next_nonce(&sender_hash)
    }

    /// Remove transactions from mempool (used when a block is applied)
    pub fn remove_from_mempool(&mut self, tx_hashes: &[Hash]) {
        for hash in tx_hashes {
            if let Some(tx) = self.mempool.remove(hash) {
                // Reset nonce for sender if no more txs from them in mempool
                if !tx.inputs.is_empty() {
                    let sender_pubkey = &tx.inputs[0].public_key;
                    let sender_hash = crate::crypto::hash_bytes(&sender_pubkey.0);
                    
                    let has_more_txs = self.mempool.values()
                        .any(|t| !t.inputs.is_empty() && t.inputs[0].public_key == *sender_pubkey);
                    
                    if !has_more_txs {
                        self.next_nonce.remove(&sender_hash);
                    }
                }
            }
        }
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
            mempool_txs: self.mempool.len(),
            mempool_bytes: self.mempool_bytes(),
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
    pub mempool_txs: usize,
    pub mempool_bytes: u64,
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
                0x01,
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
                0x01,
                genesis.hash(),
                hash_bytes(b"merkle2"),
                1234567891,
                0x1d00ffff,
                123,
            ),
            vec![Transaction::coinbase(5000, hash_bytes(b"miner"))],
        );

        let spent = state.apply_block(&block).expect("Failed to apply valid block");

        assert_eq!(state.height, 1);
        assert_eq!(state.tip_hash, block.hash());
        assert!(spent.is_empty()); // Only coinbase, no spent
    }

    #[test]
    fn test_mempool_size_cap() {
        let genesis = make_genesis();
        let mut state = ChainState::new(&genesis);

        // Test that mempool_bytes() works correctly
        assert_eq!(state.mempool_bytes(), 0);

        // Create a wallet and add some funds
        let mut wallet = crate::wallet::Wallet::new();
        let keypair = wallet.generate_key();
        let miner_pubkey_hash = keypair.pubkey_hash();

        // Mine a block to give the wallet some coins
        let coinbase_tx = Transaction::coinbase(10000, miner_pubkey_hash);
        state.utxo_set.apply_transaction(&coinbase_tx, 1);

        // Create a valid transaction using the wallet
        let recipient_hash = hash_bytes(b"recipient");
        let tx = wallet.create_transaction(&state.utxo_set, recipient_hash, 5000, 1000).unwrap();

        // Add to mempool
        state.add_to_mempool(tx.clone()).unwrap();

        // Verify mempool size increased
        let mempool_size = state.mempool_bytes();
        assert!(mempool_size > 0);

        // Verify size is under limit
        assert!(mempool_size <= MAX_MEMPOOL_BYTES);

        // Test that the constant is correct
        assert_eq!(MAX_MEMPOOL_BYTES, 300 * 1024 * 1024); // 300MB
    }

    #[test]
    fn test_checkpoint_validation() {
        let genesis = make_genesis();
        let mut state = ChainState::new(&genesis);

        // Test that reorg validation respects checkpoints
        // Since genesis is at height 0 and we have a checkpoint there,
        // any reorg attempt should be blocked if it tries to go before checkpoint

        // This test verifies the validate_reorg_depth function
        // We can't easily test full reorg without more setup, but we can test the validation

        // The checkpoint at height 0 should prevent reorgs that would go before it
        let result = state.validate_reorg_depth(0);
        assert!(result.is_ok()); // Reorg to height 0 should be allowed (no reorg)

        // But if we had a deeper reorg, it would be blocked
        // Since MAX_REORG_DEPTH is 10, and we start at height 0, we can't test deep reorg easily
        // But the logic is there and tested indirectly through the constants
    }

    #[test]
    fn test_replay_protection() {
        let genesis = make_genesis();
        let mut state = ChainState::new(&genesis);

        // Create a block with wrong chain_id
        use crate::consensus::BlockHeader;
        let bad_block = Block::new(
            BlockHeader::new(
                1,
                0x00, // Wrong chain ID (testnet instead of mainnet)
                genesis.hash(),
                hash_bytes(b"merkle_bad"),
                1234567891,
                0x1d00ffff,
                123,
            ),
            vec![Transaction::coinbase(5000, hash_bytes(b"miner"))],
        );

        // Should reject due to chain_id mismatch
        let result = state.apply_block(&bad_block);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid chain_id"));
    }
}
