use axum::{
    Router,
    Json,
    response::Html,
    routing::{get, post},
    extract::State,
};
use serde::{Deserialize, Serialize};
use log::info;
use tower_http::services::ServeDir;

use tokio::{net::TcpListener, sync::RwLock};

use std::{
    path::PathBuf,
    sync::Arc,
};

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

async fn submit_transaction(Json(req): Json<TransactionRequest>) -> Json<TransactionResponse>{
    info!("To:");
    for (to, amount) in req.to.iter().zip(req.to_amount.iter()) {
        info!("{}:{}", to, amount)
    }
    info!("Fee: {}", req.fee);


    Json(TransactionResponse { 
        success:true, 
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


async fn start_server(node: Arc<RwLock<Node>>){
    let static_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/static");

    let state = AppState{

    }

    let app = Router::new()
        .route("/", get(index))
        .route("/api/transaction", post(submit_transaction))
        .route("/api/node_status", post(get_node_status))
        .route("/api/user_status", post(get_user_status))
        .nest_service("/static", ServeDir::new(static_dir));

    let addr = "0.0.0.0:3000";

    let listener = TcpListener::bind(addr).await.unwrap();

    let url = format!("http://{}", addr);
    info!("Web ui running");

    if let Err(e) = open::that(&url) {
        log::warn!("Failed to open browser");
    }

    axum::serve(listener, app).await.unwrap();
}   

struct Node{
    pk: String,
    height: usize,
    mempool_size: usize,
    amount: usize,
}

#[tokio::main]
async fn main(){
    let node = Arc::new(RwLock::new(Node{
        pk: "adssada".to_string(),
        height: 3,
        mempool_size: 3,
        amount: 100,
    }));

     env_logger::builder()
        .filter_level(log::LevelFilter::Info)  // default level
        .init();

    let node_clone = Arc::clone(&node);
    let handle = tokio::spawn(async move {
        start_server(node_clone)
 
    });
    handle.await.unwrap().await;
}