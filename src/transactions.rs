use crate::{miner::{Block, sha256, get_timestamp}};

use std::{collections::HashMap};
use k256::{ecdsa::{Signature, SigningKey, VerifyingKey, signature::Signer}};
use k256::ecdsa::signature::Verifier;
use log::{info, warn};
use rand_core::OsRng;
use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize, de::{self, Visitor}};
use anyhow::{Result};

pub fn is_coinbase(transaction: &Transaction) -> bool{
    transaction.input_count == 0
}

#[derive(Clone, Debug)]
pub struct UTXOS(HashMap<([u8; 32], usize), TxOutput>);

impl UTXOS{
    pub fn new() -> Self{
        Self(HashMap::new())
    }

    pub fn add(&mut self, hash: [u8; 32], index: usize, output: TxOutput){
        self.0.insert((hash, index), output);
    }

    fn get(&self, output_hash: [u8; 32], index: usize) -> Option<TxOutput>{
        self.0.get(&(output_hash, index)).cloned()
    }

    pub fn get_fee(&self, transaction: Transaction) -> Option<usize>{
        let mut total_in: usize = 0;
        for input in transaction.inputs.iter(){
            match self.get(input.prev, input.output_index){
                Some(a) => {
                    total_in += a.value;
                },
                None => {
                    println!("Not in utxos: {}, {}", hex::encode(input.prev), input.output_index);
                    return None
                }
            }
        }
        let total_out: usize = transaction.outputs.iter()
            .map(|o| o.value)
            .sum();

        if total_out > total_in{
            return None
        }else{
            Some(total_in - total_out)
        }

    }

    pub fn validate_transaction(&self, transaction: Transaction) -> bool{
        if is_coinbase(&transaction){return true}

        if self.get_fee(transaction.clone()) == None{
            warn!("NO fee for: {:?}", transaction);
            return false
        }
        
        for input in transaction.inputs.clone(){
            let utxo = self.get(input.prev, input.output_index).unwrap();
            let script = Script::concat(input.script.clone(), utxo.script.clone());
            if script.validate_script(&transaction.clone(), input.output_index, &utxo){
                warn!("Invalid script");
                return false
            }
        }
        true
    }


    pub fn add_transaction(&mut self, transaction: Transaction){
        let hash = sha256(transaction.serialize().clone());
        for input in transaction.inputs{
            self.0.remove(&(input.prev, input.output_index));
        }
        for (index, output) in transaction.outputs.iter().enumerate(){
            self.0.insert((hash, index), output.clone());
        }
        
    }
    pub fn validate_block(&self, block: Block) -> bool{
        for tx in block.transactions.clone(){
            if !self.validate_transaction(tx){
                warn!("Invalid block"); return false
            }
        }
        true
    }

    pub fn add_block(&mut self, block: Block) -> bool{
        for tx in block.transactions{
            self.add_transaction(tx);
        }
        true
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Wallet{
    pub value: usize,
    utxos: UTXOS,
    pub pub_key: Vec<u8>,
}

impl Wallet{
    pub fn new(pub_key: Vec<u8>) -> Self{
        Self { 
            value: 0, 
            utxos: UTXOS::new(),
            pub_key: pub_key
        }
    }

    pub fn update(&mut self, block: Block){
        for tx in block.transactions{
            for input in tx.clone().inputs{
                if let Some(output) = self.utxos.get(input.prev, input.output_index){
                    self.value -= output.value;
                    self.utxos.0.remove(&(input.prev, input.output_index));
                }
            }
            let pk_hash = sha256(hex::encode(self.pub_key.clone())).to_vec();
            let tx_hash = sha256(tx.clone().serialize());
            for (index, output) in tx.outputs.iter().cloned().enumerate(){
                if let Some(hash) = output.clone().script.P2PKHOutput_pubkey_hash() && hash == pk_hash {
                    self.utxos.add(tx_hash, index, output.clone());
                    self.value += output.value;
                }
            }
        }
    }

    pub fn get_inputs(&self, value: usize) -> Option<(Vec<(([u8; 32], usize), TxOutput)>, usize)>{
        let mut cur_val: usize = 0;
        let mut utxo_clone = self.utxos.clone();
        let mut inputs = Vec::new();
        while cur_val <= value{
            if let Some(key) = utxo_clone.0.keys().next().cloned() {
                let output = utxo_clone.0.remove(&key).unwrap();
                cur_val += output.value;
                println!("get_inputs: {}", hex::encode(key.0));
                inputs.push((key, output));
            } else{
                return None
            }
        }
        Some((inputs, cur_val))
    }
}

impl Serialize for UTXOS{
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let map: HashMap<String, &TxOutput> = self.0
            .iter()
            .map(|((hash, idx), output)| {
                let key = format!("{}:{}", hex::encode(hash), idx);
                (key, output)
            })
            .collect();
        map.serialize(serializer)
    }
}

impl <'de>Deserialize<'de> for UTXOS{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
        struct UTXOSVisitor;
        
