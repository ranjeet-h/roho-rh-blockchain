//! RPC Method Implementations
//! 
//! Each method corresponds to a JSON-RPC call that external apps can make.

use crate::storage::ChainState;
use crate::wallet::Wallet;
use crate::crypto::Hash;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// JSON-RPC 2.0 Request
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
    pub id: serde_json::Value,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<JsonRpcError>,
    pub id: serde_json::Value,
}

/// JSON-RPC Error
#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

impl JsonRpcResponse {
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn error(id: serde_json::Value, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError { code, message }),
            id,
        }
    }
}

use crate::p2p::PeerManager;

/// RPC Handler State
pub struct RpcState {
    pub chain_state: Arc<Mutex<ChainState>>,
    pub wallet: Arc<Mutex<Wallet>>,
    pub peer_manager: Arc<Mutex<PeerManager>>,
    pub miner_address: Arc<Mutex<String>>,
    pub miner_pubkey_hash: Arc<Mutex<Hash>>,
}

/// Process a JSON-RPC request and return a response
pub fn handle_request(state: &RpcState, request: JsonRpcRequest) -> JsonRpcResponse {
    match request.method.as_str() {
        "getblockcount" => get_block_count(state, request.id),
        "getblockhash" => get_block_hash(state, request.id, request.params),
        "getblock" => get_block(state, request.id, request.params),
        "getbalance" => get_balance(state, request.id, request.params),
        "getnewaddress" => get_new_address(state, request.id),
        "getinfo" => get_info(state, request.id),
        "getmineraddress" => get_miner_address(state, request.id),
        "createrawtransaction" => create_raw_transaction(state, request.id, request.params),
        "signrawtransaction" => sign_raw_transaction(state, request.id, request.params),
        "sendrawtransaction" => send_raw_transaction(state, request.id, request.params),
        "importprivkey" => import_priv_key(state, request.id, request.params),
        _ => JsonRpcResponse::error(
            request.id,
            -32601,
            format!("Method not found: {}", request.method),
        ),
    }
}

/// Returns the current block height
fn get_block_count(state: &RpcState, id: serde_json::Value) -> JsonRpcResponse {
    let chain = state.chain_state.lock().unwrap();
    JsonRpcResponse::success(id, serde_json::json!(chain.height))
}

/// Returns the block hash at a given height
fn get_block_hash(
    state: &RpcState,
    id: serde_json::Value,
    params: Option<serde_json::Value>,
) -> JsonRpcResponse {
    let height = match params {
        Some(serde_json::Value::Array(arr)) if !arr.is_empty() => {
            arr[0].as_u64().unwrap_or(0)
        }
        Some(serde_json::Value::Number(n)) => n.as_u64().unwrap_or(0),
        _ => return JsonRpcResponse::error(id, -32602, "Invalid params: expected height".into()),
    };

    let chain = state.chain_state.lock().unwrap();
    
    match chain.get_block_hash_at_height(height) {
        Some(hash) => JsonRpcResponse::success(id, serde_json::json!(hash.to_string())),
        None => JsonRpcResponse::error(id, -8, format!("Block height {} out of range", height)),
    }
}

/// Returns full block data by hash
fn get_block(
    state: &RpcState,
    id: serde_json::Value,
    params: Option<serde_json::Value>,
) -> JsonRpcResponse {
    let hash_str = match params {
        Some(serde_json::Value::Array(arr)) if !arr.is_empty() => {
            arr[0].as_str().unwrap_or("").to_string()
        }
        Some(serde_json::Value::String(s)) => s,
        _ => return JsonRpcResponse::error(id, -32602, "Invalid params: expected block hash".into()),
    };

    let hash = match Hash::from_hex(&hash_str) {
        Ok(h) => h,
        Err(_) => return JsonRpcResponse::error(id, -5, "Block not found".into()),
    };

    let chain = state.chain_state.lock().unwrap();
    
    match chain.get_block(&hash) {
        Some(block) => {
            let block_info = serde_json::json!({
                "hash": block.hash().to_string(),
                "height": chain.get_block_height(&hash).unwrap_or(0),
                "previousblockhash": block.header.prev_hash.to_string(),
                "merkleroot": block.header.merkle_root.to_string(),
                "time": block.header.timestamp,
                "difficulty": block.header.difficulty_target,
                "nonce": block.header.nonce,
                "tx_count": block.transactions.len(),
            });
            JsonRpcResponse::success(id, block_info)
        }
        None => JsonRpcResponse::error(id, -5, "Block not found".into()),
    }
}

