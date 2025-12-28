use std::{cmp::Ordering, collections::{BinaryHeap, HashSet}, net::SocketAddr};

use serde::{Serialize, Deserialize};

use std::{hash::{Hash, Hasher}};

use crate::miner::{BlockHeader, Block};

const tx_per_block: usize = 10;

pub type Address = [u8; 20];

#[derive(Serialize, Deserialize, Debug)]
pub struct Verack{
    pub index: usize,
    version: usize,
    pub height: usize,
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

#[derive(Clone, Debug)]
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

    pub fn remove(&mut self, item: T){
        if self.elements.remove(&item){

            let vec: Vec<T> = self.heap.drain()
                .filter(|x| *x != item)
                .collect();
            self.heap = BinaryHeap::from(vec);
        }
    }
}

#[derive(Clone, Debug)]
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

    pub fn get_next_transactions(&self) -> Vec<Transaction>{
        let mut transactions = Vec::new();
        let mut mempool_clone = self.mempool.clone();
        for _ in 0..tx_per_block{
            if let Some(item) = mempool_clone.pop() {
                transactions.push(item)
            }
            else{
                break;
            }
        }

        transactions
    }

    pub fn remove(&mut self, transaction: Transaction){
        self.mempool.remove(transaction);
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


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NewBlock{
    pub block: Block
}
impl NewBlock{
    pub fn new(block: Block) -> Self{
        Self { block }
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
    pub fn to_string(&self) -> String{
        serde_json::to_string(&self).unwrap().to_string()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetInv;

impl GetInv{
    pub fn new() -> Self{
        Self{ }
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
pub struct GetHeaders{
    pub start_height: usize,
}

impl GetHeaders{
    pub fn new(start_height: usize) -> Self{
        Self { start_height }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Headers{
    pub start_height: usize,
    pub headers: Vec<BlockHeader>,
}

impl Headers{
    pub fn new(start_height: usize, headers: Vec<BlockHeader>) -> Self{
        Self { 
            start_height,
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
pub struct Ping;

#[derive(Serialize, Deserialize, Debug)]
pub struct Pong;

#[derive(Serialize, Deserialize, Debug)]
pub struct GetBlocks{
    pub start_height: usize,
}

impl GetBlocks{
    pub fn new(start_height: usize) -> Self{
        Self{
            start_height
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Blocks{
    pub start_height: usize,
    pub blockchain: Vec<Block>,
}


impl Blocks{
    pub fn new(start_height: usize, blockchain: Vec<Block>) -> Self{
        Self { start_height, blockchain }
    }
}