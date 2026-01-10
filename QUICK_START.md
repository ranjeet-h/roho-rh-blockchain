# RH Blockchain - Quick Start Guide

## What is RH?

RH is a decentralized blockchain designed to work with **millions of nodes across the globe**. Each node:
- Independently validates all blocks
- Stores a full copy of the blockchain
- Can mine new blocks
- Automatically syncs with peers

## Single Node (Local Testing)

```bash
# Start a node (auto-connects to seed nodes)
./target/release/rh-node --p2p-port 8333 --rpc-port 8334

# The node will:
# ✅ Load state from disk (or start fresh)
# ✅ Listen for peer connections on port 8333
# ✅ Serve RPC API on port 8334
# ✅ Auto-connect to seed nodes
# ✅ Discover and sync with peers
# ✅ Start mining
```

Check status:
```bash
curl http://localhost:8334/api/status
```

## Multiple Nodes (Different Machines on Internet)

### Setup 1: Local Testing (3 nodes on same machine)

```bash
# Terminal 1: Node 1 (seed-like, first to start)
./target/release/rh-node --db-path node1_data \
  --p2p-port 8333 --rpc-port 8334

# Terminal 2: Node 2 (connects to Node 1)
./target/release/rh-node --db-path node2_data \
  --p2p-port 8335 --rpc-port 8335 \
  --connect 127.0.0.1:8333

# Terminal 3: Node 3 (connects to Node 1)
./target/release/rh-node --db-path node3_data \
  --p2p-port 8336 --rpc-port 8336 \
  --connect 127.0.0.1:8333
```

What happens:
- All 3 nodes sync to the same blockchain height
- When Node 1 mines a block, Nodes 2 & 3 receive it in ~1 second
- If you kill Node 1, Nodes 2 & 3 continue mining
- If you kill Node 2, Nodes 1 & 3 continue (with no impact)
- When Node 2 restarts, it syncs blocks it missed from Node 1 or 3

### Setup 2: Internet Deployment (Different servers)

**Server 1 (192.168.1.10):**
```bash
./target/release/rh-node --p2p-port 8333 --rpc-port 8334
```

**Server 2 (192.168.1.20):**
```bash
./target/release/rh-node --p2p-port 8333 --rpc-port 8334 \
  --connect 192.168.1.10:8333
```

**Server 3 (192.168.1.30):**
```bash
./target/release/rh-node --p2p-port 8333 --rpc-port 8334 \
  --connect 192.168.1.10:8333
```

Or if you don't specify `--connect`, nodes auto-connect to hardcoded seed nodes.

## How It Works at Internet Scale

```
NETWORK OF MILLIONS:

When you mine a block on any node:

  Your Node (mines block)
        ↓
  Broadcast to 12 peers
        ↓
  Each peer broadcasts to 12 peers (144 nodes in 2 hops)
        ↓
  Those broadcast to 12 peers (1,728 nodes in 3 hops)
        ↓
  In ~10 seconds: EVERY NODE ON EARTH has the block
        ↓
  Each node independently:
    - Validates the block
    - Checks signatures
    - Applies to their chain
    - Saves to disk
  
  ✅ Network consensus reached
```

## Data Persistence

Your blockchain state is **automatically saved to disk**:

```
node1_data/
├── blocks       (all blocks with transaction data)
├── utxos        (unspent outputs - what people can spend)
└── metadata     (current height, tip hash, issued coins)
```

When you restart a node:
```
✅ Loads saved state
✅ Continues from where it left off
✅ Syncs any missed blocks from peers
✅ No re-mining, no data loss
```

## Key Differences from Centralized Systems

| Property | Centralized | RH Blockchain |
|----------|-------------|---------------|
| Who runs it? | One company | Millions of nodes |
| Data stored where? | Central database | Every node has full copy |
| Trust needed | Trust company | Trust math/code (no trust needed) |
| What if company goes down? | Everything stops | Network continues forever |
| What if your node goes down? | You lose access | Network continues, you sync on restart |
| Who validates transactions? | Company's servers | Every node validates |
| Who can censor you? | Company | No one (decentralized) |

## Next Steps

1. **Test locally**: Run 3 nodes on same machine with `--db-path` and `--connect`
2. **Verify sync**: Kill a node, restart it, watch it catch up
3. **Watch propagation**: Mine blocks on one node, see them appear on others
4. **Check persistence**: Restart nodes and verify they keep their blockchain
5. **Deploy globally**: Run nodes on different servers/cloud providers

## Files to Read

- `NETWORK_DEPLOYMENT.md` - Deep dive into P2P architecture and deployment
- `PRODUCTION_HARDENING.md` - Security features and status
- `NONCE_AND_TIMESTAMP_GUIDE.md` - Transaction ordering and block timing

## Commands

```bash
# Build
cargo build --release

# Run with custom database path (allows multiple nodes)
./target/release/rh-node --db-path ./my_node_data \
  --p2p-port 8333 --rpc-port 8334

# Connect to specific peer (instead of auto-connecting to seeds)
./target/release/rh-node --connect 192.168.1.100:8333 \
  --p2p-port 8333 --rpc-port 8334

# Custom miner address
./target/release/rh-node --miner-address RHCxotQQjf723MJvp4oDbd3QRAMaRGFj2na \
  --p2p-port 8333 --rpc-port 8334

# Stop gracefully
Ctrl+C (waits for pending operations, then saves state)
```

## Understanding the Output

```
╔══════════════════════════════════════════════════════════╗
║              ROHO (RH) BLOCKCHAIN NODE                   ║
║          Immutable · Decentralized · Trustless           ║
╚══════════════════════════════════════════════════════════╝

Genesis Block Information:           ← Confirms we're on same network
  Hash:        ff9f03b583f1a3db...
  Timestamp:   1736339922
  
📂 Restored chain to height 42      ← Node has 42 blocks saved on disk
Chain State:
  Height:      42                    ← Current blockchain height
  Tip Hash:    000000a2bbc8...      ← Hash of latest block
  UTXO Count:  45                    ← 45 unspent outputs exist
  Issued:      210000 RH             ← Total mined so far
  
🤝 Peer connected: 192.168.1.100    ← Connected to another node
👋 Peer version: roho-v1.4 (Height: 45)  ← Peer has 3 more blocks

⛏️  Block #43 | Tip: 00000042abc1... ← Mining new block #43
    | Miner: 220000 RH | Mined Supply: 220000 RH

📦 Applied block #45 from peer      ← Received blocks from peer, syncing
```

## The Real Magic

When you run millions of independent RH nodes:
- **No central authority** controls the blockchain
- **No single point of failure** (if 1 node goes down, millions continue)
- **No trust needed** (math proves validity, not people)
- **Transparent** (anyone can verify any block)
- **Immutable** (rewriting history requires 51% of computing power globally)

This is what "Decentralized" means.
