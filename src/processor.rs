use crate::{
    model::{Account, AccountActionError, DepositRecord, IncomingTransaction}, store::{AccountStore, TransactionStore},
};
use log::{error, info};

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
        macro_rules! assert_result {
            ($obj:ident . $method:ident ( $($args:tt)* )) => {
                match $obj.$method($($args)*) {
                    Err(AccountActionError::InsufficientAvailableFunds) => {
                        info!("Ignoring {} transaction due to account state error: {:?}", stringify!($method), AccountActionError::InsufficientAvailableFunds);
                        return Ok(());
                    }
                    Err(AccountActionError::UnsupportedBalanceAmount) => {
                        error!("Account balance higher than supported");
                        return Err("Account balance higher than supported".into());
                    }
                    _ => {}
                }
            };
        }

        match transaction {
            IncomingTransaction::Deposit { client, tx, amount } => {
                let account = self.account_store.get_mut(client);
                assert_result!(account.deposit(amount));

                let transaction = DepositRecord::new(client, tx, amount);
                self.transaction_store.upsert(transaction)?;
            }
            IncomingTransaction::Withdrawal { client, amount, .. } => {
                let account = self.account_store.get_mut(client);
                assert_result!(account.withdraw(amount));
                // Since only deposits can be disputed, we don't need to record this withdrawal in the transaction store
            }
            IncomingTransaction::Dispute { client, tx } => {
                if let Some(mut transaction) = self.transaction_store.get(tx).unwrap()
                    && !transaction.is_disputed()
                    && transaction.belongs_to(client)
                {
                    let account = self.account_store.get_mut(client);
                    assert_result!(account.hold(transaction.amount));

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
                    assert_result!(account.release(transaction.amount));

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
                    assert_result!(account.chargeback(transaction.amount));

                    transaction.clear_disputed();
                    self.transaction_store.upsert(transaction)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::{ClientId, TransactionId},
        store::{InMemoryAccountStore, InMemoryTransactionStore},
    };

    type TestProcessor = TransactionProcessor<InMemoryAccountStore, InMemoryTransactionStore>;

    fn processor() -> TestProcessor {
        TransactionProcessor::new(InMemoryAccountStore::new(), InMemoryTransactionStore::new())
    }

    fn assert_account_balances(processor: &TestProcessor, client: u16, available: u32, held: u32) {
        let account = processor
            .accounts()
            .find(|a| a.client_id() == ClientId::new(client))
            .unwrap();
        assert_eq!(account.available(), available.into());
        assert_eq!(account.held(), held.into());
    }

    fn stored_record(processor: &TestProcessor, tx: u32) -> DepositRecord {
        processor
            .transaction_store
            .get(TransactionId::new(tx))
            .unwrap()
            .unwrap()
    }

    #[test]
    fn deposit_adds_funds_and_records_transaction() {
        let mut processor = processor();

        processor
            .process_transaction(IncomingTransaction::deposit(
                1u16.into(),
                7u32.into(),
                100u32.into(),
            ))
            .unwrap();

        assert_account_balances(&processor, 1, 100, 0);

        let record = stored_record(&processor, 7);
        assert_eq!(record.client, ClientId::new(1));
        assert_eq!(record.amount, 100u32.into());
        assert!(!record.is_disputed());
    }

    #[test]
    fn transactions_for_multiple_accounts_are_processed_independently() {
        let mut processor = processor();

        processor
            .process_transaction(IncomingTransaction::deposit(
                1u16.into(),
                7u32.into(),
                100u32.into(),
            ))
            .unwrap();
        processor
            .process_transaction(IncomingTransaction::deposit(
                2u16.into(),
                8u32.into(),
                200u32.into(),
            ))
            .unwrap();

        processor
            .process_transaction(IncomingTransaction::withdrawal(
                1u16.into(),
                9u32.into(),
                40u32.into(),
            ))
            .unwrap();
        processor
            .process_transaction(IncomingTransaction::dispute(2u16.into(), 8u32.into()))
            .unwrap();

        assert_account_balances(&processor, 1, 60, 0);
        assert_account_balances(&processor, 2, 0, 200);
    }

    #[test]
    fn withdrawal_reduces_available_funds_and_ignores_insufficient_funds() {
        let mut processor = processor();
        processor
            .process_transaction(IncomingTransaction::deposit(
                1u16.into(),
                7u32.into(),
                100u32.into(),
            ))
            .unwrap();

        processor
            .process_transaction(IncomingTransaction::withdrawal(
                1u16.into(),
                8u32.into(),
                40u32.into(),
            ))
            .unwrap();
        processor
            .process_transaction(IncomingTransaction::withdrawal(
                1u16.into(),
                9u32.into(),
                100u32.into(),
            ))
            .unwrap();

        assert_account_balances(&processor, 1, 60, 0);
    }

    #[test]
    fn dispute_holds_funds_and_resolve_releases_them() {
        let mut processor = processor();
        processor
            .process_transaction(IncomingTransaction::deposit(
                1u16.into(),
                7u32.into(),
                100u32.into(),
            ))
            .unwrap();

        processor
            .process_transaction(IncomingTransaction::dispute(1u16.into(), 7u32.into()))
            .unwrap();
        assert_account_balances(&processor, 1, 0, 100);
        assert!(stored_record(&processor, 7).is_disputed());

        processor
            .process_transaction(IncomingTransaction::resolve(1u16.into(), 7u32.into()))
            .unwrap();
        assert!(!stored_record(&processor, 7).is_disputed());

        processor
            .process_transaction(IncomingTransaction::withdrawal(
                1u16.into(),
                8u32.into(),
                40u32.into(),
            ))
            .unwrap();
        assert_account_balances(&processor, 1, 60, 0);
    }

    #[test]
    fn chargeback_removes_held_funds_and_locks_account() {
        let mut processor = processor();
        processor
            .process_transaction(IncomingTransaction::deposit(
                1u16.into(),
                7u32.into(),
                100u32.into(),
            ))
            .unwrap();
        processor
            .process_transaction(IncomingTransaction::dispute(1u16.into(), 7u32.into()))
            .unwrap();

        processor
            .process_transaction(IncomingTransaction::chargeback(1u16.into(), 7u32.into()))
            .unwrap();
        assert_account_balances(&processor, 1, 0, 0);
        assert!(
            !processor
                .transaction_store
                .get(TransactionId::new(7))
                .unwrap()
                .unwrap()
                .is_disputed()
        );

        processor
            .process_transaction(IncomingTransaction::deposit(
                1u16.into(),
                8u32.into(),
                25u32.into(),
            ))
            .unwrap();
        assert_account_balances(&processor, 1, 0, 0);
    }

    #[test]
    fn transaction_references_for_wrong_client_are_ignored() {
        let mut processor = processor();
        processor
            .process_transaction(IncomingTransaction::deposit(
                1u16.into(),
                7u32.into(),
                100u32.into(),
            ))
            .unwrap();
        processor
            .process_transaction(IncomingTransaction::dispute(1u16.into(), 7u32.into()))
            .unwrap();
        processor
            .process_transaction(IncomingTransaction::dispute(2u16.into(), 7u32.into()))
            .unwrap();
        processor
            .process_transaction(IncomingTransaction::resolve(2u16.into(), 7u32.into()))
            .unwrap();
        processor
            .process_transaction(IncomingTransaction::chargeback(2u16.into(), 7u32.into()))
            .unwrap();

        assert_account_balances(&processor, 1, 0, 100);
        assert!(stored_record(&processor, 7).is_disputed());
    }

    #[test]
    fn transaction_references_for_unknown_transaction_are_ignored() {
        let mut processor = processor();
        processor
            .process_transaction(IncomingTransaction::deposit(
                1u16.into(),
                7u32.into(),
                100u32.into(),
            ))
            .unwrap();
        processor
            .process_transaction(IncomingTransaction::dispute(1u16.into(), 7u32.into()))
            .unwrap();

        processor
            .process_transaction(IncomingTransaction::dispute(1u16.into(), 99u32.into()))
            .unwrap();
        processor
            .process_transaction(IncomingTransaction::resolve(1u16.into(), 99u32.into()))
            .unwrap();
        processor
            .process_transaction(IncomingTransaction::chargeback(1u16.into(), 99u32.into()))
            .unwrap();

        assert_account_balances(&processor, 1, 0, 100);
        assert!(stored_record(&processor, 7).is_disputed());
    }
}
