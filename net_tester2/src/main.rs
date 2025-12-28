use std::{cmp::Ordering, collections::{BinaryHeap, HashSet}, net::SocketAddr};

use serde::{Serialize, Deserialize};

use std::{hash::{Hash, Hasher}};

use tokio::{io::{AsyncReadExt, AsyncWriteExt, AsyncRead}, net::{TcpListener, TcpStream}};

use anyhow::Result;

#[derive(Serialize, Deserialize, Debug)]
pub struct Verack{
    pub index: usize,
    version: usize,
    height: usize,
}

impl Verack{
    pub fn new(index: usize, version: usize, height: usize) -> Self{
        Self{
            index,
            version,
            height,
        }
    }
}

#[derive(Clone)]
struct HeapSet<T>{
    heap: BinaryHeap<T>,
    elements: HashSet<T>,
}

impl<T: Ord + Clone + Hash> HeapSet<T>{
    fn new() -> Self{
        Self { 
            heap: BinaryHeap::new(), 
            elements: HashSet::new() 
        }
    }
    pub fn push(&mut self, item: T) -> bool{
        if self.elements.insert(item.clone()) {
            self.heap.push(item);
            return true
        }
        false
    }
    pub fn pop(&mut self) -> Option<T>{
        if let Some(item) = self.heap.pop(){
            self.elements.remove(&item);
            Some(item)
        }  else {
            None
        }
    }
    pub fn get_vec(self) -> Vec<T>{
        self.heap.into_vec()
    }
}

type Address = [u8; 20];

#[derive(Clone)]
pub struct Mempool{
    mempool: HeapSet<Transaction>
}

impl Mempool{
    pub fn new() -> Self{
        Self {
             mempool: HeapSet::new()
        }
    }
    pub fn get_inv(self) -> Vec<Transaction>{
        self.mempool.get_vec()

    }

    pub fn add(&mut self, tx: Transaction) -> bool{
        self.mempool.push(tx)
    }
    pub fn update(&mut self, txs: Vec<Transaction>){
        txs.iter().for_each(|tx| 
            { let _ = self.mempool.push(tx.clone());
    });
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq)]
pub struct Transaction{
    from: Address,
    to: Address,
    amount: usize,
    fee: usize,
}

impl PartialEq for Transaction {
    fn eq(&self, other: &Self) -> bool{
        self.fee == other.fee
    }
}

impl Ord for Transaction{
    fn cmp(&self, other: &Self) -> Ordering{
        self.fee.cmp(&other.fee)
    }
}

impl PartialOrd for Transaction{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Transaction{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.fee.hash(state)
    }
}

impl Transaction {
    pub fn new(from: Address, to: Address, amount: usize, fee: usize) -> Self{
        Self { 
            from, 
            to, 
            amount, 
            fee 
        }
    }
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetInv;

impl GetInv{
    pub fn new() -> Self{
        Self{
            
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Inv{
    pub mempool: Vec<Transaction>,
}

impl Inv{
    pub fn new(mempool: Vec<Transaction>) -> Self{
        Self { 
            mempool 
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetHeaders;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockHeader;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Headers{
    pub height: usize,
    pub headers: Vec<BlockHeader>,
}

impl Headers{
    pub fn new(height: usize, headers: Vec<BlockHeader>) -> Self{
        Self { 
            height,
            headers,

        }
    }
}


#[derive(Serialize, Deserialize, Debug)]
pub struct GetPeerAddrs;

#[derive(Serialize, Deserialize, Debug)]
pub struct PeerAddrs{
    pub addresses: Vec<SocketAddr>
}

impl PeerAddrs{
    pub fn new(addresses: Vec<SocketAddr>) -> Self{
        Self{
            addresses,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum NetMessage{
    Verack(Verack),
    Transaction(Transaction),
    GetInv(GetInv),
    Inv(Inv),
    GetHeaders(GetHeaders),
    Headers(Headers),
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


#[derive(Serialize, Deserialize, Debug)]
pub struct Ping;

#[derive(Serialize, Deserialize, Debug)]
pub struct Pong;

#[tokio::main]
async fn main() -> Result<()>{
    let listener = TcpListener::bind("0.0.0.0:8081").await.unwrap();

    let (mut stream, _peer) = listener.accept().await.unwrap();

    let msg = NetMessage::Verack(Verack::new(1,1,1));

    let mut buf = [0u8; 1024];

    let n = stream.read(&mut buf).await.unwrap(); 
    let string = String::from_utf8_lossy(&buf[..n]).to_string();
    println!("{}", string);
    let response = NetMessage::from_string(&string)?;
    println!("{:?}", response);

    stream.write_all(msg.to_string().as_bytes()).await.unwrap();

    /*
    let from: Address = [1u8; 20]; 
    let to: Address = [3u8; 20]; 
    let msg = NetMessage::Transaction(Transaction::new(from, to, 400, 10));
    stream.write_all(msg.to_string().as_bytes()).await.unwrap();

    let mut buf = [0u8; 1024];

    let n = stream.read(&mut buf).await.unwrap(); 
    let response = NetMessage::from_string(&String::from_utf8_lossy(&buf[..n]).to_string()).unwrap();
    println!("{:?}", response);


    let msg = NetMessage::GetInv(GetInv::new());
    stream.write_all(msg.to_string().as_bytes()).await.unwrap();

    let mut buf = [0u8; 1024];

    let n = stream.read(&mut buf).await.unwrap(); 
    let response = NetMessage::from_string(&String::from_utf8_lossy(&buf[..n]).to_string()).unwrap();
    println!("{:?}", response);
    */

    stream.shutdown().await.unwrap();
    Ok(())
}