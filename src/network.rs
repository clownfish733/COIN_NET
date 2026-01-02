use std::{
    collections::HashMap, fs::File, net::SocketAddr, path::Path, sync::Arc, time::Duration,
};

use anyhow::Result;

use serde::{Deserialize, Serialize};
use sha2::digest::InvalidOutputSize;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream, tcp::{OwnedReadHalf, OwnedWriteHalf}}, sync::{Mutex, RwLock, mpsc}
};

#[allow(unused)]
use log::{error, info, warn};

use crate::{messages::{Blocks, GetBlocks, GetInv, GetPeerAddrs, Inv, Mempool, NewBlock, PeerAddrs, Ping, Pong, TransactionWithFee, Verack}, 
    miner::{Block, BlockHeader, HashDigest, MiningCommand, sha256},
    transactions::{Transaction, UTXOS, User, Wallet},
};

const DIFFICULTY: usize = 3;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node{
    pub user: User,
    pub height: usize,
    pub version: usize,
    mempool: Mempool,
    headers: Vec<BlockHeader>,
    pub block_chain: Vec<Block>,
    difficulty: usize,
    reward: usize,
    utxos: UTXOS,
    pub wallet: Wallet,
}

impl Node{
    pub fn new() -> Self{
        let user = User::new();

        Self { 
            height: 0, 
            version: 0, 
            mempool: Mempool::new(), 
            headers: Vec::new(),
            block_chain: Vec::new(),
            difficulty: DIFFICULTY,
            user: user.clone(),
            reward: 10,
            utxos: UTXOS::new(),
            wallet: Wallet::new(user.get_pub_key())
        }
    }
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self>{
        let file = File::open(path)?;
        let node: Self = serde_json::from_reader(file)?;
        Ok(node)
    }

    pub fn store<P: AsRef<Path>>(&self, path: P) -> Result<()>{
        let file = File::create(path)?;
        serde_json::to_writer_pretty(&file, self)?;
        Ok(())
    }
    /*
    pub fn update_headers(&mut self, headers: Headers){
        if headers.start_height + 1 == self.height{
            for header in headers.headers{
                self.headers.push(header);
                self.height += 1;
            }
        }
    }
    */

    pub fn update_blocks(&mut self, blocks: Blocks){
        if blocks.start_height == self.height + 1{
            for block in blocks.blockchain{
                if self.utxos.add_block(block.clone()){
                    self.block_chain.push(block.clone());
                    self.headers.push(block.block_header.clone());
                    self.height += 1;
                    self.wallet.update(block.clone());

                    for tx in block.transactions.clone(){
                        if tx.input_count != 0{
                        self.mempool.remove(TransactionWithFee::new(tx.clone(), self.utxos.get_fee(tx.clone()).unwrap()));
                        }
                    }   
                }else{
                    warn!("Invalid Block Received");
                }
            }
            info!("New Height: {}", self.height);
        }
    }

    pub fn add_block(&mut self, block: Block) -> bool{
        if block.block_header.height != (self.height + 1) {return false}
        if self.utxos.add_block(block.clone()){
            self.block_chain.push(block.clone());
            self.headers.push(block.block_header.clone());
            self.height += 1;
            info!("Adding block to wallet");
            self.wallet.update(block.clone());
            for tx in block.transactions.clone(){
                if tx.input_count != 0{
                    self.mempool.remove(TransactionWithFee::new(tx.clone(), self.utxos.get_fee(tx.clone()).unwrap()));
                }
            }   
        }
        
        true
    }

    pub fn get_next_transactions(&mut self) -> Vec<Transaction>{
        let mut valid_transactions = true;
        let txs = self.mempool.get_next_transactions();
        for tx in txs.clone(){
            if !self.utxos.validate_transaction(tx.clone()){
                self.mempool.remove(TransactionWithFee::new(tx.clone(), self.utxos.get_fee(tx).unwrap()));
                valid_transactions = false
            }
        }

        if valid_transactions{txs} else {self.get_next_transactions()}
    }

    pub fn get_prev_hash(&self) -> HashDigest{
        match self.block_chain.last(){
            Some(block) => {
                block.calculate_hash()
            }
            None => {
                sha256("00".to_string())
            }
        }
    }

    pub fn get_next_block(&mut self) -> Block{
        let mut next_transactions = self.get_next_transactions();
        next_transactions.push(Transaction::reward(self.reward, self.user.get_pub_key(), self.version));
        Block::new(next_transactions, self.get_prev_hash(), self.difficulty, self.version, self.height.clone() + 1)
    }

    
}

