use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, thread::{self, JoinHandle}, time::{SystemTime, UNIX_EPOCH}};

use anyhow::Result;

#[allow(unused)]
use log::{info, error, warn};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use rand::RngCore;
use tokio::sync::{RwLock, mpsc};

use crate::network::{NetworkCommand, Node};

pub type HashDigest = [u8; 32];

type Nonce = [u8; 16];

use crate::transactions::Transaction;

pub enum MiningCommand{
    Stop,
    UpdateBlock,
}

pub fn sha256(message: String) -> HashDigest{
    let mut hasher = Sha256::new();
    hasher.update(message.as_bytes());
    hasher.finalize().into()
}

pub fn get_timestamp() -> usize{
    let now = SystemTime::now();
    now.duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as usize
}

fn get_nonce() -> Nonce{
    let mut nonce: Nonce = [0u8; 16];
    rand::rng().fill_bytes(&mut nonce);
    nonce
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block{
    pub block_header: BlockHeader,
    pub transactions: Vec<Transaction>,
    transaction_count: usize,
}


impl Block{
    pub fn new(
        transactions: Vec<Transaction>, 
        prev_hash: HashDigest, 
        difficulty: usize, 
        version: usize, 
        height: usize 
    ) -> Self{
        
        let merkle_root = Block::get_merkle_root(transactions.clone());
        let transaction_count = transactions.len();

        Self { 
            block_header: BlockHeader::new(prev_hash, merkle_root, version, difficulty, height), 
            transactions, 
            transaction_count}
    }

    pub fn get_merkle_root(transactions: Vec<Transaction>) -> HashDigest{
        Self::rec_merkle_root(transactions.iter().map(|tx| tx.serialize()).collect())
    }

    pub fn to_string(&self) -> String{
        serde_json::to_string(self).unwrap()
    }

    fn update_nonce(&mut self, nonce: Nonce){
        self.block_header.nonce = nonce
    }

    fn meets_difficulty(&self, hash: &str, target: usize) -> bool{
        hash.starts_with(&"0".repeat(target))
    }
    
    pub fn calculate_hash(&self) -> HashDigest{
        sha256(self.to_string())
    }

    pub fn mine(&mut self, stop: Arc<AtomicBool>, id: usize, network_tx: mpsc::Sender<NetworkCommand>){
        info!("Thread {} Started mining", id);

        let mut nonce: Nonce;

        let mut count: usize = 1;

        let target = self.block_header.difficulty;

        while !stop.load(Ordering::Relaxed){
            nonce = get_nonce();
            self.update_nonce(nonce);
            if count%250000 == 0 && id==0{
                if count < 1_000_000{
                    info!("each thread tried {},000 blocks", count/1_000);
                }
                else{
                    info!("each thread tried {},{:03},000 blocks", count/1_000_000, (count%1_000_000)/1_000)
                }
            }
            count += 1;
            let hash = self.calculate_hash();
            if self.meets_difficulty(&String::from_utf8_lossy(&hash), target){
                info!("Mined: {:?}", self.block_header);
                if let Err(e) = network_tx.try_send(NetworkCommand::Block(self.clone())){
                    error!("Issue sending messages: {}", e);
                    return
                }
            }
        }
    }
    
    


    fn rec_merkle_root(transactions: Vec<String>) -> HashDigest{
        match transactions.len(){
            0 => {
                sha256("0000".to_string())
            }
            1 => {
                let message = transactions[0].repeat(2);
                sha256(message)
                }
            2 => {
            let message = transactions[..2].join("");
            sha256(message)
            }
            _ => {
                let mut stack = Vec::new();
                let mut message: String;
                for pair in transactions.chunks(2) {
                    if pair.len() == 2{
                        message = pair[..2].join("");
                        
                    }
                    else{
                        message = pair[0].repeat(2);
                    }
                    stack.push(String::from_utf8_lossy(&sha256(message.clone())).to_string());
                    message.clear();
                }
                Self::rec_merkle_root(stack)
            }
        }
        
    }


}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockHeader{
    pub prev_hash: HashDigest, 
    merkle_root: HashDigest, 
    timestamp: usize,
    difficulty: usize,
    nonce: Nonce,
    version: usize,
    pub height: usize,
}

impl BlockHeader{
    pub fn new(prev_hash: HashDigest, merkle_root: HashDigest, version: usize, difficulty: usize, height: usize) -> Self{
        Self { 
            prev_hash, 
            merkle_root, 
            timestamp: get_timestamp(), 
            difficulty, 
            height,
            nonce: [0u8; 16], 
            version 
        }
    }
    pub fn to_string(&self) -> String{
        serde_json::to_string(self).unwrap()
    }
}


fn spawn_threads(block: Block, stop: Arc<AtomicBool>, network_tx: mpsc::Sender<NetworkCommand>) -> Vec<JoinHandle<()>>{


     let num_threads = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

     info!("New block: {:?}", block);
     info!("Spawning: {} mining_threads for block: {}", num_threads, block.block_header.height);
    

    let mut handles = Vec::new();

    for i in 0..num_threads{
        let mut block_clone = block.clone();
        let stop_clone = stop.clone();
        let network_tx_clone = network_tx.clone();
        let handle = thread::spawn(move || {
            block_clone.mine(stop_clone, i, network_tx_clone);
         });
         handles.push(handle);
    }
    handles
}

pub async fn start_mine_handling(mut mining_rx : mpsc::Receiver<MiningCommand>, node: Arc<RwLock<Node>>, network_tx: mpsc::Sender<NetworkCommand>) -> Result<()>{
    info!("Starting Mine Handling ...");

    let mut stop = Arc::new(AtomicBool::new(false));

    let block = node.write().await.get_next_block();


    let mut  handles = spawn_threads(block, Arc::clone(&stop), network_tx.clone());


    while let Some(msg) = mining_rx.recv().await{
        match msg {
            MiningCommand::Stop => {
                info!("Shutting down miner threads");
                stop.store(true, Ordering::Relaxed);
                break; // Exit the loop
            }
            MiningCommand::UpdateBlock => {
                info!("Updating block");
                stop.store(true, Ordering::Relaxed);

                for handle in handles {
                    handle.join().unwrap();
                }

                stop = Arc::new(AtomicBool::new(false));
                let block = node.write().await.get_next_block();
                handles = spawn_threads(block, Arc::clone(&stop), network_tx.clone());
            
            }
        }
        info!("Handled mining command");
    };

    for handle in handles{
        handle.join().unwrap();
    }
    
    
   
    
    Ok(())
} 

