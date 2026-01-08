//! Peer management
//! 
//! Handles peer discovery and connection management.
//! No trusted bootstrap nodes - pure gossip-based discovery.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// Peer connection state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerState {
    /// Not yet connected
    Disconnected,
    /// Connection in progress
    Connecting,
    /// Fully connected and handshake complete
    Connected,
    /// Banned due to misbehavior
    Banned,
}

/// Information about a peer
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Peer's network address
    pub addr: SocketAddr,
    /// Current connection state
    pub state: PeerState,
    /// Last seen timestamp
    pub last_seen: Instant,
    /// Number of failed connection attempts
    pub failed_attempts: u32,
    /// Best known block height
    pub best_height: u64,
    /// Protocol version
    pub version: u32,
    /// Misbehavior score (100 = ban)
    pub misbehavior_score: u32,
}

impl PeerInfo {
    /// Create new peer info
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            state: PeerState::Disconnected,
            last_seen: Instant::now(),
            failed_attempts: 0,
            best_height: 0,
            version: 0,
            misbehavior_score: 0,
        }
    }

    /// Update last seen time
    pub fn touch(&mut self) {
        self.last_seen = Instant::now();
    }

    /// Add misbehavior points
    pub fn add_misbehavior(&mut self, points: u32) {
        self.misbehavior_score = self.misbehavior_score.saturating_add(points);
        if self.misbehavior_score >= 100 {
            self.state = PeerState::Banned;
        }
    }

    /// Check if peer should be banned
    pub fn should_ban(&self) -> bool {
        self.misbehavior_score >= 100
    }

    /// Check if connection has timed out
    pub fn is_stale(&self, timeout: Duration) -> bool {
        self.last_seen.elapsed() > timeout
    }
}

/// Peer manager
#[derive(Debug, Default)]
pub struct PeerManager {
    /// Known peers
    peers: HashMap<SocketAddr, PeerInfo>,
    /// Connected peer addresses
    connected: HashSet<SocketAddr>,
    /// Maximum number of connections
    max_connections: usize,
}

impl PeerManager {
    /// Create a new peer manager
    pub fn new(max_connections: usize) -> Self {
        Self {
            peers: HashMap::new(),
            connected: HashSet::new(),
            max_connections,
        }
    }

    /// Add a new peer address
    pub fn add_peer(&mut self, addr: SocketAddr) {
        if !self.peers.contains_key(&addr) {
            self.peers.insert(addr, PeerInfo::new(addr));
        }
    }

    /// Add multiple peer addresses
    pub fn add_peers(&mut self, addrs: &[SocketAddr]) {
        for addr in addrs {
            self.add_peer(*addr);
        }
    }

    /// Mark peer as connected
    pub fn peer_connected(&mut self, addr: SocketAddr, version: u32, best_height: u64) {
        if let Some(peer) = self.peers.get_mut(&addr) {
            peer.state = PeerState::Connected;
            peer.version = version;
            peer.best_height = best_height;
            peer.failed_attempts = 0;
            peer.touch();
            self.connected.insert(addr);
        }
    }

    /// Mark peer as disconnected
    pub fn peer_disconnected(&mut self, addr: &SocketAddr) {
        if let Some(peer) = self.peers.get_mut(addr) {
            peer.state = PeerState::Disconnected;
        }
        self.connected.remove(addr);
    }

    /// Mark connection attempt failed
    pub fn connection_failed(&mut self, addr: &SocketAddr) {
        if let Some(peer) = self.peers.get_mut(addr) {
            peer.state = PeerState::Disconnected;
            peer.failed_attempts += 1;
        }
    }

    /// Ban a peer
    pub fn ban_peer(&mut self, addr: &SocketAddr) {
        if let Some(peer) = self.peers.get_mut(addr) {
            peer.state = PeerState::Banned;
            peer.misbehavior_score = 100;
        }
        self.connected.remove(addr);
    }

    /// Report misbehavior
    pub fn report_misbehavior(&mut self, addr: &SocketAddr, points: u32) {
        if let Some(peer) = self.peers.get_mut(addr) {
            peer.add_misbehavior(points);
            if peer.should_ban() {
                self.connected.remove(addr);
            }
        }
    }

    /// Get peers to connect to
    pub fn get_peers_to_connect(&self, count: usize) -> Vec<SocketAddr> {
        if self.connected.len() >= self.max_connections {
            return vec![];
        }

        let remaining = self.max_connections - self.connected.len();
        let count = count.min(remaining);

        self.peers.values()
            .filter(|p| {
                p.state == PeerState::Disconnected 
                && p.failed_attempts < 5 
                && !p.should_ban()
            })
            .take(count)
            .map(|p| p.addr)
            .collect()
    }

    /// Get all connected peers
    pub fn get_connected_peers(&self) -> Vec<&PeerInfo> {
        self.peers.values()
            .filter(|p| p.state == PeerState::Connected)
            .collect()
    }

    /// Get number of connected peers
    pub fn connected_count(&self) -> usize {
        self.connected.len()
    }

    /// Get total number of known peers
    pub fn known_count(&self) -> usize {
        self.peers.len()
    }

    /// Remove stale peers
    pub fn remove_stale_peers(&mut self, timeout: Duration) {
        let stale: Vec<SocketAddr> = self.peers.iter()
            .filter(|(_, p)| p.state == PeerState::Disconnected && p.is_stale(timeout))
            .map(|(addr, _)| *addr)
            .collect();

        for addr in stale {
            self.peers.remove(&addr);
        }
    }

    /// Get peers with higher chains
    pub fn get_peers_with_height(&self, min_height: u64) -> Vec<&PeerInfo> {
        self.peers.values()
            .filter(|p| p.state == PeerState::Connected && p.best_height > min_height)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_addr(port: u16) -> SocketAddr {
        format!("127.0.0.1:{}", port).parse().unwrap()
    }

    #[test]
    fn test_add_peer() {
        let mut pm = PeerManager::new(10);
        let addr = make_addr(8000);
        
        pm.add_peer(addr);
        assert_eq!(pm.known_count(), 1);
    }

    #[test]
    fn test_peer_connection() {
        let mut pm = PeerManager::new(10);
        let addr = make_addr(8000);
        
        pm.add_peer(addr);
        pm.peer_connected(addr, 1, 100);
        
        assert_eq!(pm.connected_count(), 1);
        
        let peers = pm.get_connected_peers();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].best_height, 100);
    }

    #[test]
    fn test_ban_peer() {
        let mut pm = PeerManager::new(10);
        let addr = make_addr(8000);
        
        pm.add_peer(addr);
        pm.peer_connected(addr, 1, 100);
        pm.ban_peer(&addr);
        
        assert_eq!(pm.connected_count(), 0);
        assert_eq!(pm.peers.get(&addr).unwrap().state, PeerState::Banned);
    }

    #[test]
    fn test_misbehavior() {
        let mut pm = PeerManager::new(10);
        let addr = make_addr(8000);
        
        pm.add_peer(addr);
        pm.peer_connected(addr, 1, 100);
        
        pm.report_misbehavior(&addr, 50);
        assert_eq!(pm.connected_count(), 1);
        
        pm.report_misbehavior(&addr, 60);
        assert_eq!(pm.connected_count(), 0); // Auto-banned
    }
}
