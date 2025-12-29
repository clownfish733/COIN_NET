use crate::miner::HashDigest;

pub struct Transaction{
    version: usize,
    input_count: usize,
    inputs: Vec<TxInput>,
    output_count: usize,
    outputs: Vec<TxOutput>,
}

struct TxInput{
    prev: HashDigest,
    output_index: usize,
    script: Script
}

struct TxOutput{
    value: usize,
    script: usize,
}


struct Script { 
    script: Vec<OpCode>
}

pub enum OpCode{
    //data
    PUSHBYTES(Vec<u8>),
    //stack
    DUP,
    DROP,
    SWAP,
    
    //crypto
    HASH160,
    SHA256,
    CHECKSIG,
    CHECKMULTISIG,

    //comparison
    EQUAL,
    EQUALVERIFY,
    VERIFY,

    //flow
    IF,
    ELSE,
    ENDIF,

    //constants
    OP0,
    OP1,
    OP2,
    OP3,
}

impl Script{
    pub fn validate_script(&self) -> bool{

    }
}