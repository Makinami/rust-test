use std::collections::HashMap;

use crate::model::{Account, ClientId};

pub trait AccountStore {
    fn get_mut(&mut self, client_id: ClientId) -> &mut Account;

    // TODO: Triple check, but add comment about `impl Trait` vs `Box<dyn Iterator>` for the `iter` method.
    fn iter(&self) -> impl Iterator<Item = &Account> + '_;
}

pub struct InMemoryAccountStore {
    accounts: HashMap<ClientId, Account>,
}

impl InMemoryAccountStore {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
        }
    }
}

impl AccountStore for InMemoryAccountStore {
    fn get_mut(&mut self, client_id: ClientId) -> &mut Account {
        self.accounts
            .entry(client_id)
            .or_insert_with(|| Account::new(client_id))
    }

    fn iter(&self) -> impl Iterator<Item = &Account> + '_ {
        self.accounts.values()
    }
}
