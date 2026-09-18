use crate::model::{Amount, ClientId, TransactionId};

#[derive(Debug, Clone)]
pub struct DepositRecord {
    pub client: ClientId,
    pub tx: TransactionId,
    pub amount: Amount,
    pub disputed: bool,
}

impl DepositRecord {
    pub fn new(client: ClientId, tx: TransactionId, amount: Amount) -> Self {
        Self {
            client,
            tx,
            amount,
            disputed: false,
        }
    }

    pub fn is_disputed(&self) -> bool {
        self.disputed
    }

    pub fn mark_disputed(&mut self) {
        self.disputed = true;
    }

    pub fn clear_disputed(&mut self) {
        self.disputed = false;
    }

    pub fn belongs_to(&self, client: ClientId) -> bool {
        self.client == client
    }
}
