use std::collections::HashMap;

use crate::model::{DepositRecord, TransactionId};

pub trait TransactionStore {
    fn upsert(&mut self, record: DepositRecord) -> Result<(), Box<dyn std::error::Error>>;

    fn get(&self, tx: TransactionId) -> Result<Option<&DepositRecord>, Box<dyn std::error::Error>>;
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
    fn upsert(&mut self, record: DepositRecord) -> Result<(), Box<dyn std::error::Error>> {
        self.records.insert(record.tx, record);
        Ok(())
    }

    fn get(&self, tx: TransactionId) -> Result<Option<&DepositRecord>, Box<dyn std::error::Error>> {
        Ok(self.records.get(&tx))
    }
}
