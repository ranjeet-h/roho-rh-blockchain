//! ROHO (RH) Blockchain Node
//! 
//! Main entry point for running a ROHO node.
//! RH is the short form used in addresses and logos.

use rh_core::node::{create_genesis_block, GenesisInfo};
use rh_core::storage::{ChainState, UTXO, db::BlockChainDB};
use rh_core::mining::{Miner, MiningResult};
use rh_core::wallet::Wallet;
use rh_core::p2p::{Message, PeerManager, VersionMessage, PROTOCOL_VERSION, NETWORK_MAGIC, InvItem, InvType};
use rh_core::crypto::Hash;
use rh_core::rpc::{start_rpc_server, RpcState};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::sleep;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let connect_addr = args.iter()
        .position(|a| a == "--connect")
        .and_then(|i| args.get(i + 1));
    
    let p2p_port: u16 = args.iter()
        .position(|a| a == "--p2p-port")
        .and_then(|i| args.get(i + 1))
        .and_then(|p| p.parse().ok())
        .unwrap_or(8333);

    let rpc_port: u16 = args.iter()
        .position(|a| a == "--rpc-port")
        .and_then(|i| args.get(i + 1))
        .and_then(|p| p.parse().ok())
        .unwrap_or(8334);

    let miner_address: Option<String> = args.iter()
        .position(|a| a == "--miner-address")
        .and_then(|i| args.get(i + 1))
        .cloned();

    let db_path: String = args.iter()
        .position(|a| a == "--db-path")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "rh_data".to_string());

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

    // Initialize state and peer manager
    let genesis = create_genesis_block();
    
    let chain_state = if std::path::Path::new(&db_path).exists() {
        match BlockChainDB::open(&db_path) {
            Ok(db) => match ChainState::restore(db.clone()) {
                 Ok(state) => {
                      println!("✅ Blockchain state restored from {}", db_path);
                      Arc::new(Mutex::new(state))
                  },
                  Err(e) => {
                      eprintln!("❌ Failed to restore state: {}. Attempting database recovery...", e);
                      
                      // Try to remove corrupted database
                      if let Err(removal_err) = std::fs::remove_dir_all(&db_path) {
                          eprintln!("⚠️  Could not remove corrupted database: {}. Starting in-memory only.", removal_err);
                           Arc::new(Mutex::new(ChainState::new(&genesis)))
                      } else {
                          println!("♻️  Corrupted database removed. Starting fresh...");
                          let mut state = ChainState::new(&genesis);
                          match BlockChainDB::open(&db_path) {
                              Ok(new_db) => {
                                  // Save genesis block and metadata
                                  let _ = new_db.save_block(&genesis);
                                  let _ = new_db.update_metadata(&state.tip_hash, state.height, state.total_issued);
                                  
                                  // Save genesis UTXOs
                                  let mut new_utxos = Vec::new();
                                  for tx in &genesis.transactions {
                                      for (i, output) in tx.outputs.iter().enumerate() {
                                          new_utxos.push((
                                              (tx.hash(), i as u32),
                                              UTXO {
                                                  amount: output.amount,
                                                  pubkey_hash: output.pubkey_hash,
                                                  height: 0,
                                              }
                                          ));
                                      }
                                  }
                                  let _ = new_db.update_utxos(&[], &new_utxos);
                                  
                                  state.set_db(new_db);
                                  println!("📦 Database recreated at {}", &db_path);
                                  Arc::new(Mutex::new(state))
                              }
                              Err(creation_err) => {
                                  eprintln!("❌ Failed to recreate database: {}. State will be in-memory only.", creation_err);
                                  Arc::new(Mutex::new(state))
                              }
                          }
                      }
                  }
            },
            Err(e) => {
                 eprintln!("❌ Failed to open database: {}. Starting from genesis (in-memory).", e);
                 Arc::new(Mutex::new(ChainState::new(&genesis)))
            }
        }
    } else {
        println!("✨ Initializing new blockchain state...");
        let mut state = ChainState::new(&genesis);
        match BlockChainDB::open(&db_path) {
            Ok(db) => {
                // Save genesis block and metadata
                let _ = db.save_block(&genesis);
                let _ = db.update_metadata(&state.tip_hash, state.height, state.total_issued);
                
                // Save genesis UTXOs
                let mut new_utxos = Vec::new();
                for tx in &genesis.transactions {
                    for (i, output) in tx.outputs.iter().enumerate() {
                        new_utxos.push((
                            (tx.hash(), i as u32),
                            UTXO {
                                amount: output.amount,
                                pubkey_hash: output.pubkey_hash,
                                height: 0,
                            }
                        ));
                    }
                }
                let _ = db.update_utxos(&[], &new_utxos);
                
                state.set_db(db);
                println!("📦 Database created at {}", &db_path);
            }
            Err(e) => eprintln!("❌ Warning: Failed to create database: {}. State will be in-memory only.", e),
        }
        Arc::new(Mutex::new(state))
    };

    let peer_manager = Arc::new(Mutex::new(PeerManager::new(25)));

    // ... (existing display logic) ...
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

    // Create or load a wallet
    let wallet_path = "wallet.dat";
    let wallet = if std::path::Path::new(wallet_path).exists() {
        match Wallet::load(wallet_path) {
            Ok(w) => {
                println!("📂 Loaded wallet from {}", wallet_path);
                Arc::new(Mutex::new(w))
            }
            Err(e) => {
                eprintln!("❌ Failed to load wallet: {}. Creating new one.", e);
                let mut w = Wallet::new();
                w.generate_key();
                let _ = w.save(wallet_path);
                Arc::new(Mutex::new(w))
            }
        }
    } else {
        println!("✨ Creating new wallet: {}", wallet_path);
        let mut w = Wallet::new();
        w.generate_key();
        let _ = w.save(wallet_path);
        Arc::new(Mutex::new(w))
    };
    
    // Determine miner address - use CLI arg if provided, else use first address in wallet
    let (address, pubkey_hash) = if let Some(addr) = miner_address {
        // Use provided address
        match rh_core::wallet::address_to_pubkey_hash(&addr) {
            Ok(hash) => (addr, hash),
            Err(e) => {
                eprintln!("❌ Invalid miner address: {}. Using default wallet address.", e);
                let w = wallet.lock().unwrap();
                let first_addr = w.get_addresses()[0].to_string();
                let hash = rh_core::wallet::address_to_pubkey_hash(&first_addr).unwrap();
                (first_addr, hash)
            }
        }
    } else {
        // Use first address in wallet
        let w = wallet.lock().unwrap();
        let first_addr = w.get_addresses()[0].to_string();
        let hash = rh_core::wallet::address_to_pubkey_hash(&first_addr).unwrap();
        (first_addr, hash)
    };
    
    // Create shared mining state to allow dynamic switching
    let shared_miner_address = Arc::new(Mutex::new(address.clone()));
    let shared_miner_pubkey_hash = Arc::new(Mutex::new(pubkey_hash));

    // Create miner
    let miner = Miner::new(shared_miner_pubkey_hash.clone());
    
    // Detect CPU cores
    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    println!("⛏️  Mining to address: {}", address);
    println!("Parallel Mining: ON ({} cores)", num_threads);
    println!("Node started on port 8333");
    println!("RPC API available on http://localhost:{}", rpc_port);
    println!("Press Ctrl+C to stop.");
    println!();

    // Spawn RPC server
    let rpc_state = Arc::new(RpcState {
        chain_state: chain_state.clone(),
        wallet: wallet.clone(),
        peer_manager: peer_manager.clone(),
        miner_address: shared_miner_address,
        miner_pubkey_hash: shared_miner_pubkey_hash,
    });
    tokio::spawn(start_rpc_server(rpc_state, rpc_port));

    // Create a flag to signal shutdown to mining task
    let shutdown_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let shutdown_flag_clone = shutdown_flag.clone();
    
    // Spawn mining orchestration task
    let miner_state = chain_state.clone();
    let miner_instance = miner.clone();
    let pm_mining = peer_manager.clone();
    
    tokio::spawn(async move {
        let peer_manager = pm_mining;
        loop {
            // Check if shutdown was requested
            if shutdown_flag_clone.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            // Check if we are syncing (any peer has higher height)
            let is_syncing = {
                let pm = peer_manager.lock().unwrap();
                let state = miner_state.lock().unwrap();
                pm.get_peers_with_height(state.height).len() > 0
            };

            if is_syncing {
                // Wait while peers are ahead
                sleep(Duration::from_secs(2)).await;
                continue;
            }

            // Construct block template (requires lock)
            let block_template = {
                let state = miner_state.lock().unwrap();
                let txs = state.get_mempool_transactions();
                miner_instance.assemble_block(&state, txs)
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
                
                if let Err(e) = state.apply_block(&block) {
                    eprintln!("❌ Error applying self-mined block: {}", e);
                    continue; 
                }
                let stats = state.get_stats();
                let miner_balance = state.utxo_set.get_balance(&pubkey_hash);
                
                println!("⛏️  Block #{} | Tip: {}... | Miner: {:.2} RH | Mined Supply: {:.2} RH", 
                    stats.height, 
                    &stats.tip_hash.to_string()[..12],
                    miner_balance as f64 / 100_000_000.0,
                    stats.total_issued as f64 / 100_000_000.0
                );

                // Broadcast new block to peers
                let inv_msg = Message::Inv(vec![InvItem {
                    inv_type: InvType::Block,
                    hash: block.hash(),
                }]);
                
                // We'll need a way to reach connected peers. 
                // For this ceremony version, we'll store peer senders in PeerManager.
                {
                    let pm = peer_manager.lock().unwrap();
                    pm.broadcast_message(&inv_msg);
                }
            } else {
                // All workers stopped without success (interrupted)
                sleep(Duration::from_millis(100)).await;
            }
        }
    });

    // P2P Listener
    let listener = TcpListener::bind(format!("0.0.0.0:{}", p2p_port)).await?;

    // Outbound connection if requested, otherwise connect to seed nodes
    if let Some(addr_str) = connect_addr {
        let state = chain_state.clone();
        let pm = peer_manager.clone();
        let m_instance = miner.clone();
        let addr = addr_str.parse::<std::net::SocketAddr>()?;
        
        tokio::spawn(async move {
            println!("Connecting to peer: {}...", addr);
            match TcpStream::connect(addr).await {
                Ok(stream) => {
                    let _ = handle_peer(stream, addr, state, pm, m_instance).await;
                },
                Err(e) => eprintln!("Failed to connect to {}: {}", addr, e),
            }
        });
    } else {
        // Auto-connect to seed nodes for peer discovery
        println!("📡 No manual peer specified. Connecting to seed nodes...");
        for seed_addr_str in rh_core::constants::SEED_NODES {
            let state = chain_state.clone();
            let pm = peer_manager.clone();
            let m_instance = miner.clone();
            let seed_str = seed_addr_str.to_string();
            
            tokio::spawn(async move {
                if let Ok(addr) = seed_str.parse::<std::net::SocketAddr>() {
                    match TcpStream::connect(addr).await {
                        Ok(stream) => {
                            println!("🌱 Connected to seed node: {}", addr);
                            let _ = handle_peer(stream, addr, state, pm, m_instance).await;
                        },
                        Err(e) => eprintln!("Failed to connect to seed node {}: {}", addr, e),
                    }
                } else {
                    eprintln!("Invalid seed node address: {}", seed_str);
                }
            });
        }
    }

    tokio::select! {
        _ = async {
            loop {
                match listener.accept().await {
                    Ok((socket, addr)) => {
                        let state = chain_state.clone();
                        let pm = peer_manager.clone();
                        let m_instance = miner.clone();
                        tokio::spawn(async move {
                            let _ = handle_peer(socket, addr, state, pm, m_instance).await;
                        });
                    }
                    Err(e) => eprintln!("Connection error: {}", e),
                }
            }
        } => {},
        _ = tokio::signal::ctrl_c() => {
            println!("\nShutdown signal received. Waiting for pending operations...");
            shutdown_flag.store(true, std::sync::atomic::Ordering::Relaxed);
            miner.stop();
            
            // Give mining/block application time to complete (max 30 seconds)
            for _ in 0..30 {
                let state = chain_state.lock().unwrap();
                // If we're not in the middle of block operations, safe to exit
                drop(state);
                sleep(Duration::from_millis(100)).await;
            }
            
            println!("Stopping node...");
        }
    }

    Ok(())
}

