use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Account {
    client_id: super::ClientId,
    total: super::Balance,
    held: super::Amount,
    locked: bool,
}

impl Account {
    pub fn new(client_id: super::ClientId) -> Self {
        Self {
            client_id,
            total: super::Balance::ZERO,
            held: super::Amount::ZERO,
            locked: false,
        }
    }

    // Getters

    pub fn client_id(&self) -> super::ClientId {
        self.client_id
    }

    pub fn available(&self) -> Result<super::Balance, AccountActionError> {
        self.total
            .checked_sub(self.held.as_decimal())
            .map_err(|_| AccountActionError::UnsupportedBalanceAmount)
    }

    pub fn held(&self) -> super::Amount {
        self.held
    }

    pub fn total(&self) -> super::Balance {
        self.total
    }

    // Balance manipulation methods

    pub fn deposit(&mut self, amount: super::Amount) -> Result<(), AccountActionError> {
        self.ensure_not_locked()?;
        self.total = self
            .total
            .checked_add(amount.as_decimal())
            .map_err(|_| AccountActionError::UnsupportedBalanceAmount)?;
        Ok(())
    }

    pub fn withdraw(&mut self, amount: super::Amount) -> Result<(), AccountActionError> {
        self.ensure_not_locked()?;

        if self.available()?.as_decimal() < amount.as_decimal() {
            return Err(AccountActionError::InsufficientAvailableFunds);
        }

        self.total = self
            .total
            .checked_sub(amount.as_decimal())
            .map_err(|_| AccountActionError::UnsupportedBalanceAmount)?;
        Ok(())
    }

    pub fn dispute(&mut self, amount: super::Amount) -> Result<(), AccountActionError> {
        self.ensure_not_locked()?;
        self.held = self
            .held
            .checked_add(amount)
            .map_err(|_| AccountActionError::UnsupportedBalanceAmount)?;
        Ok(())
    }

    pub fn resolve(&mut self, amount: super::Amount) -> Result<(), AccountActionError> {
        self.ensure_not_locked()?;
        self.held = self
            .held
            .checked_sub(amount)
            .map_err(|_| AccountActionError::InsufficientHeldFunds)?;
        Ok(())
    }

    pub fn chargeback(&mut self, amount: super::Amount) -> Result<(), AccountActionError> {
        self.ensure_not_locked()?;

        self.held = self
            .held
            .checked_sub(amount)
            .map_err(|_| AccountActionError::InsufficientHeldFunds)?;
        self.total = self
            .total
            .checked_sub(amount.as_decimal())
            .map_err(|_| AccountActionError::UnsupportedBalanceAmount)?;
        self.locked = true;
        Ok(())
    }

    fn ensure_not_locked(&self) -> Result<(), AccountActionError> {
        if self.locked {
            Err(AccountActionError::AccountLocked)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Error)]
pub enum AccountActionError {
    #[error("Insufficient available funds")]
    InsufficientAvailableFunds,
    #[error("Insufficient held funds")]
    InsufficientHeldFunds,
    #[error("Account is locked")]
    AccountLocked,
    #[error("Unsupported balance amount")]
    UnsupportedBalanceAmount,
}

// Manual impl so the CSV column order/names (client, available, held, total, locked)
// don't have to match the struct's field names, and `total` can be derived.
impl serde::Serialize for Account {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("Account", 5)?;
        state.serialize_field("client", &self.client_id)?;
        state.serialize_field("available", &self.available().unwrap())?;
        state.serialize_field("held", &self.held)?;
        state.serialize_field("total", &self.total)?;
        state.serialize_field("locked", &self.locked)?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Amount, Balance, ClientId};

    #[test]
    fn new_account_has_zero_balances_and_is_not_locked() {
        let account = super::Account::new(ClientId::new(1));
        assert_eq!(account.available().unwrap(), Balance::ZERO);
        assert_eq!(account.held, Amount::ZERO);
        assert_eq!(account.total, Balance::ZERO);
        assert!(!account.locked);
    }

