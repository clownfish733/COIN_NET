use anyhow::Result;

#[allow(unused)]
use log::{info, error, warn, Level};

use tokio::sync::{RwLock, mpsc};

use std::{sync::Arc};

use Coin::{
    network::{NetworkCommand, start_network_handling, Node},
    miner::{start_mine_handling, MiningCommand}
};

const NET_ADDR: &str = "0.0.0.0:8081";

#[tokio::main]
async fn main() -> Result<()>{

//configuring logging environment
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)  // default level
        .init();

    info!("Starting Node ...");

    let node = Arc::new(RwLock::new(Node::new()));

    let (miner_tx, miner_rx) = mpsc::channel::<MiningCommand>(10);

    let (network_tx, network_rx) = mpsc::channel::<NetworkCommand>(10);

    let node_clone = Arc::clone(&node);
    let miner_tx_clone = miner_tx.clone();


    tokio::spawn(async move {
    if let Err(e) = start_network_handling(&NET_ADDR.to_string(), node_clone, miner_tx_clone, network_rx).await {
        error!("Network handling failed: {}", e);
    }
    });

    let node_clone = Arc::clone(&node);
    let miner_handle = tokio::spawn(async move {
    if let Err(e) = start_mine_handling(miner_rx, node_clone, network_tx).await {
        error!("Mine handling failed: {}", e);
    }
    }); 


    tokio::signal::ctrl_c().await?;
    info!("Shutting down ...");
    miner_tx.send(MiningCommand::Stop).await.unwrap();
    miner_handle.await?;
    let blockchain = node.read().await.block_chain.clone();
    for block in blockchain{
        println!("{:?}", block);
    }
    Ok(())
}