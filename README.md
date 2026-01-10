# ROHO (RH) Blockchain

[![Build Status](https://img.shields.io/badge/status-stable-green)]() [![License](https://img.shields.io/badge/license-MIT-blue)]() [![Language](https://img.shields.io/badge/language-Rust-orange)](https://www.rust-lang.org/)

**ROHO (RH)** is a decentralized Proof-of-Work blockchain designed for global distribution. It prioritizes **immutability, transparency, and decentralization** over upgrade flexibility.

---

## 🎯 Core Philosophy

- **No Governance**: The protocol cannot be upgraded or modified
- **No Central Authority**: Anyone can run a node; no special privileges exist
- **Immutable Constitution**: Rules are fixed at genesis and enforced by cryptography
- **Pure PoW**: Consensus through Proof-of-Work (BLAKE3), not delegation or voting
- **Transparent**: All transactions are publicly verifiable

---

## ✨ Key Features

| Feature | Details |
|---------|---------|
| **Consensus** | Proof-of-Work (BLAKE3) with difficulty adjustment every 2016 blocks |
| **Block Time** | ~10 minutes (600 seconds) |
| **Total Supply** | 100,000,000 RH (fixed, no inflation) |
| **Cryptography** | Schnorr signatures (secp256k1), UTXO ledger model |
| **Network** | P2P peer discovery, automatic block synchronization |
| **Scalability** | Designed for millions of independent nodes globally |
| **Data Storage** | Persistent block storage with automatic recovery |

---

## 📋 Constitutional Rules

The ROHO blockchain operates under an immutable constitution:

### Monetary Policy
- **Total Supply**: 100,000,000 RH (absolute maximum)
- **Founder Allocation**: 10,000,000 RH in genesis block only
- **Public Issuance**: 90,000,000 RH through mining via asymptotic decay function
- **Halving Schedule**: Issuance naturally halts by year 2200

### Consensus Rules
- **Hash Function**: BLAKE3 (cannot be changed)
- **Block Time**: 600 seconds target
- **Difficulty Adjustment**: Every 2016 blocks, max 4x change per period
- **No Emergency Rules**: Adjustments are purely mathematical

### Cryptography
- **Signatures**: Schnorr on secp256k1
- **Addresses**: Base58Check format (prefix "RH")
- **Ledger Model**: UTXO (Unspent Transaction Output)

### Governance
- **No On-Chain Voting**: Community consensus only through social coordination
- **No Planned Upgrades**: Protocol is frozen at genesis
- **No Foundation or Treasury**: No special privileges for any entity
- **Immutability**: Bugs are permanent history, not rollbacks

See [RH_CONSTITUTION.txt](RH_CONSTITUTION.txt) for the complete constitution.

---

## 🚀 Getting Started

### Prerequisites
- Rust 1.70+ ([install](https://rustup.rs/))
- 4GB+ RAM recommended
- 10GB+ disk space (for full blockchain)

### Build
```bash
git clone https://github.com/ranjeet-h/roho-rh-blockchain
cd rh-core
cargo build --release
```

### Run a Single Node
```bash
./target/release/rh-node \
  --p2p-port 8333 \
  --rpc-port 8334
```

This will:
- ✅ Load or create blockchain state
- ✅ Listen for peer connections (port 8333)
- ✅ Serve RPC API (port 8334)
- ✅ Auto-connect to seed nodes
- ✅ Start mining blocks

### Check Node Status
```bash
curl http://localhost:8334/api/status
```

Example response:
```json
{
  "height": 42,
  "tip_hash": "000000a2bbc8...",
  "peers_connected": 3,
  "issued": "210000 RH"
}
```

---

## 🌐 Network Deployment

### Local Testing (3 nodes, same machine)
```bash
# Terminal 1: Seed node
./target/release/rh-node --db-path node1_data \
  --p2p-port 8333 --rpc-port 8334

# Terminal 2: Peer 1
./target/release/rh-node --db-path node2_data \
  --p2p-port 8335 --rpc-port 8335 \
  --connect 127.0.0.1:8333

# Terminal 3: Peer 2
./target/release/rh-node --db-path node3_data \
  --p2p-port 8336 --rpc-port 8336 \
  --connect 127.0.0.1:8333
```

All nodes will sync to the same blockchain height automatically.

### Internet Deployment (Multiple servers)
```bash
# Server 1 (Seed node)
./target/release/rh-node --p2p-port 8333 --rpc-port 8334

# Server 2 (Different machine)
./target/release/rh-node --p2p-port 8333 --rpc-port 8334 \
  --connect <server1-ip>:8333

# Server 3 (Different machine)
./target/release/rh-node --p2p-port 8333 --rpc-port 8334 \
  --connect <server1-ip>:8333
```

Nodes will automatically discover and sync with peers.

---

## 🔒 Security Model

### Block Immutability
```
Block N contains Hash(Block N-1)
                    ↓
If someone modifies Block N:
  - Hash changes
  - Invalid Block N+1 (references wrong hash)
  - Invalid Block N+2, N+3, ... (cascading)
  - Invalid blockchain
                    ↓
Attacker must:
  1. Recalculate PoW for Block N (hard)
  2. Recalculate PoW for ALL subsequent blocks (very hard)
  3. Outpace honest network mining rate (nearly impossible)
```

### Transaction Security
- Every transaction is signed with sender's private key
- No one can forge transactions without the private key
- Network nodes reject invalid signatures

### Consensus Security
- All nodes independently validate every block
- Invalid blocks are rejected and not propagated
- Peer misbehavior is tracked (banscore system)
- Bad peers are disconnected after 100 banscore

### Network Security
- Difficulty adjustment prevents attack advantage
- 600-second block time ensures global propagation
- P2P broadcast reaches all nodes in ~10 seconds
- Blocks cannot be modified after acceptance

---

## 📊 Architecture

### Node Components
```
┌─────────────────────────────────────────────────┐
│            RH Blockchain Node                  │
├──────────────────┬──────────────────────────────┤
│ P2P Network      │ RPC Server                  │
│ (Port 8333)      │ (Port 8334)                 │
│ • Peer discovery │ • REST API                  │
│ • Block sync     │ • Block explorer            │
│ • Tx broadcast   │ • Wallet interface          │
└────────┬─────────┴────────┬─────────────────────┘
         │                  │
         └────────┬─────────┘
                  ▼
        ┌─────────────────────┐
        │  Chain State        │
        │ • Height            │
        │ • UTXO set          │
        │ • Mempool           │
        │ • Validates blocks  │
        └──────────┬──────────┘
                   ▼
        ┌─────────────────────┐
        │  Database (Sled)    │
        │ • Blocks            │
        │ • UTXOs             │
        │ • Metadata          │
        └──────────┬──────────┘
                   ▼
        ┌─────────────────────┐
        │  File System        │
        │  ./rh_data/         │
        └─────────────────────┘
```

### Mining Process
1. **Assemble**: Collect transactions from mempool
2. **Build**: Create block template with coinbase transaction
3. **Mine**: Compute PoW using parallel CPU workers
4. **Validate**: Check all signatures and rules
5. **Apply**: Update UTXO set, increment height
6. **Persist**: Save block and state to disk
7. **Broadcast**: Send Inv message to all 12 peer connections

### Block Propagation at Scale
```
Your Node (mines block)
        ↓
Broadcast to 12 peers
        ↓
Each peer broadcasts to 12 peers (144 nodes in 2 hops)
        ↓
Those broadcast to 12 peers (1,728 nodes in 3 hops)
        ↓
In ~10 seconds: EVERY NODE ON EARTH has the block
```

---

## 💾 Data Persistence

Blockchain state is automatically saved to disk:

```
./rh_data/
├── blocks/          # All blocks with transaction data
├── utxos/           # Unspent outputs (what can be spent)
└── metadata/        # Height, tip hash, issued coins
```

When a node restarts:
- ✅ Loads saved state
- ✅ Continues from the same height
- ✅ Syncs any missed blocks from peers
- ✅ No re-mining, no data loss

---

## 🛠️ Command-Line Options

```bash
./target/release/rh-node [OPTIONS]

Options:
  --db-path <PATH>           Database directory (default: ./rh_data)
  --p2p-port <PORT>          P2P listen port (default: 8333)
  --rpc-port <PORT>          RPC server port (default: 8334)
  --connect <PEER>           Connect to specific peer (e.g., 192.168.1.1:8333)
  --miner-address <ADDRESS>  Miner reward address (default: auto-generated)
  --help                     Show help message
```

---

## 📡 API Reference

### Status Endpoint
```bash
GET /api/status
```

Response:
```json
{
  "height": 42,
  "tip_hash": "000000a2bbc8...",
  "peers_connected": 3,
  "issued": "210000 RH",
  "utxo_count": 45
}
```

### Block by Height
```bash
GET /api/block/<height>
```

### Transaction
```bash
GET /api/transaction/<tx_hash>
```

### Wallet Balance
```bash
GET /api/address/<address>
```

---

## 🧪 Testing

Run the full test suite:
```bash
cargo test --release
```

Run with output:
```bash
cargo test --release -- --nocapture
```

Test specific module:
```bash
cargo test --release consensus::
```

---

## 📚 Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md) - Deep dive into codebase structure and design
- [QUICK_START.md](QUICK_START.md) - Practical setup guide for local testing
- [RH_CONSTITUTION.txt](RH_CONSTITUTION.txt) - Complete immutable rules

---

## 🔐 Cryptographic Primitives

| Component | Algorithm | Library | Bytes |
|-----------|-----------|---------|-------|
| Hashing | BLAKE3 | blake3 | 32 |
| Signatures | Schnorr/secp256k1 | k256 | 64 |
| Private Keys | secp256k1 | k256 | 32 |
| Public Keys | secp256k1 compressed | k256 | 33 |
| Addresses | Base58Check | bs58 | ~26 |

---

## 🌟 Why ROHO is Different

| Aspect | Traditional Blockchain | ROHO |
|--------|----------------------|------|
| **Governance** | Votes, proposals, upgrades | None - frozen at genesis |
| **Authority** | Foundation, core devs | No one - math enforces rules |
| **Supply** | Subject to change | Absolute: 100M RH maximum |
| **Consensus** | PoW or PoS with possible changes | Pure PoW, unchangeable |
| **Bugs** | Often reversed via fork | History remains (immutable) |
| **Philosophy** | "Live and upgrade" | "Fix and forget" |

---

## 📊 Scalability

ROHO is designed to support:
- **Millions of nodes** running independently
- **Global network** spanning continents
- **10-minute blocks** with consistent propagation
- **No centralized infrastructure** (fully peer-to-peer)
- **Billions of transactions** across history

Real-world propagation time: <10 seconds globally.

---

## 🤝 Contributing

ROHO is open source. To contribute:

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Commit changes: `git commit -am 'Add feature'`
4. Push branch: `git push origin feature/your-feature`
5. Submit a pull request

**Note**: The protocol itself cannot be changed (immutable constitution), but optimizations, performance improvements, and bug fixes are welcome.

---

## ⚠️ Disclaimer

ROHO is experimental software. While the cryptography is sound and the design is carefully considered:

- This is not financial advice
- Use at your own risk
- Understand the code before trusting it with real value
- The immutable constitution means bugs cannot be fixed—only documented
- Always keep private keys secure

---

## 📄 License

ROHO is released under the MIT License. See LICENSE file for details.

---

## 🙏 Acknowledgments

Built with:
- [Blake3](https://github.com/BLAKE3-team/BLAKE3) - Cryptographic hash
- [k256](https://github.com/RustCrypto/elliptic-curves) - secp256k1 signatures
- [Sled](https://github.com/spacejam/sled) - Embedded database
- [Tokio](https://tokio.rs/) - Async runtime
- [Axum](https://github.com/tokio-rs/axum) - Web framework

---

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/ranjeet-h/roho-rh-blockchain/issues)
- **Documentation**: See links above
- **Community**: Discussions welcome via issues

---

## 🎯 Roadmap

- ✅ Genesis block and core protocol
- ✅ P2P networking and peer discovery
- ✅ Mining and difficulty adjustment
- ✅ Multi-node synchronization
- ✅ Data persistence and recovery
- 🔄 Continued network testing and hardening
- 🔄 Community monitoring and statistics

---

**ROHO: Immutable by Design. Decentralized by Architecture. Trustless by Math.**

```
╔══════════════════════════════════════════════════╗
║       ROHO (RH) BLOCKCHAIN NETWORK               ║
║  Immutable · Decentralized · Trustless           ║
╚══════════════════════════════════════════════════╝
```
