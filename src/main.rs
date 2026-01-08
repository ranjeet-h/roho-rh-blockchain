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
    
    // Detect CPU cores
    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    println!("Miner initialized for address: {}", address);
    println!("Parallel Mining: ON ({} cores)", num_threads);
    println!("Node started on port 8333");
    println!("Press Ctrl+C to stop.");
    println!();

    // Spawn mining orchestration task
    let miner_state = chain_state.clone();
    let miner_instance = miner.clone();
    
    tokio::spawn(async move {
        loop {
            // Construct block template (requires lock)
            let block_template = {
                let state = miner_state.lock().unwrap();
                miner_instance.assemble_block(&state, vec![])
            };
            
            // Create a channel to receive results from workers
            let (tx, mut rx) = tokio::sync::mpsc::channel(num_threads);
            miner_instance.reset(); // Ensure stop signal is cleared
            
            // Spawn worker threads
            for i in 0..num_threads {
                let m = miner_instance.clone();
                let tx_worker = tx.clone();
                let mut block = block_template.clone();
                
                // Offset start nonces to avoid duplicate work
                block.header.nonce = i as u64 * (u64::MAX / num_threads as u64);
                
                tokio::task::spawn_blocking(move || {
                    let result = m.mine_block(block);
                    let _ = tx_worker.blocking_send(result);
                });
            }

            // Clean up: drop the original sender so rx closes when all workers finish
            drop(tx);

            // Wait for a result
            let mut found_block = None;
            while let Some(result) = rx.recv().await {
                if let MiningResult::Success(block) = result {
                    found_block = Some(block);
                    // Stop all other workers immediately
                    miner_instance.stop();
                    break;
                }
            }

            if let Some(block) = found_block {
                let mut state = miner_state.lock().unwrap();
                state.apply_block(&block);
                let stats = state.get_stats();
                let miner_balance = state.utxo_set.get_balance(&pubkey_hash);
                
                println!("⛏️  Block #{} | Tip: {}... | Miner: {} RH | Supply: {} RH", 
                    stats.height, 
                    &stats.tip_hash.to_string()[..12],
                    miner_balance / 100_000_000,
                    stats.total_issued / 100_000_000
                );
            } else {
                // All workers stopped without success (interrupted)
                sleep(Duration::from_millis(100)).await;
            }
        }
    });

    // P2P Listener Loop
    let listener = TcpListener::bind("0.0.0.0:8333").await?;

    tokio::select! {
        _ = async {
            loop {
                match listener.accept().await {
                    Ok((_socket, addr)) => {
                        println!("Peer connected: {}", addr);
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
