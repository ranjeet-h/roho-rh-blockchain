//! UTXO set implementation
//! 
//! In-memory database of unspent transaction outputs.

use std::collections::HashMap;
use crate::crypto::Hash;
use crate::validation::Transaction;
use serde::{Serialize, Deserialize};

/// Key for UTXO lookup: (tx_hash, output_index)
pub type UTXOKey = (Hash, u32);

/// Unspent Transaction Output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UTXO {
    /// Amount in base units
    pub amount: u64,
    /// Public key hash of owner
    pub pubkey_hash: Hash,
    /// Height at which this UTXO was created
    pub height: u64,
}

/// Set of all unspent transaction outputs
#[derive(Debug, Default)]
pub struct UTXOSet {
    /// Map from (tx_hash, output_index) to UTXO
    utxos: HashMap<UTXOKey, UTXO>,
}

impl UTXOSet {
    /// Create a new empty UTXO set
    pub fn new() -> Self {
        Self {
            utxos: HashMap::new(),
        }
    }

    /// Check if a UTXO exists
    pub fn contains(&self, tx_hash: &Hash, output_index: u32) -> bool {
        self.utxos.contains_key(&(*tx_hash, output_index))
    }

    /// Get a UTXO if it exists
    pub fn get(&self, tx_hash: &Hash, output_index: u32) -> Option<&UTXO> {
        self.utxos.get(&(*tx_hash, output_index))
    }

    /// Add a UTXO
    pub fn add(&mut self, tx_hash: Hash, output_index: u32, utxo: UTXO) {
        self.utxos.insert((tx_hash, output_index), utxo);
    }

    /// Remove a UTXO (when spent)
    pub fn remove(&mut self, tx_hash: &Hash, output_index: u32) -> Option<UTXO> {
        self.utxos.remove(&(*tx_hash, output_index))
    }

    /// Apply a transaction to the UTXO set
    /// 
    /// Removes spent outputs and adds new outputs.
    pub fn apply_transaction(&mut self, tx: &Transaction, height: u64) {
        let tx_hash = tx.hash();

        // Remove spent outputs (skip for coinbase)
        if !tx.is_coinbase() {
            for input in &tx.inputs {
                self.remove(&input.prev_tx_hash, input.output_index);
            }
        }

        // Add new outputs
        for (index, output) in tx.outputs.iter().enumerate() {
            self.add(
                tx_hash,
                index as u32,
                UTXO {
                    amount: output.amount,
                    pubkey_hash: output.pubkey_hash,
                    height,
                },
            );
        }
    }

    /// Revert a transaction from the UTXO set
    /// 
    /// Adds back spent outputs and removes created outputs.
    /// Requires the previous UTXOs to be provided.
    pub fn revert_transaction(&mut self, tx: &Transaction, spent_utxos: &[(UTXOKey, UTXO)]) {
        let tx_hash = tx.hash();

        // Remove created outputs
        for (index, _) in tx.outputs.iter().enumerate() {
            self.remove(&tx_hash, index as u32);
        }

        // Add back spent outputs
        for (key, utxo) in spent_utxos {
            self.add(key.0, key.1, utxo.clone());
        }
    }

    /// Get all UTXOs for a given public key hash
    /// Note: Addresses encode only first 20 bytes, so we compare those
    pub fn get_by_pubkey_hash(&self, pubkey_hash: &Hash) -> Vec<(UTXOKey, &UTXO)> {
        self.utxos
            .iter()
            .filter(|(_, utxo)| {
                // Compare first 20 bytes (address-encoded portion)
                utxo.pubkey_hash.0[0..20] == pubkey_hash.0[0..20]
            })
            .map(|(key, utxo)| (*key, utxo))
            .collect()
    }

    /// Get total balance for a public key hash
    pub fn get_balance(&self, pubkey_hash: &Hash) -> u64 {
        self.get_by_pubkey_hash(pubkey_hash)
            .iter()
            .map(|(_, utxo)| utxo.amount)
            .sum()
    }

    /// Get total number of UTXOs
    pub fn len(&self) -> usize {
        self.utxos.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.utxos.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::hash_bytes;

    fn make_hash(s: &str) -> Hash {
        hash_bytes(s.as_bytes())
    }

    #[test]
    fn test_utxo_add_and_get() {
        let mut set = UTXOSet::new();
        let tx_hash = make_hash("tx1");
        
        set.add(tx_hash, 0, UTXO {
            amount: 100,
            pubkey_hash: make_hash("owner"),
            height: 1,
        });

        assert!(set.contains(&tx_hash, 0));
        assert!(!set.contains(&tx_hash, 1));

        let utxo = set.get(&tx_hash, 0).unwrap();
        assert_eq!(utxo.amount, 100);
    }

    #[test]
    fn test_utxo_remove() {
        let mut set = UTXOSet::new();
        let tx_hash = make_hash("tx1");
        
        set.add(tx_hash, 0, UTXO {
            amount: 100,
            pubkey_hash: make_hash("owner"),
            height: 1,
        });

        assert!(set.contains(&tx_hash, 0));
        
        let removed = set.remove(&tx_hash, 0);
        assert!(removed.is_some());
        assert!(!set.contains(&tx_hash, 0));
    }

    #[test]
    fn test_get_balance() {
        let mut set = UTXOSet::new();
        let owner = make_hash("owner");

        set.add(make_hash("tx1"), 0, UTXO {
            amount: 100,
            pubkey_hash: owner,
            height: 1,
        });

        set.add(make_hash("tx2"), 0, UTXO {
            amount: 200,
            pubkey_hash: owner,
            height: 2,
        });

        set.add(make_hash("tx3"), 0, UTXO {
            amount: 50,
            pubkey_hash: make_hash("other"),
            height: 3,
        });

        assert_eq!(set.get_balance(&owner), 300);
    }

    #[test]
    fn test_apply_coinbase() {
        let mut set = UTXOSet::new();
        let miner = make_hash("miner");
        
        let coinbase = Transaction::coinbase(5000, miner);
        set.apply_transaction(&coinbase, 1);

        let tx_hash = coinbase.hash();
        assert!(set.contains(&tx_hash, 0));
        assert_eq!(set.get_balance(&miner), 5000);
    }
}