/// Returns balance for a given address
fn get_balance(
    state: &RpcState,
    id: serde_json::Value,
    params: Option<serde_json::Value>,
) -> JsonRpcResponse {
    let address = match params {
        Some(serde_json::Value::Array(arr)) if !arr.is_empty() => {
            arr[0].as_str().unwrap_or("").to_string()
        }
        Some(serde_json::Value::String(s)) => s,
        _ => return JsonRpcResponse::error(id, -32602, "Invalid params: expected address".into()),
    };

    // Decode address to pubkey hash
    let pubkey_hash = match crate::wallet::address_to_pubkey_hash(&address) {
        Ok(h) => h,
        Err(_) => return JsonRpcResponse::error(id, -5, "Invalid address".into()),
    };

    let chain = state.chain_state.lock().unwrap();
    let balance = chain.utxo_set.get_balance(&pubkey_hash);
    
    println!("💰 Balance inquiry for {}: {} RH (hash: {})", address, balance as f64 / 100_000_000.0, pubkey_hash);

    // Return balance in RH (divide by 10^8)
    let balance_rh = balance as f64 / 100_000_000.0;
    JsonRpcResponse::success(id, serde_json::json!(balance_rh))
}

/// Generates a new wallet address
fn get_new_address(state: &RpcState, id: serde_json::Value) -> JsonRpcResponse {
    let mut wallet = state.wallet.lock().unwrap();
    let keypair = wallet.generate_key();
    let priv_key_hex = hex::encode(keypair.private_key_bytes());
    
    JsonRpcResponse::success(id, serde_json::json!({
        "address": keypair.address,
        "private_key": priv_key_hex
    }))
}

/// Returns general node information
fn get_info(state: &RpcState, id: serde_json::Value) -> JsonRpcResponse {
    let chain = state.chain_state.lock().unwrap();
    let stats = chain.get_stats();
    
    let info = serde_json::json!({
        "chain": "roho-mainnet",
        "blocks": stats.height,
        "tip": stats.tip_hash.to_string(),
        "difficulty": stats.difficulty,
        "total_issued": stats.total_issued as f64 / 100_000_000.0,
        "utxo_count": stats.utxo_count,
        "version": "1.5.0",
    });
    
    JsonRpcResponse::success(id, info)
}

/// Returns the current miner address and potentially the private key for the web wallet
fn get_miner_address(state: &RpcState, id: serde_json::Value) -> JsonRpcResponse {
    let wallet = state.wallet.lock().unwrap();
    let miner_addr = state.miner_address.lock().unwrap();
    
    if let Some(kp) = wallet.get_key_for_address(&miner_addr) {
        return JsonRpcResponse::success(id, serde_json::json!({
            "address": *miner_addr,
            "private_key": hex::encode(kp.private_key_bytes())
        }));
    }
    JsonRpcResponse::success(id, serde_json::json!({ "address": *miner_addr }))
}

