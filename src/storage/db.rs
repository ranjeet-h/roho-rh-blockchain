//! Database persistence layer using Sled
//! 
//! Handles saving and loading chain state to disk.

use sled::{Db, Tree};
use crate::consensus::Block;
use crate::crypto::Hash;
use crate::storage::{UTXOSet, UTXO, UTXOKey};
use std::path::Path;

/// Database wrapper
#[derive(Debug, Clone)]
pub struct BlockChainDB {
    db: Db,
    blocks_tree: Tree,
    utxos_tree: Tree,
    metadata_tree: Tree,
}

const TIP_KEY: &str = "tip_hash";
const HEIGHT_KEY: &str = "height";
const TOTAL_ISSUED_KEY: &str = "total_issued";

impl BlockChainDB {
    /// Open or create the database
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let db = sled::open(path)?;
        let blocks_tree = db.open_tree("blocks")?;
        let utxos_tree = db.open_tree("utxos")?;
        let metadata_tree = db.open_tree("metadata")?;

        Ok(Self {
            db,
            blocks_tree,
            utxos_tree,
            metadata_tree,
        })
    }

    /// Save a block
    pub fn save_block(&self, block: &Block) -> std::io::Result<()> {
        let key = block.hash().0;
        let value = bincode::serialize(block).unwrap();
        self.blocks_tree.insert(key, value)?;
        self.db.flush()?;
        Ok(())
    }

    /// Get a block by hash
    pub fn get_block(&self, hash: &Hash) -> std::io::Result<Option<Block>> {
        match self.blocks_tree.get(hash.0)? {
            Some(bytes) => {
                let block = bincode::deserialize(&bytes).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e)
                })?;
                Ok(Some(block))
            },
            None => Ok(None),
        }
    }

    /// Save the UTXO set (differential update)
    pub fn update_utxos(&self, spent: &[UTXOKey], new_utxos: &[(UTXOKey, UTXO)]) -> std::io::Result<()> {
        // Remove spent
        for (tx_hash, index) in spent {
            let mut key = Vec::with_capacity(36);
            key.extend_from_slice(&tx_hash.0);
            key.extend_from_slice(&index.to_le_bytes());
            self.utxos_tree.remove(key)?;
        }

        // Add new
        for ((tx_hash, index), utxo) in new_utxos {
            let mut key = Vec::with_capacity(36);
            key.extend_from_slice(&tx_hash.0);
            key.extend_from_slice(&index.to_le_bytes());
            
            let value = bincode::serialize(utxo).unwrap();
            self.utxos_tree.insert(key, value)?;
        }

        self.db.flush()?;
        Ok(())
    }

    /// Load the entire UTXO set
    pub fn load_utxo_set(&self) -> std::io::Result<UTXOSet> {
        let mut set = UTXOSet::new();
        
        for item in self.utxos_tree.iter() {
            let (key, value) = item?;
            
            if key.len() != 36 { continue; }
            
            let mut hash_bytes = [0u8; 32];
            hash_bytes.copy_from_slice(&key[0..32]);
            let hash = Hash(hash_bytes);
            
            let mut idx_bytes = [0u8; 4];
            idx_bytes.copy_from_slice(&key[32..36]);
            let index = u32::from_le_bytes(idx_bytes);

            let utxo: UTXO = bincode::deserialize(&value).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, e)
            })?;

            set.add(hash, index, utxo);
        }

        Ok(set)
    }

    /// Update chain metadata
    pub fn update_metadata(&self, tip: &Hash, height: u64, total_issued: u64) -> std::io::Result<()> {
        self.metadata_tree.insert(TIP_KEY, tip.0.as_ref())?;
        self.metadata_tree.insert(HEIGHT_KEY, height.to_le_bytes().as_ref())?;
        self.metadata_tree.insert(TOTAL_ISSUED_KEY, total_issued.to_le_bytes().as_ref())?;
        self.db.flush()?;
        Ok(())
    }

    /// Load chain metadata
    pub fn load_metadata(&self) -> std::io::Result<Option<(Hash, u64, u64)>> {
        let tip_bytes = self.metadata_tree.get(TIP_KEY)?;
        let height_bytes = self.metadata_tree.get(HEIGHT_KEY)?;
        let issued_bytes = self.metadata_tree.get(TOTAL_ISSUED_KEY)?;

        if let (Some(tip), Some(height), Some(issued)) = (tip_bytes, height_bytes, issued_bytes) {
            let mut hash_bytes = [0u8; 32];
            hash_bytes.copy_from_slice(&tip);
            
            let mut h_bytes = [0u8; 8];
            h_bytes.copy_from_slice(&height);
            
            let mut i_bytes = [0u8; 8];
            i_bytes.copy_from_slice(&issued);

            Ok(Some((
                Hash(hash_bytes),
                u64::from_le_bytes(h_bytes),
                u64::from_le_bytes(i_bytes)
            )))
        } else {
            Ok(None)
        }
    }
}
