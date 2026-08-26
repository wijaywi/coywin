use sled::Db;
use serde::{Deserialize, Serialize};
use pqcrypto_traits::sign::{PublicKey as PublicKeyTrait, DetachedSignature as DetachedSignatureTrait};
use pqcrypto_dilithium::dilithium5::{PublicKey, DetachedSignature, verify_detached_signature as dilithium_verify};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GiftTransaction {
    pub art_hash: [u8; 32],       // The block hash representing the Art
    pub current_owner: Vec<u8>,   // ML-DSA-87 Public Key bytes (2592 bytes)
    pub receiver: Vec<u8>,        // ML-DSA-87 Public Key bytes
    pub signature: Vec<u8>,       // ML-DSA-87 Signature bytes (4627 bytes)
}

pub struct CoywinLedger {
    db: Db,
}

impl CoywinLedger {
    pub fn new(path: &str) -> Result<Self, sled::Error> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    /// Validates and applies a Gift Transaction
    pub fn apply_gift(&self, tx: &GiftTransaction) -> Result<(), &'static str> {
        // 1. Check if the current owner actually owns the art
        let stored_owner = self.db.get(&tx.art_hash).map_err(|_| "DB Read Error")?;
        
        if let Some(owner_bytes) = stored_owner {
            if owner_bytes.as_ref() != tx.current_owner.as_slice() {
                return Err("Current owner does not match ledger state. Theft attempt denied.");
            }
        }

        // 2. Validate the ML-DSA-87 signature
        // The message is the concatenation of the art_hash and the receiver's public key
        let mut message = Vec::new();
        message.extend_from_slice(&tx.art_hash);
        message.extend_from_slice(&tx.receiver);

        let pk = <PublicKey as PublicKeyTrait>::from_bytes(&tx.current_owner).map_err(|_| "Invalid Public Key")?;
        let sig = <DetachedSignature as DetachedSignatureTrait>::from_bytes(&tx.signature).map_err(|_| "Invalid Signature bytes")?;

        if dilithium_verify(&sig, &message, &pk).is_err() {
            return Err("ML-DSA-87 Quantum Signature Verification Failed.");
        }

        // 3. Update the Ledger
        self.db.insert(&tx.art_hash, tx.receiver.clone()).map_err(|_| "DB Write Error")?;
        self.db.flush().map_err(|_| "DB Flush Error")?;

        Ok(())
    }

    pub fn get_owner(&self, art_hash: &[u8; 32]) -> Option<Vec<u8>> {
        self.db.get(art_hash).ok().flatten().map(|v| v.to_vec())
    }
}