/// Create a raw transaction
/// Params: [to_address, amount_rh]
fn create_raw_transaction(
    state: &RpcState,
    id: serde_json::Value,
    params: Option<serde_json::Value>,
) -> JsonRpcResponse {
    let (to_address, amount_rh, from_address) = match params {
        Some(serde_json::Value::Array(arr)) if arr.len() >= 2 => {
            let to = arr[0].as_str().unwrap_or("").to_string();
            let amount = arr[1].as_f64().unwrap_or(0.0);
            let from = arr.get(2).and_then(|v| v.as_str()).map(|s| s.to_string());
            (to, amount, from)
        }
        _ => return JsonRpcResponse::error(id, -32602, "Invalid params: [to_address, amount, (optional) from_address]".into()),
    };

    let amount_base = (amount_rh * 100_000_000.0) as u64;
    if amount_base == 0 {
        return JsonRpcResponse::error(id, -1, "Amount must be greater than 0".into());
    }

    // Decode recipient address
    let recipient_hash = match crate::wallet::address_to_pubkey_hash(&to_address) {
        Ok(h) => h,
        Err(_) => return JsonRpcResponse::error(id, -5, "Invalid recipient address".into()),
    };

    // For this simplified version, we use the node's wallet to find inputs
    // In a real RPC, the user would provide the source address
    let wallet = state.wallet.lock().unwrap();
    let chain = state.chain_state.lock().unwrap();
    
    // If from_address is provided, only use UTXOs from that address
    // Otherwise use all addresses in wallet
    let addresses: Vec<String> = if let Some(ref from) = from_address {
        vec![from.clone()]
    } else {
        wallet.get_addresses().iter().map(|s| s.to_string()).collect()
    };
    
    let mut selected_utxos = Vec::new();
    let mut total_selected = 0u64;
    let fee = 1000; // Fixed fee for now

    for addr in &addresses {
        let pubkey_hash = match crate::wallet::address_to_pubkey_hash(addr) {
            Ok(h) => h,
            Err(_) => continue,
        };
        let utxos = chain.utxo_set.get_by_pubkey_hash(&pubkey_hash);
        for (key, utxo) in utxos {
            selected_utxos.push((key, utxo.clone()));
            total_selected += utxo.amount;
            if total_selected >= amount_base + fee {
                break;
            }
        }
        if total_selected >= amount_base + fee {
            break;
        }
    }

    if total_selected < amount_base + fee {
        return JsonRpcResponse::error(id, -6, "Insufficient balance".into());
    }

    // Create inputs
    let inputs: Vec<crate::validation::TxInput> = selected_utxos.iter().map(|(key, _)| {
        crate::validation::TxInput {
            prev_tx_hash: key.0,
            output_index: key.1,
            signature: crate::crypto::SchnorrSignature([0u8; 64]),
            public_key: crate::crypto::PublicKey([0u8; 32]),
        }
    }).collect();

    // Create outputs
    let mut outputs = vec![crate::validation::TxOutput {
        amount: amount_base,
        pubkey_hash: recipient_hash,
    }];

    // Change output
    if total_selected > amount_base + fee {
        let change_addr = &addresses[0]; // Send change back to first address
        let change_hash = crate::wallet::address_to_pubkey_hash(change_addr).unwrap();
        outputs.push(crate::validation::TxOutput {
            amount: total_selected - amount_base - fee,
            pubkey_hash: change_hash,
        });
    }

    let tx = crate::validation::Transaction::new(inputs, outputs);
    let tx_bytes = bincode::serialize(&tx).unwrap();
    
    JsonRpcResponse::success(id, serde_json::json!(hex::encode(tx_bytes)))
}

/// Sign a raw transaction
/// Params: [tx_hex, private_key_hex]
fn sign_raw_transaction(
    _state: &RpcState,
    id: serde_json::Value,
    params: Option<serde_json::Value>,
) -> JsonRpcResponse {
    let (tx_hex, priv_key_hex) = match params {
        Some(serde_json::Value::Array(arr)) if arr.len() >= 2 => {
            let tx = arr[0].as_str().unwrap_or("").to_string();
            let key = arr[1].as_str().unwrap_or("").to_string();
            (tx, key)
        }
        _ => return JsonRpcResponse::error(id, -32602, "Invalid params: [tx_hex, priv_key_hex]".into()),
    };

    let tx_bytes = match hex::decode(&tx_hex) {
        Ok(b) => b,
        Err(_) => return JsonRpcResponse::error(id, -22, "Invalid hex for transaction".into()),
    };

    let mut tx: crate::validation::Transaction = match bincode::deserialize(&tx_bytes) {
        Ok(t) => t,
        Err(_) => return JsonRpcResponse::error(id, -22, "Failed to deserialize transaction".into()),
    };

    let priv_key_bytes = match hex::decode(&priv_key_hex) {
        Ok(b) => {
            if b.len() != 32 {
                return JsonRpcResponse::error(id, -5, "Private key must be 32 bytes".into());
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&b);
            key
        }
        Err(_) => return JsonRpcResponse::error(id, -5, "Invalid private key hex".into()),
    };

    // Sign each input
    let signing_hash = tx.signing_hash();
    
    // Use the public field to initialize directly and avoid trait ambiguity
    let key_bytes = match k256::schnorr::SigningKey::from_bytes(priv_key_bytes.as_slice().into()) {
        Ok(k) => k,
        Err(_) => return JsonRpcResponse::error(id, -5, "Invalid private key".into()),
    };
    let private_key = crate::crypto::PrivateKey(key_bytes);
    
    let public_key = private_key.public_key();
    let signature = private_key.sign(&signing_hash).unwrap();

    for input in &mut tx.inputs {
        input.signature = signature.clone();
        input.public_key = public_key.clone();
    }

    let signed_bytes = bincode::serialize(&tx).unwrap();
    JsonRpcResponse::success(id, serde_json::json!(hex::encode(signed_bytes)))
}