pub enum NetworkCommand{
    Block(Block),
    Transaction(Transaction),
    Connect(SocketAddr),
}


#[derive(Serialize, Deserialize, Debug)]
enum NetMessage{
    NewBlock(NewBlock),
    GetBlocks(GetBlocks),
    Blocks(Blocks),
    Verack(Verack),
    Transaction(Transaction),
    GetInv(GetInv),
    Inv(Inv),
    //GetHeaders(GetHeaders),
    //Headers(Headers),
    GetPeerAddrs(GetPeerAddrs),
    PeerAddrs(PeerAddrs),
    Ping(Ping),
    Pong(Pong),
}

impl NetMessage{
    fn from_string(string: &String) -> Result<Self>{
        let msg = serde_json::from_str::<NetMessage>(string)?;
        Ok(msg)
    }
    fn to_string(&self) -> String{
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Clone)]
struct ConnectionEvent{
    peer: SocketAddr,
    connection_event_type: ConnectionEventType,
}

#[derive(Clone)]
enum ConnectionEventType{
    Close,
    Message(String)
}

impl ConnectionEvent{
    fn close(peer: SocketAddr) -> Self{
        Self { 
            peer, 
            connection_event_type: ConnectionEventType::Close 
        }
    }

    fn message(peer: SocketAddr, message: String) -> Self{
        Self {
             peer, 
             connection_event_type: ConnectionEventType::Message(message)
            }
    }
}

struct ConnectionResponse{
    connection_response_type: ConnectionResponseType,
}

#[allow(unused)]
impl ConnectionResponse{
    fn close() -> Self{
        Self{
            connection_response_type: ConnectionResponseType::Close,
        }
    }
    fn send(string: String) -> Self{
        Self { 
            connection_response_type: ConnectionResponseType::Send(string) 
        }
    }
}


#[allow(unused)]
enum ConnectionResponseType{
    Close,
    Send(String),
}


#[derive(Clone)]
struct PeerInfo{
    tx: mpsc::Sender<ConnectionResponse>
}

impl PeerInfo{
    fn new(tx: mpsc::Sender<ConnectionResponse>) -> Self{
        Self {tx}
    }
}

#[derive(Clone)]
struct PeerManager{
    peers: HashMap<SocketAddr, PeerInfo>
}

impl PeerManager{
    fn new() -> Self{
        Self { peers: HashMap::new() }
    }

    async fn send(&self, peer: &SocketAddr, response: ConnectionResponse) -> Result<()>{
        self.peers.get(&peer).unwrap().tx.send(response).await?;
        Ok(())
    }

    async fn broadcast(&self, message: String){
        info!("Broadcasting: {:?}", &message);
        for (peer, _peer_info) in &self.peers {
            self.send(peer, ConnectionResponse::send(message.clone())).await.unwrap();
        }
    }

    fn remove(&mut self, peer: &SocketAddr){
        self.peers.remove(peer);
    }

    fn add(&mut self, peer: &SocketAddr, tx: mpsc::Sender<ConnectionResponse>){
        self.peers.insert(*peer, PeerInfo::new(tx));
    }

    fn contains(self, peer: &SocketAddr) -> bool{
        self.peers.contains_key(peer)
    }
}

