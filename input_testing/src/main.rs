use axum::{
    Router,
    Json,
    response::Html,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use log::info;

use tower_http::services::ServeDir;

#[derive(Debug, Deserialize)]
struct TransactionRequest {
    from: String,
    to: String,
    amount: usize,
    fee: usize,
}

#[derive(Serialize)]
struct TransactionResponse {
    success: bool,
    message: String,
}

#[derive(Serialize)]
struct NodeStatus {
    height: usize,
    mempool_size: usize,
    difficulty: usize,
}

// Serve the HTML page
async fn index() -> Html<&'static str> {
    Html(include_str!("static/index.html"))
}

// Handle transaction submission
async fn submit_transaction(
    Json(req): Json<TransactionRequest>,
) -> Json<TransactionResponse> {
    
    // LOG THE TRANSACTION TO TERMINAL
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("📝 NEW TRANSACTION RECEIVED");
    info!("   From:   {}", req.from);
    info!("   To:     {}", req.to);
    info!("   Amount: {} coins", req.amount);
    info!("   Fee:    {} coins", req.fee);
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    Json(TransactionResponse {
        success: true,
        message: format!(
            "Transaction logged: {} → {} ({} coins, fee: {})", 
            req.from, req.to, req.amount, req.fee
        ),
    })
}

// Get node status
async fn get_status() -> Json<NodeStatus> {
    Json(NodeStatus {
        height: 0,
        mempool_size: 0,
        difficulty: 4,
    })
}

#[tokio::main]
async fn main() {
    // Initialize env_logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)  // default level
        .init();

    // Create router
    let app = Router::new()
        .route("/", get(index))
        .route("/api/transaction", post(submit_transaction))
        .route("/api/status", get(get_status))
        .nest_service("/static", ServeDir::new("src/static"));

    let addr = "0.0.0.0:3000";

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    let url = format!("http://{}", addr);

    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("🌐 Web UI running at: {}", url);
    info!("📋 Open your browser and submit transactions!");
    info!("📊 Watch this terminal for transaction logs");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if let Err(e) = open::that(&url) {
        log::warn!("Failed to open browser: {}", e);
        info!("Please open {} manually", url);
    }

    axum::serve(listener, app).await.unwrap();
}