        impl<'de>Visitor<'de> for UTXOSVisitor{
            type Value = UTXOS;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("A map of string key in format 'hash:index")
            }
            
            fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
                where
                    A: serde::de::MapAccess<'de>, {
                let mut utxos = HashMap::new();
                while let Some((key, value)) = map.next_entry::<String, TxOutput>()? {

                    let parts: Vec<&str> = key.split(":").collect();
                    if parts.len() != 2{
                        return Err(de::Error::custom("key must be in format 'hash:index"));
                    }

                    let hash_bytes = hex::decode(parts[0])
                        .map_err(|e| de::Error::custom(format!("invalid hex: {}", e)))?;

                    if hash_bytes.len() != 32{
                        return Err(de::Error::custom("hash must be 32 bytes"));
                    }

                    let mut hash = [0u8; 32];
                    hash.copy_from_slice(&hash_bytes);

                    let idx = parts[1].parse::<usize>()
                        .map_err(|e| de::Error::custom(format!("invalid index: {}", e)))?;

                    utxos.insert((hash, idx), value);
                }

                Ok(UTXOS(utxos))
            }
        }

        deserializer.deserialize_map(UTXOSVisitor)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct HexUser{
    public_key: String,
    private_key: String,
}

impl HexUser{
    fn new(public_key: VerifyingKey, private_key: SigningKey) -> Self{
        let pk = hex::encode(public_key.to_sec1_bytes());
        let sk = hex::encode(private_key.to_bytes());
        Self { 
            public_key: pk, 
            private_key: sk 
        }
    }
}

#[derive(Clone, Debug)]
pub struct User{
    public_key: VerifyingKey,
    private_key: SigningKey,
}

impl User{
    pub fn new() -> Self{
        let sk = SigningKey::random(&mut OsRng);
        let pk = VerifyingKey::from(&sk);
        Self { 
            public_key: pk, 
            private_key: sk 
        }
    }

    fn to_hex_user(&self) -> HexUser{
        HexUser::new(self.public_key.clone(), self.private_key.clone())
    }

    fn from_hex_user(user: HexUser) -> Result<Self>{
        let pk = VerifyingKey::from_sec1_bytes(&hex::decode(user.public_key)?)?;
        let sk = SigningKey::from_slice(&hex::decode(user.private_key)?)?;
        Ok(Self { 
            public_key: pk, 
            private_key: sk 
        })
    }

    fn sign(&self, message: String) -> Signature{
        self.private_key.sign(&Sha256::digest(message))
    }

    pub fn get_pub_key(&self) -> Vec<u8>{
        self.public_key.to_sec1_bytes().to_vec()
    }

    fn get_pub_key_hash(&self) -> Vec<u8>{
        sha256(hex::encode(&self.get_pub_key())).to_vec()
    }
}

fn verify_sig(public_key: VerifyingKey, message_hash: [u8; 32], signature: Signature) -> bool{
    public_key.verify(&message_hash, &signature).is_ok()
        
}

impl Serialize for User{
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        self.to_hex_user().serialize(serializer)
    }
}

