use std::{fs::File, path::Path};
use k256::{ecdsa::{Signature, SigningKey, VerifyingKey, signature::Signer}};
use k256::ecdsa::signature::Verifier;
use rand_core::OsRng;
use sha2::{Digest, Sha256};
use serde::{Serialize, Deserialize};
use anyhow::{Ok, Result, anyhow};
use std::env;

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


pub struct User{
    public_key: VerifyingKey,
    private_key: SigningKey,
}

impl User{
    fn new() -> Self{
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

    fn from_hex_user(user: HexUser) -> Self{
        let pk = VerifyingKey::from_sec1_bytes(&hex::decode(user.public_key).unwrap()).unwrap();
        let sk = SigningKey::from_slice(&hex::decode(user.private_key).unwrap()).unwrap();
        Self { 
            public_key: pk, 
            private_key: sk 
        }
    }

    fn store<P: AsRef<Path>>(&self, path: P) -> Result<()>{
        let file = File::create(path)?;
        serde_json::to_writer_pretty(&file, &self.to_hex_user())?;
        Ok(())
    }

    fn load<P: AsRef<Path>>(path: P) -> Result<Self>{
        let file = File::open(path)?;
        let hex_user: HexUser = serde_json::from_reader(file)?;
        Ok(Self::from_hex_user(hex_user))
    }

    fn sign(&self, message: String) -> Signature{
        self.private_key.sign(&Sha256::digest(message))
    }
}

fn verify_sig(public_key: VerifyingKey, message_hash: [u8; 32], signature: Signature) -> bool{
    public_key.verify(&message_hash, &signature).is_ok()
        
}



fn main() -> Result<()>{
    let path = "delete.json";
    
    let u1 = match env::args().nth(1).as_deref() {
        Some("new") => User::new(),
        Some("load") => User::load(&path)?,
        Some(arg) => return Err(anyhow!("Invalid argument '{}': expected 'new' or 'load'", arg)),
        None => return Err(anyhow!("Missing argument: expected 'new' or 'load'")),
    };
    

    
    let msg = "Hello";
    let sig = u1.sign(msg.to_string());
    
    u1.store(path)?;


    let is_valid = verify_sig(u1.public_key, Sha256::digest(msg).into(), sig);
    println!("{}", is_valid);
    
    Ok(())
}