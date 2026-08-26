use sled::Db;
use serde::{Deserialize, Serialize};
use pqcrypto_traits::sign::{PublicKey as PublicKeyTrait, DetachedSignature as DetachedSignatureTrait};
use pqcrypto_dilithium::dilithium5::{PublicKey, DetachedSignature, verify_detached_signature as dilithium_verify};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GiftTransaction {
    pub art_hash: [u8; 32],       // The block hash representing the Art
    pub current_owner: Vec<u8>,   // ML-DSA-87 Public Key bytes (2592 bytes)
    pub receiver: Vec<u8>,        // ML-DSA-87 Public Key bytes
    pub nonce: u64,               // Replay protection
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

    pub fn validate_gift(&self, tx: &GiftTransaction) -> Result<(), &'static str> {
        // 1. Check if the current owner actually owns the art
        let stored_owner = self.db.get(&tx.art_hash).map_err(|_| "DB Read Error")?;
        if let Some(owner_bytes) = stored_owner {
            if owner_bytes.as_ref() != tx.current_owner.as_slice() {
                return Err("Current owner does not match ledger state. Theft attempt denied.");
            }
        } else {
            // Art does not exist yet (genesis issue?) Or maybe we allow unowned art for some reason?
            // Wait, if it's not owned, it can't be gifted unless we assume genesis allows it.
            // Let's keep original behavior: if it's there, check it.
        }

        // Replay protection: Check nonce
        let nonce_key = {
            let mut k = Vec::new();
            k.extend_from_slice(b"nonce:");
            k.extend_from_slice(&tx.art_hash);
            k
        };
        let current_nonce = if let Ok(Some(n_bytes)) = self.db.get(&nonce_key) {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(&n_bytes);
            u64::from_le_bytes(arr)
        } else {
            0
        };

        if tx.nonce <= current_nonce {
            return Err("Replay attack detected: Nonce is too low.");
        }

        // 2. Validate the ML-DSA-87 signature
        // The message is the concatenation of the art_hash, receiver's public key, and nonce
        // Also domain separation "COYWIN_GIFT"
        let mut message = b"COYWIN_GIFT".to_vec();
        message.extend_from_slice(&tx.art_hash);
        message.extend_from_slice(&tx.receiver);
        message.extend_from_slice(&tx.nonce.to_le_bytes());

        let pk = <PublicKey as PublicKeyTrait>::from_bytes(&tx.current_owner).map_err(|_| "Invalid Public Key")?;
        let sig = <DetachedSignature as DetachedSignatureTrait>::from_bytes(&tx.signature).map_err(|_| "Invalid Signature bytes")?;

        if dilithium_verify(&sig, &message, &pk).is_err() {
            return Err("ML-DSA-87 Quantum Signature Verification Failed.");
        }

        Ok(())
    }

    /// Validates and applies a Gift Transaction
    pub fn apply_gift(&self, tx: &GiftTransaction) -> Result<(), &'static str> {
        self.validate_gift(tx)?;

        // 3. Update the Ledger
        let nonce_key = {
            let mut k = Vec::new();
            k.extend_from_slice(b"nonce:");
            k.extend_from_slice(&tx.art_hash);
            k
        };
        
        self.db.insert(&nonce_key, &tx.nonce.to_le_bytes()).map_err(|_| "DB Write Error")?;
        self.db.insert(&tx.art_hash, tx.receiver.clone()).map_err(|_| "DB Write Error")?;
        self.db.flush().map_err(|_| "DB Flush Error")?;

        Ok(())
    }

    pub fn get_owner(&self, art_hash: &[u8; 32]) -> Option<Vec<u8>> {
        self.db.get(art_hash).ok().flatten().map(|v| v.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pqcrypto_traits::sign::{PublicKey as PublicKeyTrait, SecretKey as SecretKeyTrait, DetachedSignature as DetachedSignatureTrait};
    use pqcrypto_dilithium::dilithium5::keypair;
    use tempfile::tempdir;

    fn generate_valid_tx(art_hash: [u8; 32], nonce: u64, current_owner_sk: &pqcrypto_dilithium::dilithium5::SecretKey, current_owner_pk: &pqcrypto_dilithium::dilithium5::PublicKey, receiver_pk: &pqcrypto_dilithium::dilithium5::PublicKey) -> GiftTransaction {
        let mut message = b"COYWIN_GIFT".to_vec();
        message.extend_from_slice(&art_hash);
        message.extend_from_slice(receiver_pk.as_bytes());
        message.extend_from_slice(&nonce.to_le_bytes());

        let sig = pqcrypto_dilithium::dilithium5::detached_sign(&message, current_owner_sk);

        GiftTransaction {
            art_hash,
            current_owner: current_owner_pk.as_bytes().to_vec(),
            receiver: receiver_pk.as_bytes().to_vec(),
            nonce,
            signature: sig.as_bytes().to_vec(),
        }
    }

    #[test]
    fn test_regression_c04_mempool_mutation() {
        // C-04: Validate does not mutate
        let dir = tempdir().unwrap();
        let ledger = CoywinLedger::new(dir.path().to_str().unwrap()).unwrap();

        let (pk1, sk1) = keypair();
        let (pk2, _sk2) = keypair();
        let art_hash = [1u8; 32];
        
        let tx = generate_valid_tx(art_hash, 1, &sk1, &pk1, &pk2);
        
        assert!(ledger.validate_gift(&tx).is_ok());
        
        // Assert state is NOT mutated
        let owner = ledger.get_owner(&art_hash);
        assert!(owner.is_none());
    }

    #[test]
    fn test_regression_c07_replay_attack() {
        // C-07: Replay protection
        let dir = tempdir().unwrap();
        let ledger = CoywinLedger::new(dir.path().to_str().unwrap()).unwrap();

        let (pk1, sk1) = keypair();
        let (pk2, _sk2) = keypair();
        let art_hash = [2u8; 32];
        
        let tx = generate_valid_tx(art_hash, 1, &sk1, &pk1, &pk2);
        
        // Apply first time
        assert!(ledger.apply_gift(&tx).is_ok());
        
        // Apply exactly the same tx (Replay)
        assert!(ledger.apply_gift(&tx).is_err(), "Replay attack should be blocked!");
        
        // Apply with lower nonce (Replay)
        let tx_low = generate_valid_tx(art_hash, 0, &sk1, &pk1, &pk2);
        assert!(ledger.apply_gift(&tx_low).is_err(), "Replay attack with lower nonce should be blocked!");
    }

    #[test]
    fn test_regression_c06_invalid_signature() {
        let dir = tempdir().unwrap();
        let ledger = CoywinLedger::new(dir.path().to_str().unwrap()).unwrap();

        let (pk1, sk1) = keypair();
        let (pk2, _sk2) = keypair();
        let art_hash = [3u8; 32];
        
        let mut tx = generate_valid_tx(art_hash, 1, &sk1, &pk1, &pk2);
        
        // Mutate signature
        tx.signature[10] ^= 0xFF;
        
        assert!(ledger.validate_gift(&tx).is_err(), "Invalid signature must fail validation");
    }

    #[test]
    fn test_malformed_inputs() {
        let dir = tempdir().unwrap();
        let ledger = CoywinLedger::new(dir.path().to_str().unwrap()).unwrap();

        let (pk1, sk1) = keypair();
        let (pk2, _sk2) = keypair();
        let art_hash = [4u8; 32];
        
        // 1. Truncated signature
        let mut tx_trunc = generate_valid_tx(art_hash, 1, &sk1, &pk1, &pk2);
        tx_trunc.signature.truncate(100);
        assert!(ledger.validate_gift(&tx_trunc).is_err(), "Truncated signature must fail");

        // 2. Truncated public key
        let mut tx_pk = generate_valid_tx(art_hash, 2, &sk1, &pk1, &pk2);
        tx_pk.current_owner.truncate(100);
        assert!(ledger.validate_gift(&tx_pk).is_err(), "Truncated public key must fail");

        // 3. Oversized signature
        let mut tx_over = generate_valid_tx(art_hash, 3, &sk1, &pk1, &pk2);
        tx_over.signature.extend_from_slice(&[0u8; 1000]);
        assert!(ledger.validate_gift(&tx_over).is_err(), "Oversized signature must fail");
    }
}
