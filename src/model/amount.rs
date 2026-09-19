use std::{
    fmt,
    ops::{Add, AddAssign},
};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// New Type wrapper around a monetary amount (up to 4 decimal places).
// NOTE: The currently underlying `Decimal` allows for values approximately between -7e24 and 7e24.
// Should this not be sufficient, we can consider using a different numeric type (e.g. u128/u256).
// or a custom implementation to handle larger ranges.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Amount(Decimal);

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for Amount {
    fn default() -> Self {
        Self(Decimal::ZERO)
    }
}

impl Amount {
    pub const ZERO: Self = Self(Decimal::ZERO);
}

// Mainly for convenience when dealing with integer literals.
impl From<u32> for Amount {
    fn from(value: u32) -> Self {
        Self(Decimal::from(value))
    }
}

impl TryFrom<Decimal> for Amount {
    type Error = AmountError;

    fn try_from(value: Decimal) -> Result<Self, Self::Error> {
        if value < Decimal::ZERO {
            return Err(AmountError::Negative);
        }
        Ok(Self(value.round_dp(4)))
    }
}

impl Add for Amount {
    type Output = Amount;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for Amount {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Amount {
    /// Returns the underlying decimal value, e.g. for persistence.
    pub fn as_decimal(&self) -> Decimal {
        self.0
    }

    /// Safely subtracts the given amount from this amount, returning an error if the result would be negative.
    pub fn checked_sub(self, rhs: Self) -> Result<Self, AmountError> {
        if self.0 >= rhs.0 {
            Ok(Self(self.0 - rhs.0))
        } else {
            Err(AmountError::Negative)
        }
    }
}

#[derive(Debug, Error)]
pub enum AmountError {
    #[error("Amount cannot be negative")]
    Negative,
}
