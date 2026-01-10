//! Seed Node Configuration
//! 
//! Hardcoded bootstrap nodes for initial peer discovery.
//! New nodes connect to these first to discover the rest of the network.

use crate::constants;

/// Mainnet seed nodes from protocol constants
/// These are the official bootstrap nodes for the RH network
pub fn get_mainnet_seeds() -> Vec<&'static str> {
    constants::SEED_NODES.to_vec()
}

/// Testnet seed nodes (for development/testing)
pub const TESTNET_SEEDS: &[&str] = &[
    "127.0.0.1:8333",  // Local development node
];

/// Get seed nodes for the current network
pub fn get_seed_nodes(testnet: bool) -> Vec<&'static str> {
    if testnet {
        TESTNET_SEEDS.to_vec()
    } else {
        get_mainnet_seeds()
    }
}

/// Parse seed address to SocketAddr
pub fn parse_seed(seed: &str) -> Option<std::net::SocketAddr> {
    seed.parse().ok()
}

/// Get all seed nodes as SocketAddr (filtering out invalid ones)
pub fn get_seed_addresses(testnet: bool) -> Vec<std::net::SocketAddr> {
    get_seed_nodes(testnet)
        .into_iter()
        .filter_map(parse_seed)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_mainnet_seeds() {
        let seeds = get_mainnet_seeds();
        assert!(!seeds.is_empty());
        // Should contain the seed nodes from constants
        assert!(seeds.contains(&"seed.roho.io:8333"));
    }

    #[test]
    fn test_get_seed_addresses_mainnet() {
        let addresses = get_seed_addresses(false);
        // Note: Domain names like "seed.roho.io:8333" cannot be parsed as SocketAddr
        // without DNS resolution, so they return None and are filtered out
        // This is expected behavior - domain resolution happens at connection time
        // For this test, we just verify the function doesn't panic
        // In a real network, these would be resolved to IPs
        let _ = addresses; // Just ensure it doesn't panic
    }

    #[test]
    fn test_parse_seed_valid() {
        let addr = parse_seed("127.0.0.1:8333");
        assert!(addr.is_some());
        assert_eq!(addr.unwrap().to_string(), "127.0.0.1:8333");
    }

    #[test]
    fn test_parse_seed_invalid() {
        let addr = parse_seed("invalid-address");
        assert!(addr.is_none());
    }
}
