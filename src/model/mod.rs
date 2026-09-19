mod account;
mod amount;
mod deposit_record;
mod incoming_transaction;

pub use account::Account;
pub use amount::Amount;
pub use deposit_record::DepositRecord;
pub use incoming_transaction::IncomingTransaction;

use serde::{Deserialize, Serialize};

/// New Type wrapper around a client identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct ClientId(u16);

impl From<u16> for ClientId {
    fn from(id: u16) -> Self {
        ClientId::new(id)
    }
}

impl ClientId {
    pub fn new(id: u16) -> Self {
        Self(id)
    }

    pub fn as_u16(&self) -> u16 {
        self.0
    }
}

/// New Type wrapper around a transaction identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct TransactionId(u32);

impl From<u32> for TransactionId {
    fn from(id: u32) -> Self {
        TransactionId::new(id)
    }
}

impl TransactionId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}
