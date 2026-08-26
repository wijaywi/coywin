use coywin_ledger::{GiftTransaction, CoywinLedger};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Mempool {
    pending_gifts: Arc<RwLock<Vec<GiftTransaction>>>,
    ledger: Arc<CoywinLedger>,
}

impl Mempool {
    pub fn new(ledger: Arc<CoywinLedger>) -> Self {
        Self {
            pending_gifts: Arc::new(RwLock::new(Vec::new())),
            ledger,
        }
    }

    /// Adds a transaction to the mempool if it is cryptographically valid
    pub async fn add_transaction(&self, tx: GiftTransaction) -> Result<(), &'static str> {
        // We simulate a dry-run against the ledger state to ensure the signature is valid
        if let Err(e) = self.ledger.apply_gift(&tx) {
            return Err(e);
        }

        let mut pool = self.pending_gifts.write().await;
        pool.push(tx);
        Ok(())
    }

    /// Flushes the mempool into the permanent ledger (Mocking block finalization)
    pub async fn finalize_block(&self) {
        let mut pool = self.pending_gifts.write().await;
        for tx in pool.drain(..) {
            // Apply permanently
            let _ = self.ledger.apply_gift(&tx);
        }
    }
}
