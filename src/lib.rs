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
pub mod rpc;
pub mod explorer;

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
    
    /// Seed nodes for network bootstrap
    /// New nodes will connect to these to discover peers
    /// Format: "host:port"
    pub const SEED_NODES: &[&str] = &[
        // Primary seed node (maintained by core team)
        "seed.roho.io:8333",
        
        // Backup seed nodes
        "seed2.roho.io:8333",
        "seed3.roho.io:8333",
        
        // Community nodes (to be updated as network grows)
        // Add more seed nodes here as they are proven stable
    ];
    
    /// Chain checkpoints - hard blocks that cannot be reorged past
    /// Format: (height, block_hash_hex)
    /// These checkpoints prevent deep chain reorganizations and speed up initial sync
    /// Should be updated every 100k blocks or at major milestones
    pub const CHECKPOINTS: &[(u64, &str)] = &[
        // Genesis block
        (0, "3153db7f3b03eb371f2227bdb8464626f41399de839dd739c77b6c71bc85d623"),

        // Add checkpoints here as the chain progresses
        // Example:
        // (100_000, "block_hash_at_height_100k"),
        // (200_000, "block_hash_at_height_200k"),
    ];
    
    /// Maximum depth of chain reorganization allowed (in blocks)
    /// Prevents reorgs deeper than this unless explicitly authorized
    pub const MAX_REORG_DEPTH: u64 = 10;
    
    /// Chain ID for replay protection
    /// Mainnet = 0x01, Testnet = 0x00
    pub const CHAIN_ID: u8 = 0x01;
}
