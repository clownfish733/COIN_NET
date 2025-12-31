use crate::{miner::{Block, sha256}};

use std::{collections::HashMap, fs::File, path::Path};
use k256::{ecdsa::{Signature, SigningKey, VerifyingKey, signature::Signer}};
use k256::ecdsa::signature::Verifier;
use rand_core::OsRng;
use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};
use anyhow::{Ok, Result};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UTXOS(HashMap<([u8; 32], usize), TxOutput>);

impl UTXOS{
    pub fn new() -> Self{
        Self(HashMap::new())
    }

    fn get(&self, output_hash: [u8; 32], index: usize) -> Option<TxOutput>{
        self.0.get(&(output_hash, index)).cloned()
    }

    pub fn get_fee(&self, transaction: Transaction) -> Option<usize>{
        let mut total_in: usize = 0;
        for input in transaction.inputs.iter(){
            match self.get(input.prev, input.output_index){
                Some(a) => {total_in += a.value},
                None => return None
            }
        }
        let total_out: usize = transaction.outputs.iter()
            .map(|o| o.value).sum();

        if total_out > total_in{
            return None
        }else{
            Some(total_out - total_in)
        }

    }

    pub fn validate_transaction(&self, transaction: Transaction) -> bool{
        if self.get_fee(transaction.clone()) == None && !transaction.input_count == 0{
            return false
        }
        
        for input in transaction.inputs.clone(){
            let utxo = self.get(input.prev, input.output_index).unwrap();
            let script = Script::concat(input.script.clone(), utxo.script.clone());
            if script.validate_script(&transaction.clone(), input.output_index, &utxo){
                return false
            }
        }
        true
    }

    pub fn add_transaction(&mut self, transaction: Transaction){
        let hash = sha256(transaction.serialize());
        for (index, output) in transaction.outputs.iter().enumerate(){
            self.0.insert((hash, index), output.clone());
        }
    }

    pub fn add_block(&mut self, block: Block) -> bool{
        for tx in block.transactions.clone(){
            if !self.validate_transaction(tx){return false}
        }
        for tx in block.transactions{
            self.add_transaction(tx);
        }
        true
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

    fn store<P: AsRef<Path>>(&self, path: P) -> Result<()>{
        let file = File::create(path)?;
        serde_json::to_writer_pretty(&file, &self.to_hex_user())?;
        Ok(())
    }

    fn load<P: AsRef<Path>>(path: P) -> Result<Self>{
        let file = File::open(path)?;
        let hex_user: HexUser = serde_json::from_reader(file)?;
        Self::from_hex_user(hex_user)
    }

    fn sign(&self, message: String) -> Signature{
        self.private_key.sign(&Sha256::digest(message))
    }

    pub fn get_pub_key(&self) -> Vec<u8>{
        self.public_key.to_sec1_bytes().to_vec()
    }

    fn get_pub_key_hash(&self) -> Vec<u8>{
        sha256(String::from_utf8_lossy(&self.get_pub_key()).to_string()).to_vec()
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
    version: usize,
    pub input_count: usize,
    inputs: Vec<TxInput>,
    output_count: usize,
    outputs: Vec<TxOutput>,
}

impl Transaction{
    pub fn serialize(&self) -> String{
        serde_json::to_string(self).unwrap()
    }

    pub fn reward(reward: usize, pubkey: Vec<u8>, version: usize) -> Self{
        Self { 
            version, 
            input_count: 0, 
            inputs: Vec::new(), 
            output_count: 1,
            outputs:vec![TxOutput{
                value: reward,
                script: Script::P2PKHOutput(sha256(String::from_utf8_lossy(&pubkey).to_string()).to_vec())
            }]
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
struct TxInput{
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
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests{
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
}