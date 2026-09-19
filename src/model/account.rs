use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Account {
    client_id: super::ClientId,
    available: super::Amount,
    held: super::Amount,
    locked: bool,
}

impl Account {
    pub fn new(client_id: super::ClientId) -> Self {
        Self {
            client_id,
            available: super::Amount::default(),
            held: super::Amount::default(),
            locked: false,
        }
    }

    // Getters

    pub fn total(&self) -> super::Amount {
        self.available + self.held
    }

    // Balance manipulation methods

    pub fn deposit(&mut self, amount: super::Amount) -> Result<(), AccountActionError> {
        self.ensure_not_locked()?;
        self.available += amount;
        Ok(())
    }

    pub fn withdraw(&mut self, amount: super::Amount) -> Result<(), AccountActionError> {
        self.ensure_not_locked()?;
        self.available = self
            .available
            .checked_sub(amount)
            .map_err(|_| AccountActionError::InsufficientAvailableFunds)?;
        Ok(())
    }

    pub fn hold(&mut self, amount: super::Amount) -> Result<(), AccountActionError> {
        self.ensure_not_locked()?;
        self.available = self
            .available
            .checked_sub(amount)
            .map_err(|_| AccountActionError::InsufficientAvailableFunds)?;
        self.held += amount;
        Ok(())
    }

    pub fn release(&mut self, amount: super::Amount) -> Result<(), AccountActionError> {
        self.ensure_not_locked()?;
        self.held = self
            .held
            .checked_sub(amount)
            .map_err(|_| AccountActionError::InsufficientHeldFunds)?;
        self.available += amount;
        Ok(())
    }

    pub fn chargeback(&mut self, amount: super::Amount) -> Result<(), AccountActionError> {
        self.ensure_not_locked()?;
        self.held = self
            .held
            .checked_sub(amount)
            .map_err(|_| AccountActionError::InsufficientHeldFunds)?;
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
        state.serialize_field("available", &self.available)?;
        state.serialize_field("held", &self.held)?;
        state.serialize_field("total", &self.total())?;
        state.serialize_field("locked", &self.locked)?;
        state.end()
    }
}
