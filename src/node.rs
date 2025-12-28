mod network;
mod miner;
mod messages;

use env_logger::{Builder};
use network::{start_network_handling, Node};
use miner::{start_mine_handling, MiningCommand};

use anyhow::Result;

use log::{info, error, warn, Level};

use tokio::sync::{RwLock, mpsc};

use std::{fs::{self, File}, io::{BufReader, Write}, net::{SocketAddr, ToSocketAddrs}, sync::Arc};

use crate::{messages::Address, network::NetworkCommand};

fn get_bootstrap() -> Result<Vec<SocketAddr>>{
    let file = File::open("configs/Bootstrap.json")?;
    let reader = BufReader::new(file);

    let bootstrap: Vec<SocketAddr> = serde_json::from_reader(reader)?;
    Ok(bootstrap)
}   

#[tokio::main]
async fn main() -> Result<()>{

//configuring logging environment
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)  // default level
        .init();

    info!("Starting Node ...");

    let net_addr = "0.0.0.0:8081";

    let bootstrap = match get_bootstrap(){
        Ok(bootstrap) => bootstrap,
        Err(e) => {
            error!("Couldnt read bootstrap: {}", e);
            return Err(e)
        }
    };
    

    let address: Address = [2u8; 20];

    let node = Arc::new(RwLock::new(Node::new(address.clone())));

    let (miner_tx, miner_rx) = mpsc::channel::<MiningCommand>(10);

    let (network_tx, network_rx) = mpsc::channel::<NetworkCommand>(10);

    let node_clone = Arc::clone(&node);
    let miner_tx_clone = miner_tx.clone();


    tokio::spawn(async move {
    if let Err(e) = start_network_handling(&net_addr.to_string(), node_clone, miner_tx_clone, network_rx).await {
        error!("Network handling failed: {}", e);
    }
    });
    for peer in bootstrap{
        network_tx.send(NetworkCommand::Connect(peer)).await.unwrap();
    }

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