async fn network_command_handling(mut network_rx: mpsc::Receiver<NetworkCommand>, peer_manager: Arc<Mutex<PeerManager>>, node: Arc<RwLock<Node>>, miner_tx: mpsc::Sender<MiningCommand>, handler_tx: mpsc::Sender<ConnectionEvent>){
    while let Some(msg) = network_rx.recv().await{
        match msg {
            NetworkCommand::Block(block) => {
                {
                    let mut  node_lock = node.write().await;
                    if !node_lock.add_block(block.clone()){
                        continue
                    };
                }
                {
                    let peer_manager_lock = peer_manager.lock().await;
                    peer_manager_lock.broadcast(NetMessage::NewBlock(NewBlock::new(block)).to_string()).await;
                }
                miner_tx.send(MiningCommand::UpdateBlock).await.unwrap();

            }
            NetworkCommand::Transaction(transaction) => {
                info!("Transaction preparing");
                if !node.read().await.utxos.validate_transaction(transaction.clone()) {continue}
                let fee = node.read().await.utxos.get_fee(transaction.clone()).unwrap();
                info!("Fee: {}", fee);
                {
                    let mut  node_lock = node.write().await;
                    node_lock.mempool.add(transaction.clone(), fee);
                }
                info!("Attempting to broadcast");
                {
                    
                    let peer_manager_lock = peer_manager.lock().await;
                    peer_manager_lock.broadcast(NetMessage::Transaction(transaction).to_string()).await;
                }
            }
            NetworkCommand::Connect(peer) => {
                let should_connect = {
                    let peer_manager_lock = peer_manager.lock().await.clone();
                    !peer_manager_lock.contains(&peer)
                };

                info!("Connected to: {}", &peer);

                if should_connect{
                        if let Ok(stream) = TcpStream::connect(&peer).await{
                            let (tx, rx) = mpsc::channel::<ConnectionResponse>(100);
                            {
                                let mut peer_manager_lock = peer_manager.lock().await;
                                peer_manager_lock.add(&peer.clone(), tx);
                            }
                            let (reader, writer) = stream.into_split();
                            let event_tx_clone = handler_tx.clone();
                                                
                            let new_peer_clone = peer.clone();
                            tokio::spawn(async move {
                                    connection_receiver(reader, &new_peer_clone, event_tx_clone)
                                    .await
                                    .expect("reader failed");
                                });

                            tokio::spawn(async move {
                                    connection_sender(writer, rx)
                                    .await
                                });

                            tokio::time::sleep(Duration::from_millis(200)).await;

                            {
                                let node_clone = node.read().await.clone();
                                let msg = ConnectionResponse::send(NetMessage::Verack(Verack::new(0,node_clone.version,node_clone.height)).to_string());
                                let peer_manager_lock = peer_manager.lock().await;
                                peer_manager_lock.send(&peer, msg).await.unwrap();
                            }
                        }
                        else{warn!("Failed to connect to: {}", peer)}
                                            
                    }                
            }
        }
    }
}

