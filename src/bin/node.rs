use anyhow::{Result, anyhow};

#[allow(unused)]
use log::{info, error, warn, Level, LevelFilter};

use tokio::{sync::{RwLock, mpsc}};

use std::{env, fs::File, io::{BufReader, Write}, net::SocketAddr, sync::Arc, time::Duration};

use COIN_NET::{
    network::{NetworkCommand, start_network_handling, Node},
    miner::{start_mine_handling, MiningCommand}
};

const NET_ADDR: &str = "0.0.0.0:8080";

const FILE_PATH: &str = "configs/node.json";

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


    let node = Arc::new(RwLock::new(match env::args().nth(1).as_deref(){
        Some("load") => Node::load(FILE_PATH)?,
        Some("new") => Node::new(),
        Some(arg) => return Err(anyhow!("Invalid arguement '{}' expected 'new' or 'load'", arg)),
        None => return Err(anyhow!("Missing argument: expected: 'new' or 'load'")),
    }));

    info!("Starting Node ...");

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

    let bootstrap = match get_bootstrap(){
        Ok(bootstrap) => bootstrap,
        Err(e) => {
            error!("Couldnt read bootstrap: {}", e);
            return Err(e)
        }
    };
    
    for peer in bootstrap{
        network_tx.send(NetworkCommand::Connect(peer)).await.unwrap();
    }


    tokio::signal::ctrl_c().await?;
    info!("Shutting down ...");
    miner_tx.send(MiningCommand::Stop).await.unwrap();
    miner_handle.await?;
    node.read().await.store(FILE_PATH)?;
    tokio::time::sleep(Duration::from_millis(300)).await;
    Ok(())
}