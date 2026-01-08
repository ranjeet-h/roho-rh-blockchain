//! ROHO (RH) Blockchain Node
//! 
//! Main entry point for running a ROHO node.
//! RH is the short form used in addresses and logos.

use rh_core::node::{create_genesis_block, GenesisInfo};
use rh_core::storage::ChainState;
use rh_core::mining::{Miner, MiningResult};
use rh_core::wallet::Wallet;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║              ROHO (RH) BLOCKCHAIN NODE                   ║");
    println!("║          Immutable · Decentralized · Trustless           ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();

    // Display genesis info
    let genesis_info = GenesisInfo::new();
    println!("Genesis Block Information:");
    println!("  Hash:        {}", genesis_info.hash);
    println!("  Merkle Root: {}", genesis_info.merkle_root);
    println!("  Timestamp:   {}", genesis_info.timestamp);
    println!("  Difficulty:  0x{:08x}", genesis_info.difficulty);
    println!("  Founder:     {} RH", genesis_info.founder_allocation / 100_000_000);
    println!();

    // Initialize chain state from genesis
    let genesis = create_genesis_block();
    let chain_state = Arc::new(Mutex::new(ChainState::new(&genesis)));
    
    // Display initial state
    {
        let state = chain_state.lock().unwrap();
        let stats = state.get_stats();
        println!("Chain State:");
        println!("  Height:      {}", stats.height);
        println!("  Tip Hash:    {}", stats.tip_hash);
        println!("  UTXO Count:  {}", stats.utxo_count);
        println!("  Issued:      {} RH", stats.total_issued / 100_000_000);
        println!();
    }

    // Create a wallet (just for demo purposes in this single node)
    let mut wallet = Wallet::new();
    let keypair = wallet.generate_key();
    let address = keypair.address.clone();
    let pubkey_hash = keypair.pubkey_hash();
    
    // Create miner
    let miner = Miner::new(pubkey_hash);
    
    println!("Miner initialized for address: {}", address);
    println!("Node started on port 8333");
    println!("Mining in background (CPU)...");
    println!("Press Ctrl+C to stop.");
    println!();

    // Spawn mining task
    let miner_state = chain_state.clone();
    let miner_instance = miner.clone();
    
    tokio::spawn(async move {
        loop {
            // Check if we should mine
            // In a real implementation, we'd check if we are synced and have peers
            // For shadow network demo, we mine continuously
            
            // Construct block template (requires lock)
            let block_template = {
                let state = miner_state.lock().unwrap();
                // Assemble block with empty transactions for now
                miner_instance.assemble_block(&state, vec![])
            };
            
            // Mine block (CPU intensive, find valid nonce)
            let miner_task = miner_instance.clone();
            
            // Run synchronous mining in blocking thread to avoid blocking async runtime
            let result = tokio::task::spawn_blocking(move || {
                 miner_task.mine_block(block_template)
            }).await.unwrap();

            match result {
                MiningResult::Success(block) => {
                    let mut state = miner_state.lock().unwrap();
                    let hash = block.hash();
                    let height = state.height + 1;
                    
                    // Apply block
                    state.apply_block(&block);
                    
                    println!("⛏️  Mined block {} ({})", height, hash);
                    
                    // Small delay to simulate network latency / block time management
                    // In real PoW, we just mine instantly again, but here lets be gentle to logs
                }
                _ => {
                    // Interrupted or failed or no work
                    sleep(Duration::from_millis(100)).await;
                }
            }
        }
    });

    // P2P Listener Loop
    let listener = TcpListener::bind("0.0.0.0:8333").await?;

    tokio::select! {
        _ = async {
            loop {
                // Just accept connections and log them
                match listener.accept().await {
                    Ok((_socket, addr)) => {
                        println!("Peer connected: {}", addr);
                        // Connection dropped here
                    }
                    Err(e) => {
                        eprintln!("Connection error: {}", e);
                    }
                }
            }
        } => {},
        _ = tokio::signal::ctrl_c() => {
            println!("\nShutdown signal received. Stopping node...");
            miner.stop();
        }
    }

    Ok(())
}
