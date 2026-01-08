//! P2P protocol messages
//! 
//! Defines the message types for network communication.

use serde::{Deserialize, Serialize};
use crate::consensus::Block;
use crate::crypto::Hash;
use crate::validation::Transaction;
use std::net::SocketAddr;

/// Protocol version
pub const PROTOCOL_VERSION: u32 = 1;

/// Network magic bytes (identifies RH network)
pub const NETWORK_MAGIC: [u8; 4] = [0x52, 0x48, 0x43, 0x4E]; // "RHCN"

/// Maximum message size (4 MB)
pub const MAX_MESSAGE_SIZE: usize = 4 * 1024 * 1024;

/// P2P message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Version handshake
    Version(VersionMessage),
    /// Version acknowledgement
    VerAck,
    /// Ping (liveness check)
    Ping(u64),
    /// Pong (liveness response)
    Pong(u64),
    /// Request peer addresses
    GetAddr,
    /// Share peer addresses
    Addr(Vec<SocketAddr>),
    /// Announce new block
    Inv(Vec<InvItem>),
    /// Request data
    GetData(Vec<InvItem>),
    /// Block data
    Block(Block),
    /// Transaction data
    Tx(Transaction),
    /// Request block headers
    GetHeaders(GetHeadersMessage),
    /// Block headers response
    Headers(Vec<crate::consensus::BlockHeader>),
    /// Request blocks
    GetBlocks(GetBlocksMessage),
    /// Reject message
    Reject(RejectMessage),
}

/// Inventory item type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum InvType {
    Transaction,
    Block,
}

/// Inventory item (reference to tx or block)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvItem {
    pub inv_type: InvType,
    pub hash: Hash,
}

/// Version handshake message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionMessage {
    /// Protocol version
    pub version: u32,
    /// Best block height
    pub best_height: u64,
    /// Sender's address
    pub from_addr: SocketAddr,
    /// Receiver's address
    pub to_addr: SocketAddr,
    /// Random nonce to detect self-connections
    pub nonce: u64,
    /// User agent string
    pub user_agent: String,
}

/// Get headers request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetHeadersMessage {
    /// Block locator hashes (newest to oldest)
    pub block_locators: Vec<Hash>,
    /// Stop hash (zero for no limit)
    pub stop_hash: Hash,
}

/// Get blocks request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlocksMessage {
    /// Block locator hashes (newest to oldest)
    pub block_locators: Vec<Hash>,
    /// Stop hash (zero for no limit)
    pub stop_hash: Hash,
}

/// Rejection message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectMessage {
    /// Type of message rejected
    pub message_type: String,
    /// Rejection code
    pub code: RejectCode,
    /// Reason for rejection
    pub reason: String,
    /// Hash of rejected data (if applicable)
    pub data_hash: Option<Hash>,
}

/// Rejection codes
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RejectCode {
    Malformed = 0x01,
    Invalid = 0x10,
    Obsolete = 0x11,
    Duplicate = 0x12,
    NonStandard = 0x40,
    Dust = 0x41,
    InsufficientFee = 0x42,
    Checkpoint = 0x43,
}

impl Message {
    /// Serialize message to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let payload = bincode::serialize(self).unwrap_or_default();
        
        let mut bytes = Vec::with_capacity(4 + 4 + payload.len());
        bytes.extend_from_slice(&NETWORK_MAGIC);
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&payload);
        
        bytes
    }

    /// Deserialize message from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 8 {
            return Err("Message too short".to_string());
        }

        // Check magic
        if bytes[0..4] != NETWORK_MAGIC {
            return Err("Invalid network magic".to_string());
        }

        let length = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
        
        if length > MAX_MESSAGE_SIZE {
            return Err("Message too large".to_string());
        }

        if bytes.len() < 8 + length {
            return Err("Incomplete message".to_string());
        }

        bincode::deserialize(&bytes[8..8 + length])
            .map_err(|e| format!("Deserialization error: {}", e))
    }

    /// Get the command name for this message
    pub fn command(&self) -> &'static str {
        match self {
            Message::Version(_) => "version",
            Message::VerAck => "verack",
            Message::Ping(_) => "ping",
            Message::Pong(_) => "pong",
            Message::GetAddr => "getaddr",
            Message::Addr(_) => "addr",
            Message::Inv(_) => "inv",
            Message::GetData(_) => "getdata",
            Message::Block(_) => "block",
            Message::Tx(_) => "tx",
            Message::GetHeaders(_) => "getheaders",
            Message::Headers(_) => "headers",
            Message::GetBlocks(_) => "getblocks",
            Message::Reject(_) => "reject",
        }
    }
}

/// Build block locator hashes for sync
pub fn build_block_locator(heights: &[u64], get_hash: impl Fn(u64) -> Option<Hash>) -> Vec<Hash> {
    let mut locator = Vec::new();
    let mut step = 1u64;
    let mut height = *heights.last().unwrap_or(&0);

    // Add hashes with exponentially increasing steps
    while height > 0 {
        if let Some(hash) = get_hash(height) {
            locator.push(hash);
        }
        
        if height < step {
            break;
        }
        
        height -= step;
        
        // Increase step after first 10 entries
        if locator.len() > 10 {
            step *= 2;
        }
    }

    // Always include genesis
    if let Some(hash) = get_hash(0) {
        if locator.last() != Some(&hash) {
            locator.push(hash);
        }
    }

    locator
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let msg = Message::Ping(12345);
        let bytes = msg.to_bytes();
        let recovered = Message::from_bytes(&bytes).unwrap();
        
        match recovered {
            Message::Ping(n) => assert_eq!(n, 12345),
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_network_magic() {
        let msg = Message::VerAck;
        let bytes = msg.to_bytes();
        
        assert_eq!(&bytes[0..4], &NETWORK_MAGIC);
    }

    #[test]
    fn test_invalid_magic_rejected() {
        let mut bytes = Message::VerAck.to_bytes();
        bytes[0] = 0xFF; // Corrupt magic
        
        let result = Message::from_bytes(&bytes);
        assert!(result.is_err());
    }
}
