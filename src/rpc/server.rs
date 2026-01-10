//! RPC HTTP Server
//! 
//! Axum-based HTTP server that handles JSON-RPC requests and serves the block explorer.

use axum::{
    extract::State,
    http::StatusCode,
    response::Html,
    routing::get,
    Json, Router,
};
use tower_http::cors::{Any, CorsLayer};
use std::sync::Arc;
use crate::rpc::methods::{handle_request, JsonRpcRequest, JsonRpcResponse, RpcState};
use crate::explorer::{EXPLORER_HTML, WALLET_HTML};

/// Start the RPC server on the specified port
pub async fn start_rpc_server(state: Arc<RpcState>, port: u16) {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(serve_explorer).post(handle_rpc))
        .route("/wallet", get(serve_wallet))
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    println!("🌐 RPC Server listening on http://{}", addr);
    println!("🔍 Block Explorer at http://localhost:{}", port);
    println!("💎 Wallet at http://localhost:{}/wallet", port);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Serve the block explorer HTML page
async fn serve_explorer() -> Html<&'static str> {
    Html(EXPLORER_HTML)
}

/// Serve the wallet HTML page
async fn serve_wallet() -> Html<&'static str> {
    Html(WALLET_HTML)
}

/// Handle incoming JSON-RPC requests
async fn handle_rpc(
    State(state): State<Arc<RpcState>>,
    Json(request): Json<JsonRpcRequest>,
) -> (StatusCode, Json<JsonRpcResponse>) {
    let response = handle_request(&state, request);
    (StatusCode::OK, Json(response))
}
