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
pub struct ClientId(pub u16);

/// New Type wrapper around a transaction identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct TxId(pub u32);
