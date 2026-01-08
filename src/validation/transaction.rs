//! Transaction structure and validation
//! 
//! UTXO-based transactions with Schnorr signatures.

use serde::{Deserialize, Serialize};
use crate::crypto::{Hash, hash_bytes, PublicKey, SchnorrSignature};
use crate::storage::UTXOSet;

/// A transaction input referencing a previous output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxInput {
    /// Hash of the transaction containing the output
    pub prev_tx_hash: Hash,
    /// Index of the output in that transaction
    pub output_index: u32,
    /// Signature proving ownership
    pub signature: SchnorrSignature,
    /// Public key of the signer
    pub public_key: PublicKey,
}

/// A transaction output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxOutput {
    /// Amount in base units (satoshi-equivalent)
    pub amount: u64,
    /// Public key hash of the recipient
    pub pubkey_hash: Hash,
}

/// A complete transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction version
    pub version: u32,
    /// Transaction inputs
    pub inputs: Vec<TxInput>,
    /// Transaction outputs
    pub outputs: Vec<TxOutput>,
    /// Lock time (block height or timestamp)
    pub lock_time: u32,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(inputs: Vec<TxInput>, outputs: Vec<TxOutput>) -> Self {
        Self {
            version: 1,
            inputs,
            outputs,
            lock_time: 0,
        }
    }

    /// Create a coinbase transaction (mining reward)
    pub fn coinbase(reward: u64, miner_pubkey_hash: Hash) -> Self {
        Self {
            version: 1,
            inputs: vec![TxInput {
                prev_tx_hash: Hash::zero(),
                output_index: 0xFFFFFFFF,
                signature: SchnorrSignature([0u8; 64]),
                public_key: PublicKey([0u8; 32]),
            }],
            outputs: vec![TxOutput {
                amount: reward,
                pubkey_hash: miner_pubkey_hash,
            }],
            lock_time: 0,
        }
    }

    /// Check if this is a coinbase transaction
    pub fn is_coinbase(&self) -> bool {
        self.inputs.len() == 1 
            && self.inputs[0].prev_tx_hash == Hash::zero()
            && self.inputs[0].output_index == 0xFFFFFFFF
    }

    /// Calculate transaction hash
    pub fn hash(&self) -> Hash {
        let bytes = self.to_bytes_for_signing();
        hash_bytes(&bytes)
    }

    /// Get the signing hash (excludes signatures)
    pub fn signing_hash(&self) -> Hash {
        let bytes = self.to_bytes_for_signing();
        hash_bytes(&bytes)
    }

    /// Serialize for signing (without signatures)
    fn to_bytes_for_signing(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        
        // Version
        bytes.extend_from_slice(&self.version.to_le_bytes());
        
        // Input count
        bytes.extend_from_slice(&(self.inputs.len() as u32).to_le_bytes());
        
        // Inputs (without signatures for signing hash)
        for input in &self.inputs {
            bytes.extend_from_slice(&input.prev_tx_hash.0);
            bytes.extend_from_slice(&input.output_index.to_le_bytes());
        }
        
        // Output count
        bytes.extend_from_slice(&(self.outputs.len() as u32).to_le_bytes());
        
        // Outputs
        for output in &self.outputs {
            bytes.extend_from_slice(&output.amount.to_le_bytes());
            bytes.extend_from_slice(&output.pubkey_hash.0);
        }
        
        // Lock time
        bytes.extend_from_slice(&self.lock_time.to_le_bytes());
        
        bytes
    }

    /// Verify all input signatures
    pub fn verify_signatures(&self, utxo_set: &UTXOSet) -> bool {
        if self.is_coinbase() {
            return true;
        }

        let signing_hash = self.signing_hash();

        for input in &self.inputs {
            // Verify signature
            if !input.public_key.verify(&signing_hash, &input.signature) {
                return false;
            }

            // Verify public key matches the UTXO
            if let Some(utxo) = utxo_set.get(&input.prev_tx_hash, input.output_index) {
                let pubkey_hash = hash_bytes(&input.public_key.0);
                if pubkey_hash != utxo.pubkey_hash {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// Calculate total input value (requires UTXO lookup)
    pub fn total_input_value(&self, utxo_set: &UTXOSet) -> u64 {
        self.inputs.iter()
            .filter_map(|input| {
                utxo_set.get(&input.prev_tx_hash, input.output_index)
                    .map(|utxo| utxo.amount)
            })
            .sum()
    }

    /// Calculate total output value
    pub fn total_output_value(&self) -> u64 {
        self.outputs.iter().map(|o| o.amount).sum()
    }

    /// Calculate transaction fee
    pub fn fee(&self, utxo_set: &UTXOSet) -> u64 {
        let input_value = self.total_input_value(utxo_set);
        let output_value = self.total_output_value();
        input_value.saturating_sub(output_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coinbase_detection() {
        let coinbase = Transaction::coinbase(5000, Hash::zero());
        assert!(coinbase.is_coinbase());
        
        let regular = Transaction::new(vec![], vec![]);
        assert!(!regular.is_coinbase());
    }

    #[test]
    fn test_transaction_hash_deterministic() {
        let tx = Transaction::coinbase(5000, Hash::zero());
        let hash1 = tx.hash();
        let hash2 = tx.hash();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_output_value_calculation() {
        let tx = Transaction::new(
            vec![],
            vec![
                TxOutput { amount: 100, pubkey_hash: Hash::zero() },
                TxOutput { amount: 200, pubkey_hash: Hash::zero() },
            ],
        );
        assert_eq!(tx.total_output_value(), 300);
    }

    #[test]
    fn test_signing_hash_excludes_signatures() {
        let tx1 = Transaction {
            version: 1,
            inputs: vec![TxInput {
                prev_tx_hash: hash_bytes(b"prev"),
                output_index: 0,
                signature: SchnorrSignature([1u8; 64]),
                public_key: PublicKey([0u8; 32]),
            }],
            outputs: vec![TxOutput { amount: 100, pubkey_hash: Hash::zero() }],
            lock_time: 0,
        };

        let tx2 = Transaction {
            version: 1,
            inputs: vec![TxInput {
                prev_tx_hash: hash_bytes(b"prev"),
                output_index: 0,
                signature: SchnorrSignature([2u8; 64]), // Different signature
                public_key: PublicKey([0u8; 32]),
            }],
            outputs: vec![TxOutput { amount: 100, pubkey_hash: Hash::zero() }],
            lock_time: 0,
        };

        // Signing hash should be the same
        assert_eq!(tx1.signing_hash(), tx2.signing_hash());
    }
}