/// Broadcast a raw transaction
/// Params: [tx_hex]
fn send_raw_transaction(
    state: &RpcState,
    id: serde_json::Value,
    params: Option<serde_json::Value>,
) -> JsonRpcResponse {
    let tx_hex = match params {
        Some(serde_json::Value::Array(arr)) if !arr.is_empty() => {
            arr[0].as_str().unwrap_or("").to_string()
        }
        Some(serde_json::Value::String(s)) => s,
        _ => return JsonRpcResponse::error(id, -32602, "Invalid params: expected tx_hex".into()),
    };

    let tx_bytes = match hex::decode(&tx_hex) {
        Ok(b) => b,
        Err(_) => return JsonRpcResponse::error(id, -22, "Invalid hex for transaction".into()),
    };

    let tx: crate::validation::Transaction = match bincode::deserialize(&tx_bytes) {
        Ok(t) => t,
        Err(_) => return JsonRpcResponse::error(id, -22, "Failed to deserialize transaction".into()),
    };

    // Add to mempool
    let mut chain = state.chain_state.lock().unwrap();
    match chain.add_to_mempool(tx.clone()) {
        Ok(_) => {
            println!("📥 New transaction added to mempool: {}", tx.hash());
            
            // Broadcast to peers
            let pm = state.peer_manager.lock().unwrap();
            pm.broadcast_message(&crate::p2p::Message::Tx(tx.clone()));

            JsonRpcResponse::success(id, serde_json::json!(tx.hash().to_string()))
        }
        Err(e) => {
            eprintln!("❌ Failed to add tx to mempool: {}", e);
            JsonRpcResponse::error(id, -32603, e)
        }
    }
}

/// Import a private key
/// Params: [priv_key_hex]
fn import_priv_key(
    state: &RpcState,
    id: serde_json::Value,
    params: Option<serde_json::Value>,
) -> JsonRpcResponse {
    let priv_key_hex = match params {
        Some(serde_json::Value::Array(arr)) if !arr.is_empty() => {
            arr[0].as_str().unwrap_or("").to_string()
        }
        Some(serde_json::Value::String(s)) => s,
        _ => return JsonRpcResponse::error(id, -32602, "Invalid params: expected priv_key_hex".into()),
    };

    let priv_key_bytes = match hex::decode(&priv_key_hex) {
        Ok(b) => {
            if b.len() != 32 {
                return JsonRpcResponse::error(id, -5, "Private key must be 32 bytes".into());
            }
            let mut key = [0u8; 32];
            key.copy_from_slice(&b);
            key
        }
        Err(_) => return JsonRpcResponse::error(id, -5, "Invalid private key hex".into()),
    };

    let mut wallet = state.wallet.lock().unwrap();
    match wallet.import_key(&priv_key_bytes) {
        Ok(keypair) => {
            println!("🔑 Imported private key for address: {}", keypair.address);
            
            // Automatically switch miner to this newly imported key!
            let mut miner_addr = state.miner_address.lock().unwrap();
            let mut miner_pkh = state.miner_pubkey_hash.lock().unwrap();
            *miner_addr = keypair.address.clone();
            *miner_pkh = keypair.pubkey_hash();
            
            println!("⛏️  Automatically switched miner to address: {}", *miner_addr);
            
            JsonRpcResponse::success(id, serde_json::json!({
                "address": keypair.address,
                "public_key": hex::encode(keypair.public_key.0)
            }))
        }
        Err(e) => {
            eprintln!("❌ Failed to import private key: {}", e);
            JsonRpcResponse::error(id, -1, e.to_string())
        }
    }
}
