use std::collections::HashMap;

use crate::model::{DepositRecord, TxId};

pub trait TransactionStore {
    fn upsert(&mut self, record: DepositRecord);

    fn get(&self, tx: TxId) -> Option<&DepositRecord>;
}

pub struct InMemoryTransactionStore {
    records: HashMap<TxId, DepositRecord>,
}

impl InMemoryTransactionStore {
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }
}

impl TransactionStore for InMemoryTransactionStore {
    fn upsert(&mut self, record: DepositRecord) {
        self.records.insert(record.tx, record);
    }

    fn get(&self, tx: TxId) -> Option<&DepositRecord> {
        self.records.get(&tx)
    }
}
