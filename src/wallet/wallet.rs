//! Wallet implementation
//! 
//! Handles key generation, UTXO tracking, and transaction signing.
//! The wallet does NOT affect consensus - bugs here cannot affect supply.

use crate::crypto::{Hash, PrivateKey, PublicKey, hash_bytes};
use crate::storage::{UTXOSet, UTXO, UTXOKey};
use crate::validation::{Transaction, TxInput, TxOutput};
use thiserror::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// Wallet errors
#[derive(Debug, Error)]
pub enum WalletError {
    #[error("Insufficient funds: have {have}, need {need}")]
    InsufficientFunds { have: u64, need: u64 },
    #[error("No UTXOs available")]
    NoUTXOs,
    #[error("Signing error: {0}")]
    SigningError(String),
    #[error("Invalid address")]
    InvalidAddress,
}

/// A wallet key pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPair {
    /// Private key (for signing)
    private_key: PrivateKey,
    /// Public key
    pub public_key: PublicKey,
    /// Address (derived from public key)
    pub address: String,
}

impl KeyPair {
    /// Generate a new random keypair
    pub fn generate() -> Self {
        let private_key = PrivateKey::generate();
        let public_key = private_key.public_key();
        let address = public_key.to_address();

        Self {
            private_key,
            public_key,
            address,
        }
    }

    /// Import from private key bytes
    pub fn from_private_key_bytes(bytes: &[u8; 32]) -> Result<Self, WalletError> {
        let private_key = PrivateKey::from_bytes(bytes)
            .map_err(|_| WalletError::SigningError("Invalid private key".to_string()))?;
        let public_key = private_key.public_key();
        let address = public_key.to_address();

        Ok(Self {
            private_key,
            public_key,
            address,
        })
    }

    /// Export private key bytes
    pub fn private_key_bytes(&self) -> [u8; 32] {
        self.private_key.to_bytes()
    }

    /// Get the public key hash (used in outputs)
    /// Returns the first 20 bytes of the BLAKE3 hash, padded to 32 bytes.
    /// This matches the address encoding format.
    pub fn pubkey_hash(&self) -> Hash {
        let full_hash = hash_bytes(&self.public_key.0);
        let mut addr_hash = [0u8; 32];
        addr_hash[0..20].copy_from_slice(&full_hash.0[0..20]);
        Hash(addr_hash)
    }

    /// Sign a message
    pub fn sign(&self, message: &Hash) -> Result<crate::crypto::SchnorrSignature, WalletError> {
        self.private_key.sign(message)
            .map_err(|e| WalletError::SigningError(e.to_string()))
    }
}

/// A simple wallet
#[derive(Debug, Serialize, Deserialize)]
pub struct Wallet {
    /// Wallet keys (pubkey_hash -> keypair)
    keys: HashMap<Hash, KeyPair>,
}

impl Default for Wallet {
    fn default() -> Self {
        Self::new()
    }
}