async fn start_network_handler(mut handler_rx: mpsc::Receiver<ConnectionEvent> ,peer_manager: Arc<Mutex<PeerManager>>, node: Arc<RwLock<Node>>, handler_tx: mpsc::Sender<ConnectionEvent>, miner_tx: mpsc::Sender<MiningCommand>) -> Result<()>{
        while let Some(event) = handler_rx.recv().await{
            let peer = event.peer;
            match event.connection_event_type{
                ConnectionEventType::Close => {
                    info!("Closed: {}", &peer);
                    let mut peer_manager_lock = peer_manager.lock().await;
                    peer_manager_lock.remove(&peer);
                }
                ConnectionEventType::Message(message) => {

                    let mut response = None;

                    info!("Received: {} from {}", &message, &peer);
                    match NetMessage::from_string(&message){
                        Ok(net_msg) => {
                            match net_msg{
                                NetMessage::Verack(verack) => {
                                    let node_clone = node.read().await.clone();
                                    if verack.index == 0{
                                        {
                                            peer_manager.lock().await.send(&peer,ConnectionResponse::send(NetMessage::Verack(Verack::new(1, node_clone.version, node_clone.height)).to_string())).await.unwrap();
                                        }
                                    }
                                    if verack.height > node_clone.height{
                                        let msg = NetMessage::GetBlocks(GetBlocks { start_height: node_clone.height + 1});
                                        {
                                            let peer_manager_lock = peer_manager.lock().await;
                                            peer_manager_lock.send(&peer, ConnectionResponse::send(msg.to_string())).await.unwrap();
                                        }
                                    }
                                }
                                
                                /*
                                NetMessage::GetHeaders(gh) => {
                                    let start_height = gh.start_height;
                                    
                                    let node_clone = node.read().await.clone();
                                    let headers: Vec<BlockHeader> = node_clone.headers[start_height..].to_vec();
                                    
                                    let msg = NetMessage::Headers(Headers::new(start_height, headers));
                                    
                                    response = Some(ConnectionResponse::send(msg.to_string()));
                                }

                                NetMessage::Headers(headers) => {
                                    {
                                    let mut node_lock = node.write().await;
                                    node_lock.update_headers(headers);
                                    }
                                }
                                */

                                NetMessage::GetInv(_) => {
                                    
                                    let node_clone = node.read().await.clone();
                                    let mempool = node_clone.mempool;
                                    let msg = NetMessage::Inv(Inv::new(mempool.get_inv()));
                                    
                                    response = Some(ConnectionResponse::send(msg.to_string()));
                                }

                                NetMessage::Inv(inv) => {
                                    let mut txwf = Vec::new();
                                    for tx in inv.mempool.clone(){
                                        if let Some(fee) = node.read().await.utxos.get_fee(tx.clone()) && node.read().await.utxos.validate_transaction(tx.clone()){
                                            txwf.push(TransactionWithFee::new(tx, fee));
                                        }
                                    }
                                    {
                                        node.write().await.mempool.update(txwf);
                                    }
                                }

                                NetMessage::GetPeerAddrs(_) => {
                                    
                                    let peer_manager_clone = peer_manager.lock().await.clone();
                                    let addresses: Vec<SocketAddr> = peer_manager_clone.peers.keys().copied().collect();
                                    let msg = NetMessage::PeerAddrs(PeerAddrs::new(addresses));
                                    
                                    response = Some(ConnectionResponse::send(msg.to_string()));
                                }
                                
                                NetMessage::PeerAddrs(peers) => {
                                    for new_peer in peers.addresses.iter(){

                                        {
                                        let should_connect = {
                                            let peer_manager_lock = peer_manager.lock().await.clone();
                                            !peer_manager_lock.contains(new_peer)
                                        };

                                        if should_connect{
                                            if let Ok(stream) = TcpStream::connect(&new_peer).await{
                                                let (tx, rx) = mpsc::channel::<ConnectionResponse>(100);
                                                {
                                                    let mut peer_manager_lock = peer_manager.lock().await;
                                                    peer_manager_lock.add(&new_peer.clone(), tx);
                                                }
                                                let (reader, writer) = stream.into_split();
                                                let event_tx_clone = handler_tx.clone();
                                                
                                                let new_peer_clone = new_peer.clone();
                                                tokio::spawn(async move {
                                                    connection_receiver(reader, &new_peer_clone, event_tx_clone)
                                                    .await
                                                    .expect("reader failed");
                                                });

                                                tokio::spawn(async move {
                                                    connection_sender(writer, rx)
                                                    .await
                                                });

                                                tokio::time::sleep(Duration::from_millis(100)).await;

                                                {
                                                let msg = ConnectionResponse::send(NetMessage::Verack(Verack::new(0,1,1,)).to_string());
                                                let peer_manager_lock = peer_manager.lock().await;
                                                peer_manager_lock.send(&new_peer, msg).await.unwrap();
                                                }
                                            }
                                            
                                        }
                                        }
                                    }
                                }

                                NetMessage::Ping(_) => {
                                    response = Some(ConnectionResponse::send(NetMessage::Pong(Pong{}).to_string()));
                                }
                                NetMessage::Pong(_) => {

                                }
                                
                                NetMessage::Transaction(transaction) => {
                                    if let Some(fee) = node.read().await.utxos.get_fee(transaction.clone()) && node.read().await.utxos.validate_transaction(transaction.clone()){
                                        let mut node_lock = node.write().await;
                                        if node_lock.mempool.add(transaction.clone(), fee){
                                            let peer_manager_lock = peer_manager.lock().await;
                                            peer_manager_lock.broadcast(NetMessage::Transaction(transaction).to_string()).await;
                                        }
                                }
                                }

                                NetMessage::NewBlock(new_block) => {
                                    let block = new_block.block;
                                    let is_new = {
                                        let mut  node_lock = node.write().await;
                                        node_lock.add_block(block.clone())
                                    };
                                    if is_new{
                                    {
                                        let peer_manager_lock = peer_manager.lock().await;
                                        peer_manager_lock.broadcast(NetMessage::NewBlock(NewBlock::new(block)).to_string()).await;
                                    }
                                    miner_tx.send(MiningCommand::UpdateBlock).await.unwrap();
                                    }
                                }

                                NetMessage::Blocks(blocks) => {
                                    {
                                    let mut node_lock = node.write().await;
                                    node_lock.update_blocks(blocks);
                                    }
                                    miner_tx.send(MiningCommand::UpdateBlock).await.unwrap();
                                }

                                NetMessage::GetBlocks(get_blocks) => {
                                    /*
                                    let mut start_height = get_blocks.start_height;
                                    let node_clone = node.read().await.clone();
                                    while start_height + 3 <= node_clone.height{
                                        let block_chain: Vec<Block> = node_clone.block_chain[start_height-1..start_height+10].to_vec();
                                    
                                        let msg = NetMessage::Blocks(Blocks::new(start_height, block_chain));
                                        peer_manager.lock().await.send(&peer, ConnectionResponse::send(msg.to_string())).await.unwrap();
                                        start_height += 3;
                                    }
                                    let block_chain: Vec<Block> = node_clone.block_chain[start_height-1..].to_vec();
                                    
                                        let msg = NetMessage::Blocks(Blocks::new(start_height, block_chain));
                                    
                                        response = Some(ConnectionResponse::send(msg.to_string()));
                                    */
                                    for block in &node.read().await.block_chain[get_blocks.start_height-1..]{
                                        let msg = NetMessage::NewBlock(NewBlock::new(block.clone()));
                                        peer_manager.lock().await.send(&peer, ConnectionResponse::send(msg.to_string())).await.unwrap();
                                        tokio::time::sleep(Duration::from_millis(100)).await;
                                    }

                                }   

                                }

                                if let Some(response) = response{
                                    peer_manager.lock().await.send(&peer, response).await?;
                                }
                            }
                            
                        Err(e) => {
                            error!("Error Deserializing message from: {} : {}", peer, e);
                        }
                    }
                }
            }

        }

        Ok(())
}

