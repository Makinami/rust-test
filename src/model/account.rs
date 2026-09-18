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

    pub fn client_id(&self) -> super::ClientId {
        self.client_id
    }

    pub fn available(&self) -> super::Amount {
        self.available
    }

    pub fn held(&self) -> super::Amount {
        self.held
    }

    pub fn total(&self) -> super::Amount {
        self.available + self.held
    }

    pub fn locked(&self) -> bool {
        self.locked
    }

    // Balance manipulation methods

    pub fn deposit(&mut self, amount: super::Amount) {
        self.available = self.available + amount;
    }

    pub fn withdrawal(&mut self, amount: super::Amount) -> Result<(), &'static str> {
        if self.available >= amount {
            self.available = self.available - amount;
            Ok(())
        } else {
            Err("Insufficient available funds")
        }
    }

    pub fn hold(&mut self, amount: super::Amount) -> Result<(), &'static str> {
        if self.available >= amount {
            self.available = self.available - amount;
            self.held = self.held + amount;
            Ok(())
        } else {
            Err("Insufficient available funds to hold")
        }
    }

    pub fn release(&mut self, amount: super::Amount) -> Result<(), &'static str> {
        if self.held >= amount {
            self.held = self.held - amount;
            self.available = self.available + amount;
            Ok(())
        } else {
            Err("Insufficient held funds to release")
        }
    }

    pub fn chargeback(&mut self, amount: super::Amount) -> Result<(), &'static str> {
        if self.held >= amount {
            self.held = self.held - amount;
            self.locked = true;
            Ok(())
        } else {
            Err("Insufficient held funds for chargeback")
        }
    }
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