impl Wallet {
    /// Create a new empty wallet
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }

    /// Generate a new key and add to wallet
    pub fn generate_key(&mut self) -> &KeyPair {
        let keypair = KeyPair::generate();
        let pubkey_hash = keypair.pubkey_hash();
        self.keys.insert(pubkey_hash, keypair);
        self.keys.get(&pubkey_hash).unwrap()
    }

    /// Import a key
    pub fn import_key(&mut self, bytes: &[u8; 32]) -> Result<&KeyPair, WalletError> {
        let keypair = KeyPair::from_private_key_bytes(bytes)?;
        let pubkey_hash = keypair.pubkey_hash();
        self.keys.insert(pubkey_hash, keypair);
        Ok(self.keys.get(&pubkey_hash).unwrap())
    }

    /// Get a keypair by address
    pub fn get_key_for_address(&self, address: &str) -> Option<&KeyPair> {
        let pubkey_hash = crate::wallet::address_to_pubkey_hash(address).ok()?;
        self.keys.get(&pubkey_hash)
    }

    /// Get all addresses
    pub fn get_addresses(&self) -> Vec<&str> {
        self.keys.values().map(|kp| kp.address.as_str()).collect()
    }

    /// Get all pubkey hashes
    pub fn get_pubkey_hashes(&self) -> Vec<Hash> {
        self.keys.keys().copied().collect()
    }

    /// Get total balance across all keys
    pub fn get_balance(&self, utxo_set: &UTXOSet) -> u64 {
        self.keys.keys()
            .map(|pkh| utxo_set.get_balance(pkh))
            .sum()
    }

    /// Get balance for a specific pubkey hash
    pub fn get_balance_for(&self, pubkey_hash: &Hash, utxo_set: &UTXOSet) -> u64 {
        utxo_set.get_balance(pubkey_hash)
    }

    /// Get all UTXOs owned by this wallet
    pub fn get_utxos(&self, utxo_set: &UTXOSet) -> Vec<(UTXOKey, UTXO, Hash)> {
        let mut result = Vec::new();
        
        for pubkey_hash in self.keys.keys() {
            for (key, utxo) in utxo_set.get_by_pubkey_hash(pubkey_hash) {
                result.push((key, utxo.clone(), *pubkey_hash));
            }
        }
        
        result
    }

    /// Create and sign a transaction
    pub fn create_transaction(
        &self,
        utxo_set: &UTXOSet,
        recipient_pubkey_hash: Hash,
        amount: u64,
        fee: u64,
    ) -> Result<Transaction, WalletError> {
        let total_needed = amount + fee;

        // Collect UTXOs until we have enough
        let mut selected_utxos: Vec<(UTXOKey, UTXO, &KeyPair)> = Vec::new();
        let mut total_input: u64 = 0;

        for pubkey_hash in self.keys.keys() {
            if total_input >= total_needed {
                break;
            }

            let keypair = self.keys.get(pubkey_hash).unwrap();

            for (key, utxo) in utxo_set.get_by_pubkey_hash(pubkey_hash) {
                if total_input >= total_needed {
                    break;
                }
                selected_utxos.push((key, utxo.clone(), keypair));
                total_input += utxo.amount;
            }
        }

        if total_input < total_needed {
            return Err(WalletError::InsufficientFunds {
                have: total_input,
                need: total_needed,
            });
        }

        // Create outputs
        let mut outputs = vec![TxOutput {
            amount,
            pubkey_hash: recipient_pubkey_hash,
        }];

        // Add change output if needed
        let change = total_input - total_needed;
        if change > 0 {
            // Send change to first key
            let change_pubkey_hash = *self.keys.keys().next()
                .ok_or(WalletError::NoUTXOs)?;
            
            outputs.push(TxOutput {
                amount: change,
                pubkey_hash: change_pubkey_hash,
            });
        }

        // Create unsigned inputs
        let inputs: Vec<TxInput> = selected_utxos.iter()
            .map(|((tx_hash, index), _, keypair)| TxInput {
                prev_tx_hash: *tx_hash,
                output_index: *index,
                signature: crate::crypto::SchnorrSignature([0u8; 64]), // Placeholder
                public_key: keypair.public_key.clone(),
            })
            .collect();

        // Create unsigned transaction
        let mut tx = Transaction::new(inputs, outputs);

        // Sign each input
        let signing_hash = tx.signing_hash();
        
        for (i, (_, _, keypair)) in selected_utxos.iter().enumerate() {
            let signature = keypair.sign(&signing_hash)?;
            tx.inputs[i].signature = signature;
        }

        Ok(tx)
    }

    /// Save wallet to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let bytes = bincode::serialize(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let mut file = File::create(path)?;
        file.write_all(&bytes)
    }

    /// Load wallet from file
    pub fn load<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let wallet = bincode::deserialize(&bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(wallet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let kp = KeyPair::generate();
        assert!(kp.address.starts_with("RH"));
    }

    #[test]
    fn test_keypair_export_import() {
        let kp1 = KeyPair::generate();
        let bytes = kp1.private_key_bytes();
        let kp2 = KeyPair::from_private_key_bytes(&bytes).unwrap();

        assert_eq!(kp1.public_key.0, kp2.public_key.0);
        assert_eq!(kp1.address, kp2.address);
    }

    #[test]
    fn test_wallet_generate_key() {
        let mut wallet = Wallet::new();
        let kp = wallet.generate_key();
        
        assert!(kp.address.starts_with("RH"));
        assert_eq!(wallet.get_addresses().len(), 1);
    }

    #[test]
    fn test_wallet_balance() {
        let mut wallet = Wallet::new();
        let kp = wallet.generate_key();
        let pubkey_hash = kp.pubkey_hash();

        let mut utxo_set = UTXOSet::new();
        utxo_set.add(
            hash_bytes(b"tx1"),
            0,
            UTXO {
                amount: 1000,
                pubkey_hash,
                height: 1,
            },
        );

        assert_eq!(wallet.get_balance(&utxo_set), 1000);
    }

    #[test]
    fn test_insufficient_funds() {
        let mut wallet = Wallet::new();
        wallet.generate_key();

        let utxo_set = UTXOSet::new(); // Empty

        let result = wallet.create_transaction(
            &utxo_set,
            Hash::zero(),
            1000,
            10,
        );

        assert!(matches!(result, Err(WalletError::InsufficientFunds { .. })));
    }
}