impl <'de>Deserialize<'de>for User{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
        let user = HexUser::deserialize(deserializer)?;
        use serde::de::Error;
        User::from_hex_user(user).map_err(D::Error::custom)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Transaction{
    timestamp: usize,
    version: usize,
    pub input_count: usize,
    pub inputs: Vec<TxInput>,
    output_count: usize,
    pub outputs: Vec<TxOutput>,
}

impl Transaction{
    pub fn serialize(&self) -> String{
        serde_json::to_string(self).unwrap()
    }

    pub fn reward(reward: usize, pubkey: Vec<u8>, version: usize) -> Self{
        Self { 
            timestamp: get_timestamp(),
            version, 
            input_count: 0, 
            inputs: Vec::new(), 
            output_count: 1,
            outputs:vec![TxOutput{
                value: reward,
                script: Script::P2PKHOutput(sha256(hex::encode(&pubkey)).to_vec())
            }]
        }
    }

    pub fn new(version: usize, user: User, inputs: Vec<(([u8; 32], usize), TxOutput)>, outputs: Vec<(String, usize)>) -> Self{
        let mut transaction = Transaction{
            timestamp: get_timestamp(),
            version,
            input_count: inputs.len(),
            inputs: inputs.iter().map(|((hash, index), _ouput)| TxInput{
                    prev: hash.clone(), 
                    output_index: index.clone(), 
                    script: Script::empty()
                })
                .collect(),
            output_count: outputs.len(),
            outputs: outputs.iter().map(|(pub_key, amount)| TxOutput{
                value: amount.clone(),
                script: Script::P2PKHOutput(sha256(pub_key.clone()).to_vec())
            })
            .collect(),
        };
        for (index,(_, output)) in inputs.iter().enumerate(){
            let sig = user.sign(hex::encode(compute_sig_hash(transaction.clone(), index, &output))).to_vec();
            let pubkey = user.get_pub_key();
            transaction.inputs[index].script = Script::P2PKHInput(sig, pubkey);
        }
        transaction
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TxInput{
    prev: [u8; 32],
    output_index: usize,
    script: Script
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TxOutput{
    value: usize,
    script: Script,
}

fn compute_sig_hash(tx: Transaction, input_index: usize, utxo: &TxOutput) -> [u8; 32]{
    let mut modified_tx = tx.clone();
    for input in &mut modified_tx.inputs{
        input.script = Script::empty();
    }
    
    modified_tx.inputs[input_index].script = utxo.script.clone();
    sha256(modified_tx.serialize())

}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Script (Vec<OpCode>);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum OpCode{
    //data
    PUSHBYTES(Vec<u8>),
    //stack
    DUP,
    //DROP,
    //SWAP,
    
    //crypto
    //HASH160,
    SHA256,
    CHECKSIG,
    //CHECKMULTISIG,

    //comparison
    //EQUAL,
    EQUALVERIFY,
    //VERIFY,

    //flow
    //IF,
    //ELSE,
    //ENDIF,

    //constants
    //OP0,
    //OP1,
    //OP2,
    //OP3,
}

impl Script{
    pub fn empty() -> Self{
        Self(vec![])
    }
    pub fn validate_script(&self, tx: &Transaction, input_index: usize, utxo: &TxOutput) -> bool{
        let mut stack: Vec<Vec<u8>> = Vec::new();
        for op in self.0.iter(){
            match op{
                OpCode::PUSHBYTES(data) => {
                    stack.push(data.clone());
                }
                OpCode::DUP => {
                    if let Some(top) = stack.last(){
                        stack.push(top.clone());
                    }
                    else{
                        return false
                    }
                }
                OpCode::SHA256 => {
                    if let Some(top) = stack.pop(){
                        let x = sha256(String::from_utf8_lossy(&top.clone()).to_string()).to_vec();
                        stack.push(x.clone());
                    }
                }
                OpCode::EQUALVERIFY => {
                    if let Some(x1) = stack.pop() && let Some(x2) = stack.pop(){
                        if x1 != x2{
                            return false
                        }
                    }
                    else{return false}
                }
                OpCode::CHECKSIG => {
                    let sighash = compute_sig_hash(tx.clone(), input_index, utxo);
                    let pk = match stack.pop(){
                        Some(pk) => pk,
                        None => return false
                    };
                    let sig = match stack.pop(){
                        Some(sig) => sig,
                        None => return false
                    };

                    let public_key = VerifyingKey::from_sec1_bytes(&pk).unwrap();
                    let signature= Signature::from_slice(&sig).unwrap();
                    match verify_sig(public_key, sighash, signature){
                        true => stack.push(vec![1]),
                        false => return false
                    }
                }
            }
        }
        match stack.last() {
            Some(top) => top.iter().any(|&b| b != 0),
            None => false
        }
    }

    fn concat(s1: Script, s2: Script) -> Self{
        let mut s = s1.0;
        s.extend(s2.0);
        Self(s)
    }

    #[allow(non_snake_case)]
    pub fn P2PKHInput(sig: Vec<u8>, pubkey: Vec<u8>) -> Self{
        Self(vec![
            OpCode::PUSHBYTES(sig),
            OpCode::PUSHBYTES(pubkey)
        ])
    }

    #[allow(non_snake_case)]
    pub fn P2PKHOutput(pubkey_hash: Vec<u8>) -> Self{
        Self(vec![
            OpCode::DUP,
            OpCode::SHA256,
            OpCode::PUSHBYTES(pubkey_hash),
            OpCode::EQUALVERIFY,
            OpCode::CHECKSIG,
        ])
    }

    pub fn P2PKHOutput_pubkey_hash(&self) -> Option<Vec<u8>>{
        match self.0.get(2){
            Some(OpCode::PUSHBYTES(hash)) => {
                Some(hash.clone())
            }
            _ => {
                None
            }
        }
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests{
    use crate::messages::{Mempool, TransactionWithFee};

    use super::*;

    #[test]
    fn script_test() {
        //A -> B -> C

        
        let _A = User::new();
        let B = User::new();
        let C = User::new();

        let utxo = TxOutput{
            value: 10,
            script: Script(vec![
                OpCode::DUP,
                OpCode::SHA256,
                OpCode::PUSHBYTES(B.get_pub_key_hash()),
                OpCode::EQUALVERIFY,
                OpCode::CHECKSIG,
            ])
        };

        let mut tx = Transaction{
            timestamp: get_timestamp(),
            version: 10,
            input_count: 1,
            inputs: vec![TxInput{
                prev: sha256("Hello World".to_string()),
                output_index: 1,
                script: utxo.script.clone(),
            }],
            output_count: 1,
            outputs: vec![TxOutput{
                value: 10,
                script: Script(vec![
                    OpCode::DUP,
                    OpCode::SHA256,
                    OpCode::PUSHBYTES(C.get_pub_key_hash()),
                    OpCode::EQUALVERIFY,
                    OpCode::CHECKSIG,
                ])
            }]
        };
        let sig = B.sign(tx.serialize()).to_vec();
        let unlocking_script = Script(vec![
            OpCode::PUSHBYTES(sig),
            OpCode::PUSHBYTES(B.get_pub_key()),
        ]);
        tx.inputs[0].script = unlocking_script.clone();

        let script = Script::concat(unlocking_script.clone(), utxo.script.clone());
        assert_eq!(script.validate_script(&tx, 0, &utxo), true)

    }

    fn display_wallet(wallet: &Wallet){
        println!("Wallet");
        println!("value: {}", wallet.value);
        for (key, utxo) in wallet.utxos.0.iter(){
            println!("{},{} : {} for: {}", hex::encode(key.0), key.1, &utxo.value, hex::encode(utxo.script.P2PKHOutput_pubkey_hash().unwrap()));
        }
    }

    fn display_utxos(utxos: &UTXOS){
        println!("UTXOS");
        for (key, utxo) in utxos.0.iter(){
            println!("{},{} : {} for: {}", hex::encode(key.0), key.1, &utxo.value, hex::encode(utxo.script.P2PKHOutput_pubkey_hash().unwrap()));
        }
    }

    fn display_mempool(mempool: &Mempool){
        println!("Mempool");
        let mut m =  mempool.0.clone();
        while let Some(txwf) = m.pop(){
            println!("Fee: {}", txwf.fee);
            println!("Tx: {}", hex::encode(txwf.transaction.serialize()));
        }
    }

    #[test]
    fn test_wallet(){
        let mut utxos = UTXOS::new();
        let mut mempool = Mempool::new();
        let user = User::new();
        let user2 = User::new();
        println!("User pubkey hash: {}", hex::encode(user.get_pub_key_hash()));
        println!("User2 pubkey hash: {}", hex::encode(user2.get_pub_key_hash()));
        let new_tx = Transaction::reward(10, user.get_pub_key(), 1);
        let mut wallet = Wallet::new(user.get_pub_key());
        let block1 = Block::new(
        vec![new_tx.clone()],
        sha256("0000".to_string()),
        3,
        1,
        1
        );
        println!("\nAdding first block\n");
        wallet.update(block1.clone());
        display_wallet(&wallet);
        utxos.add_block(block1.clone());
        display_utxos(&utxos);
        println!("tx: {}", hex::encode(sha256(new_tx.clone().serialize())));
        for tx in block1.clone().transactions{
            if !is_coinbase(&tx){
                mempool.remove(tx.clone());
            }
        }
        display_mempool(&mempool);



        let total: usize = 5;
        let fee: usize = 1;
        let (inputs, value) = wallet.get_inputs(total).unwrap();
        let mut outputs = vec![(hex::encode(user2.get_pub_key()), total - fee)];
        outputs.push((hex::encode(wallet.pub_key.clone()), value - total));
        let new_tx = Transaction::new(
                1,
                user.clone(),
                inputs,
                outputs,
            );
        mempool.add(new_tx.clone(), utxos.get_fee(new_tx.clone()).unwrap());
        
        let mut new_txs = mempool.get_next_transactions();
        new_txs.push(Transaction::reward(10, user2.get_pub_key(), 1));
        let block2 = Block::new(
            new_txs.clone(),
            sha256(block1.to_string()),
            3,
            1,
            2,
        );
        println!("\nAdding second block\n");
        for tx in block2.clone().transactions{
            if !is_coinbase(&tx){
                if let Some(fee) = utxos.get_fee(tx.clone()){
                    mempool.remove(tx.clone());
                }
                else{
                    assert_eq!(1,3);
                    return
                }
            }
        }
        wallet.update(block2.clone());
        display_wallet(&wallet);
        utxos.add_block(block2.clone());
        display_utxos(&utxos);
        for tx in new_txs.clone(){
            for input in tx.inputs{
                println!("{}, {}" , hex::encode(input.prev), input.output_index);
            }
        }
        display_mempool(&mempool);


        let total: usize = 4;
        let fee: usize = 1;
        let (inputs, value) = wallet.get_inputs(total).unwrap();
    
        let mut outputs = vec![(hex::encode(user2.get_pub_key()), total - fee)];
        if value > total{
            outputs.push((hex::encode(wallet.pub_key.clone()), value - total));
        }

        let new_tx = Transaction::new(
                1,
                user.clone(),
                inputs,
                outputs,
            );
        mempool.add(new_tx.clone(), utxos.get_fee(new_tx.clone()).unwrap());
        let mut new_txs = mempool.get_next_transactions();
        new_txs.push(Transaction::reward(10, user.get_pub_key(), 1));
        let block3 = Block::new(
            new_txs,
            sha256(block2.to_string()),
            3,
            1,
            3,
        );

        for tx in block3.clone().transactions{
            if !is_coinbase(&tx){
                if let Some(fee) = utxos.get_fee(tx.clone()){
                    mempool.remove(tx.clone());
                }
                else{
                    assert_eq!(1,3);
                    return
                }
            }
        }
        println!("\nAdding third block\n");
        wallet.update(block3.clone());
        display_wallet(&wallet);
        utxos.add_block(block3.clone());
        display_utxos(&utxos);
        display_mempool(&mempool);
        
    }

}