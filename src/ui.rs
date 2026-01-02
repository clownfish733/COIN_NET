use axum::{
    Router,
    Json,
    response::Html,
    routing::{get, post},
    extract::State,
};
use serde::{Deserialize, Serialize};
use log::{info, warn};
use tower_http::services::ServeDir;

use tokio::{net::TcpListener, sync::{RwLock, mpsc}};

use std::{
    os::linux::raw::stat, path::PathBuf, sync::Arc
};

use crate::{
    network::{Node, NetworkCommand},
    transactions::Transaction,
};

use anyhow::Result;

#[derive(Debug, Deserialize)]
struct TransactionRequest{
    to: Vec<String>,
    to_amount: Vec<usize>,
    fee: usize,
}

#[derive(Debug, Serialize)]
struct TransactionResponse{
    success: bool,
    message: String,
}

async fn index() -> Html<&'static str>{
    Html(include_str!("static/index.html"))
}

#[derive(Serialize)]
struct NodeStatus{
    height: usize,
    mempool_size: usize,
    difficulty: usize,
}

#[derive(Serialize)]
struct UserStatus{
    amount: usize,
    pk: String,
}

async fn submit_transaction(State(state): State<AppState>, Json(req): Json<TransactionRequest>) -> Json<TransactionResponse>{
    info!("To:");
    for (to, amount) in req.to.iter().zip(req.to_amount.iter()) {
        info!("{}:{}", to, amount)
    }
    info!("Fee: {}", req.fee);

    let mut total_spend: usize = req.to_amount.iter().sum();
    total_spend += req.fee;

    if let Some((inputs, excess)) = state.node.read().await.wallet.get_inputs(total_spend){
        let mut outputs: Vec<(String, usize)> = req.to.iter().cloned().zip(req.to_amount).collect();
        outputs.push((hex::encode(state.node.read().await.wallet.pub_key.clone()), excess - total_spend));
        let tx = Transaction::new(state.node.read().await.version, state.node.read().await.user.clone(), inputs, outputs);
        state.network_tx.send(NetworkCommand::Transaction(tx)).await.unwrap();
    }else{
        warn!("Amount larger: {} than currently available {}", total_spend, state.node.read().await.wallet.value);
    }




    Json(TransactionResponse { 
        success:false, 
        message: "Success".to_string(), 
    })
}

async fn get_node_status() -> Json<NodeStatus>{
    Json(NodeStatus { 
        height: 10, 
        mempool_size: 10, 
        difficulty: 3 
    })
}

async fn get_user_status() -> Json<UserStatus>{
    Json(UserStatus { 
        amount: 10, 
        pk: "1dhassd78ad234892".to_string() 
    })
}

#[derive(Clone)]
struct AppState{
    node: Arc<RwLock<Node>>,
    network_tx: mpsc::Sender<NetworkCommand>,
}


pub async fn start_server(node: Arc<RwLock<Node>>, network_tx: mpsc::Sender<NetworkCommand>) -> Result<()>{
    let static_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/static");

    let state = AppState{
        node,
        network_tx,
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/api/transaction", post(submit_transaction))
        .route("/api/node_status", post(get_node_status))
        .route("/api/user_status", post(get_user_status))
        .nest_service("/static", ServeDir::new(static_dir))
        .with_state(state);

    let addr = "0.0.0.0:3000";

    let listener = TcpListener::bind(addr).await?;

    let url = format!("http://127.0.0.1:3000");
    info!("Web ui running");

    if let Err(e) = webbrowser::open(&url) {
        log::warn!("Failed to open browser: {}", e);
    }

    axum::serve(listener, app).await?;

    Ok(())
}   