async fn handle_peer(
    mut stream: TcpStream, 
    addr: std::net::SocketAddr, 
    chain_state: Arc<Mutex<ChainState>>,
    peer_manager: Arc<Mutex<PeerManager>>,
    miner: Miner,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🤝 Peer connected: {}", addr);

    // 1. Initial Handshake
    let local_height = { chain_state.lock().unwrap().height };
    let version_msg = Message::Version(VersionMessage {
        version: PROTOCOL_VERSION,
        best_height: local_height,
        from_addr: "127.0.0.1:8333".parse()?,
        to_addr: addr,
        nonce: rand::random(),
        user_agent: "roho-v1.4".to_string(),
    });
    
    send_message(&mut stream, &version_msg).await?;

    // Create a channel for outbound messages to this peer
    let (peer_tx, mut peer_rx) = tokio::sync::mpsc::channel::<Message>(100);

    // Split stream for concurrent read/write
    let (mut reader, mut writer) = stream.into_split();

    // Outbound message task
    tokio::spawn(async move {
        while let Some(msg) = peer_rx.recv().await {
            let bytes = msg.to_bytes();
            if let Err(e) = writer.write_all(&bytes).await {
                eprintln!("Failed to send to {}: {}", addr, e);
                break;
            }
        }
    });

    // 2. Message Loop
    loop {
        match read_message_stream(&mut reader).await {
            Ok(msg) => {
                match msg {
                    Message::Version(v) => {
                        println!("👋 Peer version: {} (Height: {})", v.user_agent, v.best_height);
                        let _ = peer_tx.send(Message::VerAck).await;
                        
                        // Register peer in manager
                        {
                            let mut pm = peer_manager.lock().unwrap();
                            pm.add_peer(addr);
                            pm.peer_connected(addr, v.version, v.best_height, peer_tx.clone());
                        }

                        // If they are ahead, request block hashes
                        if v.best_height > local_height {
                            let locators = {
                                let state = chain_state.lock().unwrap();
                                let mut l = Vec::new();
                                l.push(state.tip_hash);
                                l
                            };
                            let _ = peer_tx.send(Message::GetBlocks(rh_core::p2p::GetBlocksMessage {
                                block_locators: locators,
                                stop_hash: rh_core::crypto::Hash::zero(),
                            })).await;
                        }
                    },
                    Message::Inv(items) => {
                        for item in items {
                            match item.inv_type {
                                InvType::Block => {
                                    let _ = peer_tx.send(Message::GetData(vec![item])).await;
                                }
                                InvType::Transaction => {
                                    // Only request if we don't have it in mempool
                                    let has_tx = {
                                        let state = chain_state.lock().unwrap();
                                        state.get_mempool_transactions().iter().any(|tx| tx.hash() == item.hash)
                                    };
                                    if !has_tx {
                                        let _ = peer_tx.send(Message::GetData(vec![item])).await;
                                    }
                                }
                            }
                        }
                    },
                    Message::GetData(items) => {
                        for item in items {
                            match item.inv_type {
                                InvType::Block => {
                                    let block = {
                                        let state = chain_state.lock().unwrap();
                                        state.get_block(&item.hash).cloned()
                                    };
                                    if let Some(b) = block {
                                        let _ = peer_tx.send(Message::Block(b)).await;
                                    }
                                }
                                InvType::Transaction => {
                                    let tx = {
                                        let state = chain_state.lock().unwrap();
                                        state.get_mempool_transactions().into_iter().find(|tx| tx.hash() == item.hash)
                                    };
                                    if let Some(t) = tx {
                                        let _ = peer_tx.send(Message::Tx(t)).await;
                                    }
                                }
                            }
                        }
                    },
                    Message::Block(block) => {
                        let (result, request_missing) = {
                            let mut state = chain_state.lock().unwrap();
                            let block_hash = block.hash();
                            
                            // 1. Index the block (even if it's on a side chain)
                            state.index_block(&block);

                            if block.header.prev_hash == state.tip_hash {
                                // 2. Direct connection to current tip
                                match state.apply_block(&block) {
                                    Ok(_) => (Some(state.get_stats()), None),
                                    Err(e) => {
                                        eprintln!("❌ Invalid block #{} from peer: {}", state.height + 1, e);
                                        (None, None)
                                    }
                                }
                            } else {
                                // 3. Fork detection / Higher chain?
                                let peer_height = state.get_block_height(&block_hash).unwrap_or(0);
                                if peer_height > state.height {
                                     match state.reorganize(block_hash) {
                                         Ok(_) => {
                                             println!("✅ Successfully reorganized to better chain height {}", state.height);
                                             (Some(state.get_stats()), None)
                                         }
                                         Err(_) => {
                                             // If we are missing intermediate blocks, request them
                                             if state.get_block_header(&block.header.prev_hash).is_none() {
                                                 let locator = rh_core::p2p::build_block_locator(&[state.height], |h| state.get_block_hash_at_height(h));
                                                 (None, Some(locator))
                                             } else {
                                                 (None, None)
                                             }
                                         }
                                     }
                                } else {
                                    (None, None)
                                }
                            }
                        };

                        if let Some(locator) = request_missing {
                             println!("❓ Received potentially better block from peer with unknown parent. Requesting history...");
                             let _ = peer_tx.send(Message::GetBlocks(rh_core::p2p::GetBlocksMessage {
                                block_locators: locator,
                                stop_hash: Hash::zero(),
                            })).await;
                        }
                        
                        if let Some(stats) = result {
                            println!("📦 Applied block #{} from peer", stats.height);
                            
                            // Update peer height in manager
                            {
                                let mut pm = peer_manager.lock().unwrap();
                                pm.update_peer_height(&addr, stats.height);
                                pm.broadcast_message(&Message::Inv(vec![InvItem {
                                    inv_type: InvType::Block,
                                    hash: block.hash(),
                                }]));
                            }
                            
                            miner.stop(); // Interrupt to start on new tip
                        }
                    },
                    Message::Tx(tx) => {
                        let added = {
                            let mut state = chain_state.lock().unwrap();
                            match state.add_to_mempool(tx.clone()) {
                                Ok(_) => true,
                                Err(e) => {
                                    eprintln!("❌ Failed to add relay tx {} to mempool: {}", tx.hash(), e);
                                    false
                                }
                            }
                        };

                        if added {
                            println!("📥 Relaying transaction: {}", tx.hash());
                            // Gossip to others
                            let pm = peer_manager.lock().unwrap();
                            pm.broadcast_message(&Message::Inv(vec![InvItem {
                                inv_type: InvType::Transaction,
                                hash: tx.hash(),
                            }]));
                        }
                    },
                    Message::GetBlocks(req) => {
                        let inv_items = {
                            let state = chain_state.lock().unwrap();
                            let mut start_height = 0;
                            
                            // 1. Find the first common block from locators
                            for hash in &req.block_locators {
                                if let Some(h) = state.get_block_height(hash) {
                                    start_height = h + 1;
                                    break;
                                }
                            }

                            // 2. Collect up to 500 block hashes after common point
                            let mut items = Vec::new();
                            let max_height = state.height;
                            for h in start_height..=std::cmp::min(start_height + 500, max_height) {
                                if let Some(hash) = state.get_block_hash_at_height(h) {
                                    items.push(InvItem {
                                        inv_type: InvType::Block,
                                        hash,
                                    });
                                    if hash == req.stop_hash {
                                        break;
                                    }
                                }
                            }
                            items
                        };

                        if !inv_items.is_empty() {
                            let _ = peer_tx.send(Message::Inv(inv_items)).await;
                        }
                    },
                    Message::GetHeaders(req) => {
                        let headers = {
                            let state = chain_state.lock().unwrap();
                            let mut start_height = 0;

                            for hash in &req.block_locators {
                                if let Some(h) = state.get_block_height(hash) {
                                    start_height = h + 1;
                                    break;
                                }
                            }

                            let mut items = Vec::new();
                            let max_height = state.height;
                            for h in start_height..=std::cmp::min(start_height + 2000, max_height) {
                                if let Some(hash) = state.get_block_hash_at_height(h) {
                                    if let Some(header) = state.get_block_header(&hash) {
                                        items.push(header.clone());
                                        if hash == req.stop_hash {
                                            break;
                                        }
                                    }
                                }
                            }
                            items
                        };

                        if !headers.is_empty() {
                            let _ = peer_tx.send(Message::Headers(headers)).await;
                        }
                    },
                    _ => {}
                }
            },
            Err(_) => {
                println!("🔌 Peer disconnected: {}", addr);
                let mut pm = peer_manager.lock().unwrap();
                pm.peer_disconnected(&addr);
                break;
            }
        }
    }
    
    Ok(())
}

async fn send_message(stream: &mut TcpStream, msg: &Message) -> tokio::io::Result<()> {
    let bytes = msg.to_bytes();
    stream.write_all(&bytes).await
}

async fn read_message_stream(stream: &mut tokio::net::tcp::OwnedReadHalf) -> Result<Message, String> {
    let mut magic = [0u8; 4];
    stream.read_exact(&mut magic).await.map_err(|e| e.to_string())?;
    if magic != NETWORK_MAGIC {
        return Err("Invalid magic".to_string());
    }

    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes).await.map_err(|e| e.to_string())?;
    let len = u32::from_le_bytes(len_bytes) as usize;
    
    if len > 4 * 1024 * 1024 {
        return Err("Message too large".to_string());
    }

    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload).await.map_err(|e| e.to_string())?;

    let mut full_msg = Vec::with_capacity(8 + len);
    full_msg.extend_from_slice(&magic);
    full_msg.extend_from_slice(&len_bytes);
    full_msg.extend_from_slice(&payload);
    
    Message::from_bytes(&full_msg)
}
