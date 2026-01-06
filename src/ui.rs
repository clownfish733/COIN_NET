use axum::{
    Router,
    Json,
    response::Html,
    routing::{get, post},
    extract::State,
};
use serde::{Deserialize, Serialize};
#[allow(unused)]
use log::{info, warn};
use tower_http::services::ServeDir;

use tokio::{net::TcpListener, sync::{RwLock, mpsc}};

use std::{
    fs::File,
    path::PathBuf, 
    sync::{Arc, atomic::{AtomicBool, Ordering}},
    collections::HashMap
};

use crate::{
    network::{Node, NetworkCommand},
    transactions::Transaction,
};

use anyhow::Result;

const FILE_PATH: &str = "configs/AddressBook.json";

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

#[derive(Serialize, Deserialize)]
struct AddressBook(HashMap<String, String>);

impl AddressBook{
    fn new() -> Self{
        Self(HashMap::new())
    }
    fn load() -> Self{
        if let Ok(file) = File::open(FILE_PATH){
            let address_book: Self = serde_json::from_reader(file).unwrap();
            address_book
        }else{
            AddressBook::new()
        }
    }

    fn save(&self){
        let file = File::create(FILE_PATH).unwrap();
        serde_json::to_writer(file, self).unwrap();

    }
}

async fn check_save_request(State(state): State<AppState>) -> Json<serde_json::Value>{
    let should_save = state.save_requested.swap(false, Ordering::SeqCst);
    Json(serde_json::json!({"save": should_save }))
}

async fn get_address_book() -> Json<AddressBook>{
    Json(AddressBook::load())
}

async fn save_address_book(
    Json(address_book): Json<AddressBook>
) -> Json<serde_json::Value>{
    address_book.save();
    Json(serde_json::json!({"success": true}))
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
    info!("New Transaction");
    info!("\tRecipients:");
    for (to, amount) in req.to.iter().zip(req.to_amount.iter()) {
        info!("\t\t{}:{}", to, amount)
    }
    info!("\tFee: {}", req.fee);

    let mut total_spend: usize = req.to_amount.iter().sum();
    total_spend += req.fee;
    if let Some((inputs, excess)) = state.node.read().await.wallet.get_inputs(total_spend){
        let mut outputs: Vec<(String, usize)> = req.to.iter().cloned().zip(req.to_amount).collect();
        outputs.push((hex::encode(state.node.read().await.user.get_pub_key().clone()), excess - total_spend));
        let tx = {
            let node_read = state.node.read().await;
            Transaction::new(node_read.version, node_read.user.clone(), inputs, outputs)
        };
        state.network_tx.send(NetworkCommand::Transaction(tx)).await.unwrap();
        Json(TransactionResponse { 
            success:true, 
            message: "Transaction being broadcasted".to_string()
        }) 
    }else{
        
        Json(TransactionResponse { 
            success: false, 
            message: format!("Amount larger: {} than currently available {}", total_spend, state.node.read().await.wallet.value)
        })
    }




    
}

async fn get_node_status(State(state): State<AppState>) -> Json<NodeStatus>{
    let node_read = state.node.read().await;
    Json(NodeStatus { 
        height: node_read.height, 
        mempool_size: node_read.get_mempool_size(), 
        difficulty: node_read.difficulty
    })
}

async fn get_user_status(State(state): State<AppState>) -> Json<UserStatus>{
    let wallet_read = state.node.read().await.wallet.clone();
    Json(UserStatus { 
        amount: wallet_read.value, 
        pk: hex::encode(wallet_read.pub_key) 
    })
}

#[derive(Clone)]
struct AppState{
    node: Arc<RwLock<Node>>,
    network_tx: mpsc::Sender<NetworkCommand>,
    save_requested: Arc<AtomicBool>,
}


pub async fn start_server(node: Arc<RwLock<Node>>, network_tx: mpsc::Sender<NetworkCommand>, save_requested: Arc<AtomicBool>) -> Result<()>{
    let static_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/static");

    let state = AppState{
        node,
        network_tx,
        save_requested
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/api/transaction", post(submit_transaction))
        .route("/api/node_status", get(get_node_status))
        .route("/api/user_status", get(get_user_status))
        .route("/api/address_book", get(get_address_book))
        .route("/api/address_book", post(save_address_book))
        .route("/api/save_check", get(check_save_request))
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
