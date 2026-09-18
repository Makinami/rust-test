use std::collections::HashMap;

use crate::model::{DepositRecord, TransactionId};

pub trait TransactionStore {
    fn upsert(&mut self, record: DepositRecord);

    fn get(&self, tx: TransactionId) -> Option<&DepositRecord>;
}

pub struct InMemoryTransactionStore {
    records: HashMap<TransactionId, DepositRecord>,
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

    fn get(&self, tx: TransactionId) -> Option<&DepositRecord> {
        self.records.get(&tx)
    }
}