async fn connection_receiver(mut reader: OwnedReadHalf, peer: &SocketAddr, tx: mpsc::Sender<ConnectionEvent>) -> Result<()>{
    let mut buf =vec![0u8; 1024*1024];
    loop{
        let n = match reader.read(&mut buf).await{
            Ok(0) => {
                tx.send(ConnectionEvent::close(peer.clone())).await?;
                return Ok(())
            }
            Ok(n) => {
                n
            }
            Err(e) => {
                error!("Error reading from: {}", peer);
                tx.send(ConnectionEvent::close(peer.clone())).await?;
                return Err(e.into())
            }
        };
        let message = String::from_utf8_lossy(&buf[..n]).to_string();
        tx.send(ConnectionEvent::message(*peer, message)).await?;
    }
}

async fn connection_sender( mut writer: OwnedWriteHalf, mut rx: mpsc::Receiver<ConnectionResponse>){
    while let Some(response) = rx.recv().await{
        match response.connection_response_type{
            ConnectionResponseType::Close => {
                writer.shutdown().await.unwrap();
                return;
            }
            ConnectionResponseType::Send(message) => {
                info!("Sending: {}", message);
                writer.write_all(message.as_bytes()).await.unwrap();           
            }
        }
    }
}

pub async fn start_network_handling(addr: &String, node : Arc<RwLock<Node>>, miner_tx: mpsc::Sender<MiningCommand>, network_rx: mpsc::Receiver<NetworkCommand>) -> Result<()>{
    info!("Starting Network Handling ...");
    
    let listener = TcpListener::bind(addr).await?;
    
    info!("Listening on: {}", addr);

    let peer_manager = Arc::new(Mutex::new(PeerManager::new()));

    let (event_tx, rx) = mpsc::channel::<ConnectionEvent>(100);

    let peer_manager_clone = Arc::clone(&peer_manager);
    let event_tx_clone = event_tx.clone();
    let node_clone = Arc::clone(&node);
    let miner_tx_clone = miner_tx.clone();

    tokio::spawn(async move {
        start_network_handler(rx, peer_manager_clone, node_clone, event_tx_clone, miner_tx_clone)
        .await
        .expect("Network handler failed");
    });

    let peer_manager_clone = Arc::clone(&peer_manager); 
    let node_clone = Arc::clone(&node);
    let miner_tx_clone = miner_tx.clone();
    let handler_tx_clone = event_tx.clone();

    tokio::spawn(async move {
        network_command_handling(network_rx, peer_manager_clone, node_clone, miner_tx_clone, handler_tx_clone)
        .await
    });

    loop{
        let (stream, peer) = listener.accept().await?;

        let(tx, rx) = mpsc::channel::<ConnectionResponse>(100);
        
        {
        let mut peer_manager_lock = peer_manager.lock().await;
        peer_manager_lock.add(&peer, tx);
        }

        let (reader, writer) = stream.into_split();

        let event_tx_clone = event_tx.clone();
        tokio::spawn(async move {
            connection_receiver(reader, &peer, event_tx_clone)
            .await
            .expect("reader failed");
        });

        tokio::spawn(async move {
            connection_sender(writer, rx)
            .await
        });



    }
}