    #[test]
    fn available_is_total_minus_held() {
        let mut account = super::Account::new(ClientId::new(1));
        account.total = Balance::from(5i32);
        account.held = Amount::from(3u32);
        assert_eq!(account.available().unwrap(), Balance::from(2i32));
    }

    #[test]
    fn deposit_increases_available_balance() {
        let mut account = super::Account::new(ClientId::new(1));
        account.total = Balance::from(1i32);
        let amount = 1u32.into();
        account.deposit(amount).unwrap();
        assert_eq!(account.available().unwrap(), Balance::from(2i32));
    }

    #[test]
    fn withdraw_decreases_available_balance() {
        let mut account = super::Account::new(ClientId::new(1));
        account.total = Balance::from(2i32);
        let amount = 1u32.into();
        account.withdraw(amount).unwrap();
        assert_eq!(account.available().unwrap(), Balance::from(1i32));
    }

    #[test]
    fn withdraw_fails_when_insufficient_funds() {
        let mut account = super::Account::new(ClientId::new(1));
        account.total = Balance::from(1i32);
        let amount = 2u32.into();
        assert!(matches!(
            account.withdraw(amount),
            Err(AccountActionError::InsufficientAvailableFunds)
        ));
    }

    #[test]
    fn dispute_moves_funds_from_available_to_held() {
        let mut account = super::Account::new(ClientId::new(1));
        account.total = Balance::from(2i32);
        let amount = 1u32.into();
        account.dispute(amount).unwrap();
        assert_eq!(account.available().unwrap(), Balance::from(1i32));
        assert_eq!(account.held, 1u32.into());
    }

    #[test]
    fn resolve_moves_funds_from_held_to_available() {
        let mut account = super::Account::new(ClientId::new(1));
        account.total = Balance::from(3i32);
        account.held = 2u32.into();
        let amount = 1u32.into();
        account.resolve(amount).unwrap();
        assert_eq!(account.available().unwrap(), Balance::from(2i32));
        assert_eq!(account.held, 1u32.into());
    }

    #[test]
    fn resolve_fails_when_insufficient_held_funds() {
        let mut account = super::Account::new(ClientId::new(1));
        account.held = 1u32.into();
        let amount = 2u32.into();
        assert!(matches!(
            account.resolve(amount),
            Err(AccountActionError::InsufficientHeldFunds)
        ));
    }

    #[test]
    fn chargeback_moves_funds_from_held_and_locks_account() {
        let mut account = super::Account::new(ClientId::new(1));
        account.held = 2u32.into();
        let amount = 1u32.into();
        account.chargeback(amount).unwrap();
        assert_eq!(account.held, 1u32.into());
        assert_eq!(account.total(), Balance::from(-1i32));
        assert!(account.locked);
    }

    #[test]
    fn chargeback_fails_when_insufficient_held_funds() {
        let mut account = super::Account::new(ClientId::new(1));
        account.held = 1u32.into();
        let amount = 2u32.into();
        assert!(matches!(
            account.chargeback(amount),
            Err(AccountActionError::InsufficientHeldFunds)
        ));
    }

    #[test]
    fn no_operation_can_be_performed_on_locked_account() {
        let mut account = super::Account::new(ClientId::new(1));
        account.locked = true;

        assert!(matches!(
            account.deposit(Amount::ZERO),
            Err(AccountActionError::AccountLocked)
        ));
        assert!(matches!(
            account.withdraw(Amount::ZERO),
            Err(AccountActionError::AccountLocked)
        ));
        assert!(matches!(
            account.dispute(Amount::ZERO),
            Err(AccountActionError::AccountLocked)
        ));
        assert!(matches!(
            account.resolve(Amount::ZERO),
            Err(AccountActionError::AccountLocked)
        ));
        assert!(matches!(
            account.chargeback(Amount::ZERO),
            Err(AccountActionError::AccountLocked)
        ));
    }
}
