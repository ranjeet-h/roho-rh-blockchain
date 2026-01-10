//! Wallet module - Key management and transaction signing

mod wallet;

pub use wallet::*;

use crate::crypto::Hash;

/// Decode a ROHO address back to its pubkey hash
/// Address format: "RH" + Base58Check(pubkey_hash[0:20] + checksum[0:4])
pub fn address_to_pubkey_hash(address: &str) -> Result<Hash, String> {
    if !address.starts_with("RH") {
        return Err("Invalid address prefix".to_string());
    }

    let encoded = &address[2..]; // Strip "RH" prefix
    let decoded = bs58::decode(encoded)
        .into_vec()
        .map_err(|_| "Invalid base58 encoding")?;

    if decoded.len() != 24 {
        return Err("Invalid address length".to_string());
    }

    let addr_bytes = &decoded[0..20];
    let checksum = &decoded[20..24];

    // Verify checksum
    let expected_checksum = crate::crypto::double_hash(addr_bytes);
    if checksum != &expected_checksum.0[0..4] {
        return Err("Invalid checksum".to_string());
    }

    // Create a 32-byte hash with the 20-byte address padded
    let mut hash_bytes = [0u8; 32];
    hash_bytes[0..20].copy_from_slice(addr_bytes);
    
    Ok(Hash(hash_bytes))
}
