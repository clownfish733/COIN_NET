use anyhow::Result;

#[allow(unused)]
use log::{info, error, warn, Level};

use tokio::sync::{RwLock, mpsc};

use std::{sync::Arc, io::Write};

use log::LevelFilter;

use COIN_NET::{
    network::{NetworkCommand, start_network_handling, Node},
    miner::{start_mine_handling, MiningCommand}
};

const NET_ADDR: &str = "0.0.0.0:8081";

#[tokio::main]
async fn main() -> Result<()>{

//configuring logging environment
    env_logger::builder()
        .format(|buf, record| {
            // Color by log level
            let level_color = match record.level() {
                log::Level::Error => "\x1b[31m", // Red
                log::Level::Warn => "\x1b[33m",  // Yellow
                log::Level::Info => "\x1b[32m",  // Green
                log::Level::Debug => "\x1b[36m", // Cyan
                log::Level::Trace => "\x1b[90m", // Gray
            };
            
            // Color by module/target
            let target = record.target();
            let module_color = if target.contains("network") {
                "\x1b[35m" // Magenta for network
            } else if target.contains("miner") {
                "\x1b[33m" // Yellow for mining
            } else {
                "\x1b[37m" // White for others
            };
            
            writeln!(
                buf,
                "{}{:<5}\x1b[0m {}[{}]\x1b[0m {}",
                level_color,
                record.level(),
                module_color,
                target,
                record.args()
            )
        })
        .filter_level(LevelFilter::Info)
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
    let network_tx_clone = network_tx.clone();
    let miner_handle = tokio::spawn(async move {
    if let Err(e) = start_mine_handling(miner_rx, node_clone, network_tx_clone).await {
        error!("Mine handling failed: {}", e);
    }
    }); 


    tokio::signal::ctrl_c().await?;
    info!("Shutting down ...");
    miner_tx.send(MiningCommand::Stop).await.unwrap();
    miner_handle.await?;
    let blockchain = node.read().await.block_chain.clone();
    for block in blockchain{
        println!("H: {:?}, P: {:?}", block.block_header.height, block.block_header.prev_hash);
    }
    Ok(())
}