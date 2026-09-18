use crate::{
    model::{DepositRecord, IncomingTransaction},
    store::{AccountStore, TransactionStore},
};

pub struct TransactionProcessor<A: AccountStore, D: TransactionStore> {
    account_store: A,
    transaction_store: D,
}

impl<A: AccountStore, D: TransactionStore> TransactionProcessor<A, D> {
    pub fn new(account_store: A, transaction_store: D) -> Self {
        Self {
            account_store,
            transaction_store,
        }
    }

    pub fn accounts(&self) -> impl Iterator<Item = &crate::model::Account> + '_ {
        self.account_store.iter()
    }
}

impl<A: AccountStore, D: TransactionStore> TransactionProcessor<A, D> {
    pub fn process_transaction(&mut self, transaction: crate::model::IncomingTransaction) {
        match transaction {
            IncomingTransaction::Deposit { client, tx, amount } => {
                let account = self.account_store.get_mut(client);
                account.deposit(amount);
                let deposit_record = DepositRecord::new(client, tx, amount);
                self.transaction_store.upsert(deposit_record);
            }
            IncomingTransaction::Withdrawal { client, amount, .. } => {
                let account = self.account_store.get_mut(client);
                let _ = account.withdraw(amount);
            }
            IncomingTransaction::Dispute { client, tx } => {
                if let Some(mut transaction) = self.transaction_store.get(tx).cloned()
                    && !transaction.is_disputed()
                    && transaction.client == client
                {
                    let account = self.account_store.get_mut(client);
                    account.hold(transaction.amount);
                    transaction.mark_disputed();
                    self.transaction_store.upsert(transaction);
                };
            }
            IncomingTransaction::Resolve { client, tx } => {
                // Handle resolve
                if let Some(mut transaction) = self.transaction_store.get(tx).cloned()
                    && transaction.is_disputed()
                    && transaction.client == client
                {
                    let account = self.account_store.get_mut(client);
                    account.release(transaction.amount);
                    transaction.clear_disputed();
                    self.transaction_store.upsert(transaction);
                };
            }
            IncomingTransaction::Chargeback { client, tx } => {
                // Handle chargeback
                if let Some(mut transaction) = self.transaction_store.get(tx).cloned()
                    && transaction.is_disputed()
                    && transaction.client == client
                {
                    let account = self.account_store.get_mut(client);
                    account.chargeback(transaction.amount);
                    transaction.clear_disputed();
                    self.transaction_store.upsert(transaction);
                };
            }
        }
    }
}