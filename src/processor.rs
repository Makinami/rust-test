use crate::{
    model::{Account, DepositRecord, IncomingTransaction},
    store::{AccountStore, TransactionStore},
};
use log::info;

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

    pub fn accounts(&self) -> impl Iterator<Item = &Account> + '_ {
        self.account_store.iter()
    }
}

impl<A: AccountStore, D: TransactionStore> TransactionProcessor<A, D> {
    pub fn process_transaction(
        &mut self,
        transaction: IncomingTransaction,
    ) -> Result<(), Box<dyn std::error::Error>> {
        macro_rules! skip_on_error {
            ($obj:ident . $method:ident ( $($args:tt)* )) => {
                if let Err(err) = $obj.$method($($args)*) {
                    info!("Ignoring {} transaction due to account state error: {:?}", stringify!($method), err);
                    return Ok(());
                }
            };
        }

        match transaction {
            IncomingTransaction::Deposit { client, tx, amount } => {
                let account = self.account_store.get_mut(client);
                skip_on_error!(account.deposit(amount));

                let transaction = DepositRecord::new(client, tx, amount);
                self.transaction_store.upsert(transaction)?;
            }
            IncomingTransaction::Withdrawal { client, amount, .. } => {
                let account = self.account_store.get_mut(client);
                skip_on_error!(account.withdraw(amount));
                // Since only deposits can be disputed, we don't need to record this withdrawal in the transaction store
            }
            IncomingTransaction::Dispute { client, tx } => {
                if let Some(mut transaction) = self.transaction_store.get(tx).unwrap()
                    && !transaction.is_disputed()
                    && transaction.belongs_to(client)
                {
                    let account = self.account_store.get_mut(client);
                    skip_on_error!(account.hold(transaction.amount));

                    transaction.mark_disputed();
                    self.transaction_store.upsert(transaction)?;
                }
            }
            IncomingTransaction::Resolve { client, tx } => {
                if let Some(mut transaction) = self.transaction_store.get(tx).unwrap()
                    && transaction.is_disputed()
                    && transaction.belongs_to(client)
                {
                    let account = self.account_store.get_mut(client);
                    skip_on_error!(account.release(transaction.amount));

                    transaction.clear_disputed();
                    self.transaction_store.upsert(transaction)?;
                }
            }
            IncomingTransaction::Chargeback { client, tx } => {
                if let Some(mut transaction) = self.transaction_store.get(tx).unwrap()
                    && transaction.is_disputed()
                    && transaction.belongs_to(client)
                {
                    let account = self.account_store.get_mut(client);
                    skip_on_error!(account.chargeback(transaction.amount));

                    transaction.clear_disputed();
                    self.transaction_store.upsert(transaction)?;
                }
            }
        }
        Ok(())
    }
}
