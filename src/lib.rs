//! ROHO (RH) Blockchain Core Library
//! 
//! A deterministic, immutable cryptocurrency with PoW consensus,
//! UTXO ledger, and Schnorr signatures.
//! 
//! RH is the short form used in addresses, logos, and protocol identifiers.

pub mod consensus;
pub mod crypto;
pub mod validation;
pub mod storage;
pub mod p2p;
pub mod mining;
pub mod wallet;
pub mod node;

/// Protocol constants - HARD-CODED, NEVER CONFIGURABLE
pub mod constants {
    /// Total supply of RH coins (in base units, 8 decimal places)
    pub const TOTAL_SUPPLY: u64 = 100_000_000 * 100_000_000; // 100M RH
    
    /// Founder allocation (in base units)
    pub const FOUNDER_ALLOCATION: u64 = 10_000_000 * 100_000_000; // 10M RH
    
    /// Public issuance through mining (in base units)
    pub const PUBLIC_ISSUANCE: u64 = 90_000_000 * 100_000_000; // 90M RH
    
    /// Target block time in seconds
    pub const BLOCK_TIME_TARGET: u64 = 600;
    
    /// Difficulty adjustment interval (blocks)
    pub const DIFFICULTY_ADJUSTMENT_INTERVAL: u64 = 2016;
    
    /// Number of decimal places
    pub const DECIMAL_PLACES: u8 = 8;
    
    /// Chain name (short form for addresses/logos)
    pub const CHAIN_NAME: &str = "RH";
    
    /// Full chain name
    pub const CHAIN_FULL_NAME: &str = "ROHO";
    
    /// Genesis timestamp (Unix timestamp)
    pub const GENESIS_TIMESTAMP: u64 = 1736339922; // 2026-01-08
    
    /// Founder address (will be set during genesis ceremony)
    pub const FOUNDER_ADDRESS: &str = "RH2Q3hRrvJ1MZFFW7LYbUghLCKEUjCHZWXU";
    
    /// Constitution hash (SHA256 of RH_CONSTITUTION.txt)
    /// FROZEN: This value is immutable after genesis
    pub const CONSTITUTION_HASH: &str = "c38b2b1333db0280b786f5ea750911b7a5dbf12df6e3e6e7e468b9e7b39e62bf";
}
