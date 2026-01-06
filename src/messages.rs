use std::{cmp::Ordering, collections::{BinaryHeap, HashSet}, net::SocketAddr};

use serde::{Serialize, Deserialize};

use anyhow::Result;

use std::{hash::{Hash, Hasher}};

use crate::{
    miner::{Block},
    transactions::Transaction,
};

const TX_PER_BLOCK: usize = 10;

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
pub struct HeapSet<T>{
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
    pub fn get_vec(&self) -> Vec<T>{
        self.heap.clone().into_vec()
    }

    pub fn from_vec(v: Vec<T>) -> Self{
        Self {
            heap: BinaryHeap::from(v.clone()), 
            elements: HashSet::from_iter(v) 
        }
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
pub struct Mempool(pub HeapSet<TransactionWithFee>);


impl Mempool{
    pub fn new() -> Self{
        Self(HeapSet::new())
    }
    pub fn get_inv(self) -> Vec<Transaction>{
        self.0.get_vec().iter()
            .map(|txwf| txwf.transaction.clone())
            .collect()

    }

    pub fn add(&mut self, tx: Transaction, fee: usize) -> bool{
        self.0.push(TransactionWithFee::new(tx, fee))
    }
    pub fn update(&mut self, txs: Vec<TransactionWithFee>){
        txs.iter().for_each(|tx| 
            { let _ = self.0.push(tx.clone());
    });
    }

    pub fn get_next_transactions(&self) -> Vec<Transaction>{
        let mut mempool_clone = self.0.clone();
        let mut txs = Vec::new();
        for _ in 0..TX_PER_BLOCK{
            match mempool_clone.pop(){
                Some(tx) => {
                    txs.push(tx.transaction);
                }
                None => {
                    return txs
                }
            }
        }

        txs
    }

    pub fn size(&self) -> usize{
        self.0.elements.len()
    }

    pub fn remove(&mut self, transaction: Transaction){
        self.0.remove(TransactionWithFee::new(transaction, 0));
    }

    pub fn to_vec(&self) -> Vec<TransactionWithFee>{
        self.0.get_vec()
    }

    pub fn from_vec(txs: Vec<TransactionWithFee>) -> Result<Self>{
        Ok(Self(HeapSet::from_vec(txs)))
    }

    
}

impl Serialize for Mempool{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        self.to_vec().clone().serialize(serializer)
    }
}

impl <'de>Deserialize<'de> for Mempool{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
        let txs = Vec::<TransactionWithFee>::deserialize(deserializer)?;
        use serde::de::Error;
        Mempool::from_vec(txs).map_err(D::Error::custom)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransactionWithFee {
    pub transaction: Transaction,
    pub fee: usize,
}

impl TransactionWithFee{
    pub fn new(transaction: Transaction, fee: usize) -> Self{
        Self { 
            transaction, 
            fee 
        }
    }
}

// Equality: based on BOTH transaction and fee (for HashSet)
impl PartialEq for TransactionWithFee {
    fn eq(&self, other: &Self) -> bool {
        self.transaction == other.transaction
    }
}

impl Eq for TransactionWithFee {}

// Hash: based on BOTH transaction and fee (must match equality)
impl Hash for TransactionWithFee {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.transaction.hash(state);
    }
}

// Ordering: by fee ONLY (for heap priority)
impl Ord for TransactionWithFee {
    fn cmp(&self, other: &Self) -> Ordering {
        self.fee.cmp(&other.fee)
    }
}

impl PartialOrd for TransactionWithFee {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
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

/*
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

*/

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